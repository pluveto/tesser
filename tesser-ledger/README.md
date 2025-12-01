# tesser-ledger

`tesser-ledger` contains the accounting primitives that keep Tesser's balances, realized PnL, and adjustments auditable. It defines the canonical `LedgerEntry` structure plus repositories that write to SQLite or Apache Parquet, so the same APIs cover low-latency disk storage and analytics exports.

## Components

- **LedgerEntry / LedgerType** – Strongly typed rows that satisfy the accounting identity and capture metadata (exchange, asset, reference id, optional JSON payloads).
- **LedgerRepository** – Trait for storage engines; the crate ships `SqliteLedgerRepository` for on-disk durability and `ParquetLedgerRepository` for analytical pipelines.
- **LedgerSequencer** – Monotonic sequence allocator ensuring deterministic replay.
- **Journal helpers** – `entries_from_fill` converts `tesser_core::Fill` events into the correct ledger lines for spot or perpetual instruments, including realized PnL and fees.

## Quick start

```rust
use rust_decimal::Decimal;
use tesser_core::{AssetId, ExchangeId};
use tesser_ledger::{LedgerEntry, LedgerRepository, LedgerType, LedgerSequencer, SqliteLedgerRepository};

let repo = SqliteLedgerRepository::new("ledger.db").expect("open sqlite");
let mut sequencer = LedgerSequencer::new(repo.latest_sequence()?.unwrap_or(0));

let entry = LedgerEntry::new(
    ExchangeId::from("paper"),
    AssetId::from("paper:USDT"),
    Decimal::new(1_000, 0),
    LedgerType::TransferIn,
    "initial_funding",
)
.with_sequence(sequencer.next());

repo.append(&entry).expect("persist ledger line");
```

To export the same data for downstream analytics, instantiate `ParquetLedgerRepository` and call `append_batch` with the same `LedgerEntry` values. The schema aligns with Arrow so you can load the files directly into Python/Polars.

## Querying

`LedgerQuery` lets you filter the repository by exchange, asset, ledger type, or sequence range. This makes it easy to:

- Reconstruct balance history for a specific settlement currency.
- Audit fees or funding across sessions.
- Generate statements for sub-accounts.

```rust
use tesser_core::AssetId;
use tesser_ledger::LedgerQuery;

let entries = repo.query(LedgerQuery::default().with_asset(AssetId::from("binance:USDT")))?;
```

## Integration notes

- Transactions are batched so parents can use `append_batch` to commit all adjustments derived from a fill atomically.
- `entries_from_fill` and `FillLedgerContext` ensure derivative vs. spot products produce the correct cash movements and metadata.
- The crate is `no_std`-friendly aside from the storage backends; you can depend on just the types if you disable the repositories or gate them behind features in your own crate.
