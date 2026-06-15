# z048-wasm

Browser (WebAssembly) bindings for [z048](https://github.com/z048/z048), a 2048 self-play engine.
All game and network logic is reused from z048's public API (`Board` / `Slide` / `Spawn` / `Dicer` /
`Rater`, pulled in as a git dependency); this crate only adds a thin [`wasm-bindgen`](https://github.com/rustwasm/wasm-bindgen)
wrapper — a single-board `Game` state machine you can drive from JavaScript. It does **not** fork or
re-implement any of z048's logic.

## Concept

A full turn has two phases on one shared board:

1. **slide** — the *order* side merges tiles (the maximizer).
2. **spawn** — the *chaos* side drops an adversarial tile on the afterstate (the minimizer).

`Game` holds a single `board` and a `phase` flag and steps through `slide → spawn → slide → …` until the
board is full and unmergeable (`end()`).

The AI methods **only suggest** a move; they do not change the board:

- `order(depth, tau)` returns the slide the net would play.
- `chaos(depth, tau)` returns the spawn the net would play.

You apply a move (whether it came from the AI or a human) with `slide(dir)` / `spawn(x, y, rank)`. This
suggest-then-apply split lets the same methods serve PvE, PvP, and EvE — the caller decides, per phase,
whether to call `order`/`chaos` (AI) or feed in a human move. A single `Rater` drives both sides, and a
process-wide entropy-seeded `Dicer` provides randomness (board init and sampling).

## Build

This crate builds **only** for `wasm32-unknown-unknown`: `.cargo/config.toml` makes it the default target
(so `cargo build` needs no `--target`), and a `compile_error!` in `src/lib.rs` rejects any other target.

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-pack            # if not already installed
wasm-pack build --target web --release
```

The output lands in `pkg/` (`wasm_bg.wasm` plus the ES-module glue `wasm.js`).

> **Toolchain note.** Building for wasm needs a toolchain whose sysroot has the wasm32 std component. If
> `cargo`/`rustc` resolve to one that doesn't (e.g. a Homebrew install shadowing rustup), you'll see
> *can't find crate for `std`*; make sure the wasm-capable toolchain is the one on `PATH`.
>
> **getrandom.** `rand 0.9` (used here and by candle) pulls in `getrandom 0.3`, which needs its `wasm_js`
> backend on wasm — already enabled in `Cargo.toml` (`getrandom = { version = "0.3", features = ["wasm_js"] }`).

## Model

`new Game(bytes)` takes a **postcard-serialized** z048 model — exactly the bytes `Rater::save` writes:
`postcard` of `Vec<(Vec<f32>, Vec<f32>)>`, one `(weights, bias)` pair per layer (weights flat, row-major
`in × out`). The dims chain from the 256-wide input (4×4×16 one-hot) to a 2-output head. Train and export a
network with z048, then load its bytes here. It is a binary format — not human-readable JSON.

## Usage from JavaScript

wasm must be served over HTTP (it cannot be loaded from `file://`):

```js
import init, { Game } from './pkg/wasm.js';

await init();
const bytes = new Uint8Array(await (await fetch('./model.bin')).arrayBuffer());
const game = new Game(bytes);

// One half-move: ask the AI, then apply its suggestion.
if (!game.phase()) {                 // slide turn
  const dir = game.order(2, 0.0);
  if (dir !== undefined) game.slide(dir);
} else {                             // spawn turn
  const mv = game.chaos(2, 0.0);
  if (mv) game.spawn(mv[0], mv[1], mv[2]);
}
```

For a human move, call `game.slide(dir)` / `game.spawn(x, y, rank)` directly instead of `order` / `chaos`.

## `Game` API (`#[wasm_bindgen]`)

Construction:

| Method | Description |
| --- | --- |
| `new Game(bytes: Uint8Array)` | Build the rater from postcard model bytes and seed a random board from the global dicer. Starts in the slide phase. |
| `reset()` | Start a fresh game: new random board, back to the slide phase (keeps the loaded model). |

Read-only:

| Method | Returns |
| --- | --- |
| `board()` | `Uint8Array` — 16 ranks, row-major `[x][y]` (x outer, y inner); `0` is empty (tile value = `2^rank`). |
| `phase()` | `boolean` — `false` = slide turn, `true` = spawn turn. |
| `end()` | `boolean` — game over (board full and unmergeable). |
| `score()` / `escore()` | `number` (`f64`). |

AI — **suggest only** (return the move, do not apply it):

| Method | Returns |
| --- | --- |
| `order(depth, tau)` | `number \| undefined` — slide direction `0..=3` (U/D/L/R), or `undefined` outside the slide turn / when over. |
| `chaos(depth, tau)` | `Uint8Array \| undefined` — `[x, y, rank]`, or `undefined` outside the spawn turn. |

Apply a move (no-op if it's the wrong phase or the move is illegal):

| Method | Description |
| --- | --- |
| `slide(dir)` | Apply a slide, `dir` in `0..=3`. Advances to the spawn phase. |
| `spawn(x, y, rank)` | Apply a spawn, `x`,`y` in `0..3`, `rank` in `{1, 2}`. Advances to the slide phase (or ends). |

`tau` defaults to `0.0` (greedy); `depth` is best kept in `1..=3` — deeper search is **much** slower
because each ply expands both the slide and spawn branches, so the branching factor compounds.
