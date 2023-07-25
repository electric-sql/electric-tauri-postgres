# Tauri + Postgres
An experiment of trying to embed as much of postgres as possible and as less of postgres as necessary.

## Architecture
This app was created with the `create-tauri-app` util.

Tauri backend + Vite frontend app.

The main dependency is `pg_embed`, a package that wraps an embedded postgres in Rust bindings. It doesn't get it to being in-process embeddable as sqlite, but we are trying to get there.

The app currently starts in two windows, one being the "app", the other being a postgres debug console, connected to the same database.

## Commands
What you may want to do with this code:
- Bundle for ubuntu (currently the tested system):
```bash
pnpm run tauri build
```
- Run the app in development mode:
```bash
pnpm run tauri dev
```
- Run the selenium tests:
```bash
pnpm test
```
- TODO: Rust unit tests, TypeScript unit tests.
- Build the backend for release:
```bash
cd src-tauri/ && cargo build --release
```
- Build the backend for debugging:
```bash
cd src-tauri/ && cargo build
```

A Makefile is also provided, helping with many of these commands.
