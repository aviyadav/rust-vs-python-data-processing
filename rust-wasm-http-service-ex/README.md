# my-wasm-http-service

A server-side HTTP service written in Rust, compiled to a WebAssembly component
that targets the `wasi:http/proxy` world. Built with
[`cargo-component`](https://github.com/bytecodealliance/cargo-component) and
served by [Wasmtime](https://wasmtime.dev/).

This project follows the Medium article *"Rust & Wasm: Building Lightning-Fast
HTTP Services with cargo-component"* (PDF included in this repo), **updated with
the fixes needed to make it work with current toolchains**. See
[Changes from the original article](#changes-from-the-original-article) for the
details.

## Routes

| Endpoint              | Response                                             |
| --------------------- | ---------------------------------------------------- |
| `/` (anything else)   | `Welcome to your Rust Wasm HTTP Service!`            |
| `/hello`              | `Hello from WebAssembly with Rust!`                  |
| `/greet?name=<name>`  | `Greetings, <name> from Wasm!` (or `...stranger!`)   |
| `/info`               | Dumps request method, path, and `User-Agent` header  |

## Prerequisites

1. **Rust toolchain** (via [rustup](https://rustup.rs/)):

   ```sh
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **cargo-component** (tested with v0.21.1):

   ```sh
   cargo install cargo-component
   ```

3. **Wasmtime** (tested with v48.0.1; any version with Component Model +
   `wasi:http/proxy` support works):

   ```sh
   curl https://wasmtime.dev/install.sh -sSf | bash
   ```

4. **Wasm target** — cargo-component builds for `wasm32-wasip1` and usually
   installs it automatically. If not:

   ```sh
   rustup target add wasm32-wasip1
   ```

## 1. Create the project

```sh
cargo component new --lib my-wasm-http-service
cd my-wasm-http-service
```

The `--lib` flag creates a *reactor* component (a long-lived library-style
component, right for HTTP services). If the template generates a
`wit/world.wit` with an example world, it can be deleted — the `proxy = true`
setting below targets the `wasi:http/proxy` world without any custom WIT.

## 2. Configure `Cargo.toml`

```toml
[package]
name = "my-wasm-http-service"
version = "0.1.0"
edition = "2021"

[dependencies]
# 0.14.7+ re-exports the `wasip2` crate: the handler trait lives at
# `wasi::exports::http::incoming_handler::Guest` and the `export!` macro
# bundles its own wit-bindgen runtime, so no extra deps are needed.
wasi = "0.14.7"
url = "2.5"

[lib]
crate-type = ["cdylib"]

[package.metadata.component]
package = "component:my-wasm-http-service"
# Targets the `wasi:http/proxy` world: this component implements the
# WASI HTTP incoming-handler interface. No custom WIT files needed.
proxy = true
```

## 3. Implement the service

`src/lib.rs` implements the incoming-handler trait and uses the `export!` macro
to wire it up:

```rust
wasi::http::proxy::export!(Component);

struct Component;

impl wasi::exports::http::incoming_handler::Guest for Component {
    fn handle(request: IncomingRequest, response_out: ResponseOutparam) {
        // ... routing, headers, body ...
    }
}
```

See [`src/lib.rs`](src/lib.rs) for the full implementation, including routing,
query-parameter parsing, and header reading.

## 4. Build

```sh
cargo component build --release
```

The component lands at:

```
target/wasm32-wasip1/release/my_wasm_http_service.wasm
```

## 5. Run

`wasmtime serve` needs the **path to the built `.wasm` file**:

```sh
wasmtime serve --addr 127.0.0.1:8080 target/wasm32-wasip1/release/my_wasm_http_service.wasm
```

You should see: `Serving HTTP on http://127.0.0.1:8080/`

## 6. Test

From another terminal:

```sh
# Default route
curl http://127.0.0.1:8080/
# → Welcome to your Rust Wasm HTTP Service!

# Hello route
curl http://127.0.0.1:8080/hello
# → Hello from WebAssembly with Rust!

# Greet route with a query parameter
curl "http://127.0.0.1:8080/greet?name=Explorer"
# → Greetings, Explorer from Wasm!

# Info route (echoes request details)
curl http://127.0.0.1:8080/info
# → Wasm HTTP Service Info:
#   Method: Method::Get
#   Path: Some("/info")
#   User-Agent: curl/8.x.y
```

Requests are also logged to stderr by the service (`eprintln!`), which Wasmtime
prints as `stderr [n] :: Incoming request: ...`.

## Changes from the original article

The article's code was written against an older `wasi` crate API. This repo
contains the corrected version:

1. **`wasi` 0.14.2 → 0.14.7.** Cargo resolves `wasi = "0.14.2"` to
   `0.14.7+wasi-0.2.4`, which re-exports the `wasip2` crate. The handler trait
   moved: `wasi::http::proxy::IncomingHandler` no longer exists — implement
   **`wasi::exports::http::incoming_handler::Guest`** instead. (The
   `wasi::http::proxy::export!` macro stayed put.)
2. **`wit-bindgen-rt` dependency removed.** The `export!` macro is
   self-contained; no separate runtime crate is needed.
3. **Response construction.** `OutgoingResponse::new(headers)` now takes the
   headers and defaults to status 200. There is no `StatusCode::OK`
   (`StatusCode` is just `u16`; use `set_status_code()` for other codes) and no
   `set_headers()`.
4. **Headers API.** `Fields::set(name, values)` takes a slice of byte vectors
   (e.g. `&[b"text/plain".to_vec()]`); `Fields::get(name)` returns
   `Vec<Vec<u8>>`, not `Option`.
5. **Body writing.** `response.write()` + `std::io::Write` is replaced by:
   `response.body()` → `body.write()` → `stream.blocking_write_and_flush(...)`
   → `drop(stream)` → `OutgoingBody::finish(body, None)` (the stream must be
   dropped before `finish`, or it traps).
6. **`Method` has no `Display` impl** — format it with `{:?}`, not
   `.to_string()`.
7. **Routing fix.** The article matched routes against `path_with_query`, so
   `/greet?name=...` never matched `"/greet"`. The path and query string are now
   split before routing.
8. **Artifact path.** The component is emitted at
   `target/wasm32-wasip1/release/...` (the article said `target/wasm32-wasi/`).
9. **`wasmtime serve` invocation.** Pass the full path to the built component
   (see step 5 above).
