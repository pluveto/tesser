use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use futures::{stream, Stream};
use tesser_core::{DepthUpdate, OrderBook, Symbol, Tick};

use crate::{
    analytics::collect_parquet_files,
    parquet::{DepthCursor, OrderBookCursor, TickCursor},
};

#[derive(Clone, Copy)]
enum Source {
    Tick,
    Book,
    Depth,
}

/// Unified event emitted by the merged parquet cursors.
#[derive(Debug)]
pub struct UnifiedEvent {
    pub timestamp: DateTime<Utc>,
    pub kind: UnifiedEventKind,
}

/// Enum describing the concrete payload contained within a [`UnifiedEvent`].
#[derive(Debug)]
pub enum UnifiedEventKind {
    OrderBook(OrderBook),
    Depth(DepthUpdate),
    Trade(Tick),
}

/// Builder that merges heterogeneous parquet cursors into a single chronological stream.
pub struct UnifiedEventStream {
    symbols: HashSet<Symbol>,
    ticks: Option<TickCursor>,
    tick_peek: Option<Tick>,
    books: Option<OrderBookCursor>,
    book_peek: Option<OrderBook>,
    depth: Option<DepthCursor>,
    depth_peek: Option<DepthUpdate>,
}

impl UnifiedEventStream {
    /// Construct a stream backed by parquet files located under a flight-recorder root.
    pub fn from_flight_recorder(root: impl AsRef<Path>, symbols: &[Symbol]) -> Result<Self> {
        let root = root.as_ref();
        let tick_paths = collect_parquet_files(&root.join("ticks"))?;
        let book_paths = collect_first_existing(root, &["order_books", "books"])?;
        let depth_paths = collect_first_existing(root, &["depth", "depth_updates"])?;
        Self::from_paths(symbols, tick_paths, book_paths, depth_paths)
    }

    /// Construct a stream from explicit parquet path lists.
    pub fn from_paths(
        symbols: &[Symbol],
        tick_paths: Vec<PathBuf>,
        order_book_paths: Vec<PathBuf>,
        depth_paths: Vec<PathBuf>,
    ) -> Result<Self> {
        if tick_paths.is_empty() && order_book_paths.is_empty() && depth_paths.is_empty() {
            return Err(anyhow!("at least one parquet data source must be provided"));
        }
        Ok(Self {
            symbols: symbols.iter().cloned().collect(),
            ticks: (!tick_paths.is_empty()).then(|| TickCursor::new(tick_paths)),
            tick_peek: None,
            books: (!order_book_paths.is_empty()).then(|| OrderBookCursor::new(order_book_paths)),
            book_peek: None,
            depth: (!depth_paths.is_empty()).then(|| DepthCursor::new(depth_paths)),
            depth_peek: None,
        })
    }

    /// Convert this stream into a [`futures::Stream`] implementation.
    pub fn into_stream(self) -> impl Stream<Item = Result<UnifiedEvent>> {
        stream::unfold(self, |mut state| async move {
            match state.next_event().await {
                Ok(Some(event)) => Some((Ok(event), state)),
                Ok(None) => None,
                Err(err) => Some((Err(err), state)),
            }
        })
    }

    async fn next_event(&mut self) -> Result<Option<UnifiedEvent>> {
        self.ensure_tick().await?;
        self.ensure_book().await?;
        self.ensure_depth().await?;

        let mut candidate: Option<(DateTime<Utc>, Source)> = None;

        if let Some(tick) = self.tick_peek.as_ref() {
            candidate = pick_candidate(candidate, tick.exchange_timestamp, Source::Tick);
        }
        if let Some(book) = self.book_peek.as_ref() {
            candidate = pick_candidate(candidate, book.timestamp, Source::Book);
        }
        if let Some(update) = self.depth_peek.as_ref() {
            candidate = pick_candidate(candidate, update.timestamp, Source::Depth);
        }

        let Some((_, source)) = candidate else {
            return Ok(None);
        };

        let event = match source {
            Source::Tick => {
                let tick = self
                    .tick_peek
                    .take()
                    .expect("tick candidate must be populated");
                UnifiedEvent {
                    timestamp: tick.exchange_timestamp,
                    kind: UnifiedEventKind::Trade(tick),
                }
            }
            Source::Book => {
                let book = self
                    .book_peek
                    .take()
                    .expect("order book candidate must be populated");
                UnifiedEvent {
                    timestamp: book.timestamp,
                    kind: UnifiedEventKind::OrderBook(book),
                }
            }
            Source::Depth => {
                let update = self
                    .depth_peek
                    .take()
                    .expect("depth candidate must be populated");
                UnifiedEvent {
                    timestamp: update.timestamp,
                    kind: UnifiedEventKind::Depth(update),
                }
            }
        };
        Ok(Some(event))
    }

    async fn ensure_tick(&mut self) -> Result<()> {
        if self.tick_peek.is_some() {
            return Ok(());
        }
        let filter = self.symbols.clone();
        let allow_all = filter.is_empty();
        let Some(cursor) = self.ticks.as_mut() else {
            return Ok(());
        };
        while let Some(tick) = cursor.next().await? {
            if allow_all || filter.contains(&tick.symbol) {
                self.tick_peek = Some(tick);
                break;
            }
        }
        Ok(())
    }

    async fn ensure_book(&mut self) -> Result<()> {
        if self.book_peek.is_some() {
            return Ok(());
        }
        let filter = self.symbols.clone();
        let allow_all = filter.is_empty();
        let Some(cursor) = self.books.as_mut() else {
            return Ok(());
        };
        while let Some(book) = cursor.next().await? {
            if allow_all || filter.contains(&book.symbol) {
                self.book_peek = Some(book);
                break;
            }
        }
        Ok(())
    }

    async fn ensure_depth(&mut self) -> Result<()> {
        if self.depth_peek.is_some() {
            return Ok(());
        }
        let filter = self.symbols.clone();
        let allow_all = filter.is_empty();
        let Some(cursor) = self.depth.as_mut() else {
            return Ok(());
        };
        while let Some(update) = cursor.next().await? {
            if allow_all || filter.contains(&update.symbol) {
                self.depth_peek = Some(update);
                break;
            }
        }
        Ok(())
    }
}

fn pick_candidate(
    current: Option<(DateTime<Utc>, Source)>,
    ts: DateTime<Utc>,
    source: Source,
) -> Option<(DateTime<Utc>, Source)> {
    match current {
        Some((existing_ts, existing_source)) => {
            if ts < existing_ts {
                Some((ts, source))
            } else {
                Some((existing_ts, existing_source))
            }
        }
        None => Some((ts, source)),
    }
}

fn collect_first_existing(root: &Path, names: &[&str]) -> Result<Vec<PathBuf>> {
    for name in names {
        let path = root.join(name);
        if path.exists() {
            return collect_parquet_files(&path);
        }
    }
    Ok(Vec::new())
}
