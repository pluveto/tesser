use std::sync::atomic::{AtomicU64, Ordering};

use crate::{LedgerRepository, LedgerResult};

/// Simple atomic sequencer used to assign monotonic ledger IDs.
#[derive(Debug)]
pub struct LedgerSequencer {
    counter: AtomicU64,
}

impl LedgerSequencer {
    /// Create a new sequencer that starts after the provided value.
    pub fn new(last_sequence: u64) -> Self {
        Self {
            counter: AtomicU64::new(last_sequence),
        }
    }

    /// Bootstrap the sequencer by reading the persisted tail sequence.
    pub fn bootstrap(repo: &dyn LedgerRepository) -> LedgerResult<Self> {
        let last = repo.latest_sequence()?.unwrap_or(0);
        Ok(Self::new(last))
    }

    /// Return the next monotonic sequence.
    pub fn next(&self) -> u64 {
        self.counter.fetch_add(1, Ordering::SeqCst) + 1
    }
}
