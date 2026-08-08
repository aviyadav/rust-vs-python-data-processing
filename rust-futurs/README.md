# rust-futurs

A small, hands-on introduction to **futures and async/await in Rust**, built on the
[Tokio](https://tokio.rs) runtime. Each binary is a self-contained demo you can run
and read top to bottom.

## Prerequisites

- [Rust](https://rustup.rs/) (stable toolchain)
- Internet access for the first build (Cargo downloads `tokio` from crates.io)

## Running the demos

### Main demo — futures in practice

```sh
cargo run --bin rust-futurs
```

Walks through five sections, printing output and timings as it goes:

1. **Futures are lazy** — calling an `async fn` only *builds* a future;
   its body doesn't run until you `.await` it.
2. **Sequential awaiting** — two 500 ms fetches take ~1 s, one after another.
3. **Concurrency with `join!`** — the same two fetches overlap and finish
   in ~0.5 s, because `.await` yields to the runtime instead of blocking.
4. **Spawning tasks** — `tokio::spawn` hands a future to the runtime as a
   background task; the returned `JoinHandle` is itself a future you `.await`
   later to collect the result.
5. **Racing with `select!`** — takes whichever future completes first.

### Under the hood — a hand-written Future

```sh
cargo run --bin manual_future
```

Implements the `Future` trait manually with a background thread and a shared
`Waker`, showing the `poll` / `Poll::Pending` / `wake()` contract that every
`.await` ultimately drives. Useful for understanding what async runtimes
actually do with your futures.

## Project layout

```
src/
├── main.rs              # Main demo: laziness, join!, spawn, select!
└── bin/
    └── manual_future.rs # Hand-rolled Future impl (poll/waker internals)
```

## Key concepts

| Concept | Where | What it shows |
|---|---|---|
| Laziness | `main.rs` §1 | An `async fn` returns an inert `Future`; nothing runs until `.await` |
| `.await` | everywhere | Drives the future, yielding the task at points where it can't progress |
| `tokio::join!` | `main.rs` §3 | Polls multiple futures concurrently on one task |
| `tokio::spawn` | `main.rs` §4 | Schedules a future as an independent task on the runtime |
| `tokio::select!` | `main.rs` §5 | Races futures, taking the first to complete |
| `Future::poll` | `manual_future.rs` | The trait method runtimes call to make progress |
| `Waker` | `manual_future.rs` | How a pending future asks to be polled again |

## Dependencies

- [`tokio`](https://crates.io/crates/tokio) `1.x` with the `macros`,
  `rt-multi-thread`, and `time` features.
