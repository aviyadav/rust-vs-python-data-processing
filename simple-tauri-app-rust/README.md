# Tauri App

A minimal [Tauri 2](https://tauri.app) desktop application demonstrating basic
frontend-to-backend communication (IPC) with shared, mutable application
state protected by an async mutex.

### setting up tauri - Ubuntu

```sh
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev \
  build-essential \
  curl \
  wget \
  file \
  libxdo-dev \
  libssl-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev

```


### setup the project

```sh
cargo tauri init
```


## Overview

The app exposes two Rust commands to the frontend via Tauri's `invoke` API:

- `login` — accepts a username, password, and email, and stores them as the
  current user in shared app state.
- `get_user` — returns the currently stored user from shared app state.

The frontend is a single static HTML page with two buttons that call these
commands directly through `window.__TAURI__.core.invoke`.

## Project Structure

```
src-tauri/
├── src/
│   ├── main.rs        # Application entry point, calls app_lib::run()
│   └── lib.rs          # Tauri Builder setup, state management, and commands
├── ui/
│   └── index.html      # Frontend UI (static HTML/JS, no build step)
├── capabilities/
│   └── default.json    # Tauri capability/permissions configuration
├── icons/               # App icons used for bundling
├── build.rs             # Tauri build script
├── tauri.conf.json      # Tauri app configuration
└── Cargo.toml           # Rust package manifest
```

## Backend (Rust)

The backend is defined in [`src/lib.rs`](src/lib.rs).

### State

A single `User` struct is managed as shared app state, wrapped in a
`tauri::async_runtime::Mutex` (an async/Tokio mutex) so it can safely be
accessed from async commands:

```rust
struct User {
    id: u32,
    username: String,
    password: String,
    email: String,
}
```

The state is initialized with an empty/default `User` when the app starts.

### Commands

| Command    | Parameters                                             | Returns             | Description                                  |
|------------|---------------------------------------------------------|----------------------|-----------------------------------------------|
| `get_user` | –                                                       | `Result<User, ()>`  | Returns a clone of the currently stored user. |
| `login`    | `username: String`, `password: String`, `email: String` | `Result<bool, ()>`  | Overwrites the stored user and returns `true`. |

Both commands are `async` and take `tauri::State<'_, Mutex<User>>` as their
first parameter. Because they borrow state asynchronously, Tauri requires
them to return a `Result`.

## Frontend (HTML/JS)

The frontend lives in [`ui/index.html`](ui/index.html) and requires no build
step — it's served directly as static content (see `frontendDist` in
`tauri.conf.json`).

- **Login** button → calls `doLogin()`, which invokes the `login` command
  with a hardcoded username/password/email, then alerts on success.
- **Get User** button → calls `getUser()`, which invokes the `get_user`
  command and logs the result to the console.

`withGlobalTauri` is enabled in `tauri.conf.json`, which exposes the
`window.__TAURI__` API to the frontend without needing the `@tauri-apps/api`
npm package.

## Configuration

Key settings in [`tauri.conf.json`](tauri.conf.json):

- **Product name:** `Tauri App`
- **Identifier:** `com.tauri.dev`
- **Frontend dist:** `./ui` (static files, no dev server/bundler)
- **Window:** `800x600`, resizable, non-fullscreen
- **CSP:** disabled (`null`)

Permissions are defined in [`capabilities/default.json`](capabilities/default.json),
granting the `core:default` permission set to the `main` window.

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (edition 2021, minimum
  version 1.77.2)
- [Tauri CLI](https://tauri.app/start/prerequisites/) (`cargo install
  tauri-cli` or use `cargo tauri` via the `tauri-cli` dev dependency)
- Platform-specific Tauri prerequisites (WebView2 on Windows, WebKitGTK on
  Linux, Xcode command line tools on macOS)

## Development

Run the app in development mode:

```sh
cargo tauri dev
```

This launches the app with the static `ui/index.html` frontend and hot
rebuilds the Rust backend on changes.

## Building

Build a release bundle for your platform:

```sh
cargo tauri build
```

Bundled installers/targets are configured under `bundle` in
`tauri.conf.json` (`"targets": "all"`), and app icons are sourced from the
`icons/` directory.

## Dependencies

From [`Cargo.toml`](Cargo.toml):

- `tauri` `2.11.3` — core Tauri runtime
- `tauri-build` `2.6.3` — build-time code generation
- `tauri-plugin-log` `2` — logging plugin
- `serde` / `serde_json` — serialization for commands and state
- `log` — logging facade

## Notes

- The stored `password` is kept in plain text in memory for this demo — do
  not use this pattern in a real application without proper hashing and
  secure storage.
- State is in-memory only and resets every time the app restarts.
