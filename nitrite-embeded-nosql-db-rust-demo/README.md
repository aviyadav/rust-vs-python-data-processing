# Nitrite Embedded NoSQL Database Rust Demo

A small Rust project demonstrating several ways to work with the embedded
[Nitrite](https://crates.io/crates/nitrite) NoSQL database:

- Flexible, JSON-like documents using `doc!` and collection filters.
- Type-safe Rust entities using `ObjectRepository` and `nitrite_derive`.
- Timed bulk insertion and lookups against randomly generated users.

The document and type-safe examples use an in-memory database. The volume test
uses a temporary disk-backed database so large imports do not need to fit in
RAM.

## Prerequisites

- Rust 1.85 or newer (the project uses the Rust 2024 edition)
- Cargo, installed with Rust

Install Rust through [rustup](https://rustup.rs/) if it is not already
available, then confirm the installation:

```sh
rustc --version
cargo --version
```

## Setup

From the project directory, download dependencies and compile all binaries:

```sh
cargo fetch
cargo build --all-targets
```

The main dependencies are:

- `nitrite` for the embedded database, collections, repositories, and filters.
- `nitrite_derive` for deriving type-safe entity conversions.
- `nitrite_fjall_adapter` for low-memory, disk-backed volume testing.
- `rand` for generating random user names and email addresses.

## Run the document API example

Run the default binary:

```sh
cargo run
```

This example:

1. Opens an in-memory Nitrite database.
2. Creates a `users` collection.
3. Inserts two JSON-like documents.
4. Finds users whose role is `engineer` and whose `active` field is `true`.
5. Prints the matching user's name and age.

Expected output:

```text
--- Searching for active engineers ---
Found: Jane Doe (Age: 28)
```

When using Nitrite's `doc!` macro, arrays use JSON-like square brackets:

```rust
nitrite::doc! {
    "skills": ["rust", "systems", "database"]
}
```

A nested `vec![]` invocation is not accepted by this macro.

## Run the type-safe repository example

Run the additional binary by name:

```sh
cargo run --bin main-with-type-safety
```

This example:

1. Defines a `User` entity with `Convertible` and `NitriteEntity` derives.
2. Opens an in-memory database and obtains an `ObjectRepository<User>`.
3. Inserts a `User` value directly.
4. Retrieves the user by ID and prints it.

Expected output:

```text
Retrieved user: Alice Jones (alice@example.com)
```

## Run the volume test

Run the volume-testing binary in release mode:

```sh
cargo run --release --bin volume-testing
```

The default run generates 1,000,000 users in batches of 100,000. Override the
record count and batch size with arguments after `--`:

```sh
cargo run --release --bin volume-testing -- 10000000 100000
```

To request one billion records:

```sh
cargo run --release --bin volume-testing -- 1000000000 100000
```

The example keeps only one batch in memory and writes records to
`target/volume-testing-db` using Fjall's low-memory preset. It creates indexes
for name and email, selects a generated user as the lookup target, and measures
generation, insertion, ID lookup, name lookup, and email lookup. The temporary
database is deleted after a successful run.

Batching prevents a single multi-gigabyte `Vec` allocation, but one billion
persisted and indexed records can still require hundreds of gigabytes of free
disk space and may take many hours or days. Start with a smaller count to
measure storage and throughput on the target machine before attempting the full
run. If the process is interrupted, its database remains under `target` and is
removed automatically when the next volume test starts.

## Project structure

```text
.
├── Cargo.toml
├── Cargo.lock
├── README.md
└── src
    ├── main.rs
    └── bin
        ├── main-with-type-safety.rs
        └── volume-testing.rs
```

- `src/main.rs` contains the flexible document and filter example.
- `src/bin/main-with-type-safety.rs` contains the typed entity repository
  example.
- `src/bin/volume-testing.rs` contains the timed bulk insertion and lookup
  example.

## Development checks

Format the code and verify every target compiles:

```sh
cargo fmt -- --check
cargo check --all-targets
```

To apply standard formatting, run:

```sh
cargo fmt
```
