# tesser-journal

`tesser-journal` provides the LMDB-backed persistence layer used by the Tesser runtime to store live portfolio snapshots and execution algorithm state. It exposes repositories that implement the same traits as the in-memory stores in `tesser-portfolio` and `tesser-execution`, so the CLI can switch to durable persistence without changing upstream code.

## Highlights

- **LMDB environment management** – `LmdbJournal::open` provisions a directory, configures map sizing (10 GiB by default), and initializes the internal databases for you.
- **Repositories for both domains** – `LmdbStateRepository` persists `LiveState` snapshots while `LmdbAlgoStateRepository` stores serialized execution plugin/algorithm state keyed by `Uuid`.
- **Resilient by construction** – Writes happen inside LMDB transactions, giving crash-safe commits. Snapshot reads fall back to defaults when no state has been stored yet.

## Quick start

```rust
use rust_decimal::Decimal;
use tesser_execution::StoredAlgoState;
use tesser_journal::LmdbJournal;
use uuid::Uuid;

let journal = LmdbJournal::open("./journal").expect("LMDB env");
let state_repo = journal.state_repo();
let algo_repo = journal.algo_repo();

// Store a portfolio snapshot
let mut state = state_repo.load().expect("initial snapshot");
state.portfolio_equity += Decimal::new(1_000, 0);
state_repo.save(&state).expect("durable write");

// Persist execution algorithm state
let algo_id = Uuid::new_v4();
let algo_state = StoredAlgoState {
    plugin: "twap".into(),
    payload: serde_json::json!({ "pending": 2 }),
};
algo_repo.save(&algo_id, &algo_state).expect("commit state");
```

## Operational notes

- **Directory layout** – The crate normalizes the provided path and will create it if it does not exist. If you point it at a file, a sibling directory with the `.lmdb` extension is created automatically.
- **Map sizing** – `MAP_SIZE_BYTES` defaults to 10 GiB to avoid live reallocation. Adjust the constant (or expose your own builder) if you need a different cap.
- **Backups & recovery** – LMDB’s copy-on-write storage means you can take consistent backups by snapshotting the directory with `mmap`-friendly tools (`rsync`, `cp --reflink`, EBS snapshots, etc.) while the runtime is paused.

## Testing

Integration tests can use `tempfile` to create isolated LMDB directories:

```rust
let dir = tempfile::tempdir().unwrap();
let journal = LmdbJournal::open(dir.path()).unwrap();
```

This mirrors how `tesser-cli` provisions the journal when `journal.type = "lmdb"` is set in the runtime configuration.
