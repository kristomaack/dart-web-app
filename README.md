# Dart scorer

A mobile-first web app for scoring darts at the board, written in Rust.
It runs entirely in the browser (Leptos + WebAssembly) and is served by a tiny
Axum server in a single Docker container.

## Games

- **301 / 501**: optional double-in and double-out, bust handling, checkout hints.
- **Close Out**: close 15–20 and Bull by hitting each 1, 3, 5 or 7 times
  (single = 1, double = 2, triple = 3); first to close everything wins.

All games support up to 8 players, drag-to-reorder and shuffle, best-of-N legs,
"first out wins" or "play on for places", undo across turns, per-player stats
(3-dart average / marks per round) and rematch. Games in progress are saved in
the browser, so a refresh or a locked phone doesn't lose the game.

## Layout

```
crates/core    Pure game rules (no web deps). A match is its config plus an
               append-only list of darts; all state is derived by replaying it.
crates/web     Leptos client (CSR, built with Trunk).
crates/server  Axum static server with SPA fallback, /healthz and an empty /api.
```

New minigames: add a `ModeKind`/`ModeConfig`/`Board` variant in
`crates/core/src/modes/mod.rs` and a rules module next to it; the compiler
points out every other place that needs to handle it.

Storage sits behind the `Repo` trait in `crates/web/src/repo.rs`
(localStorage today), so a backend can be swapped in later and reuse
`dart-core` to validate the same dart log.

## Run

With Docker:

```sh
docker compose up -d --build   # http://localhost:8020
```

Locally (needs the `wasm32-unknown-unknown` target and [Trunk](https://trunkrs.dev)):

```sh
cd crates/web && trunk serve          # dev server on :8081
# or build and serve like production:
trunk build --release && cd ../.. && cargo run -p dart-server
```

The server reads `PORT` (default `8020`) and `DIST_DIR` (default `crates/web/dist`).
It serves plain HTTP; put it behind a reverse proxy for TLS.

## Test

```sh
cargo test -p dart-core -p dart-server
```

## License

MIT
