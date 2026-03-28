# Cellular Automata on Apple Silicon

A GPU-accelerated cellular automata playground exploring Apple Silicon's Unified Memory Architecture (UMA). Built with Rust and Metal — no buffer copies, no staging, CPU and GPU share the same physical memory.

Supports two simulation modes:

- **Game of Life** — Conway's GoL with fade-out dying cells, random glider spawning, and seed patterns
- **Physarum** — Multi-species slime mould simulation (Jones 2010) with organic, vein-like trail patterns

## Requirements

- macOS on Apple Silicon (M1/M2/M3/M4)
- Rust toolchain (2024 edition)
- Xcode Command Line Tools (for Metal shader compilation)

## Build

```sh
cargo build --release
```

## Usage

```sh
cargo run --release -- [OPTIONS]
```

### Options

| Flag | Values | Default | Description |
|------|--------|---------|-------------|
| `--mode` | `gol`, `physarum` | `gol` | Simulation mode |
| `--seed` | `blinker`, `glider`, `r-pentomino` | `r-pentomino` | Initial seed pattern (Game of Life only) |
| `--wallpaper` | — | off | Render as desktop wallpaper behind all windows |

### Examples

```sh
# Game of Life with default r-pentomino seed
cargo run --release

# Game of Life with glider seed
cargo run --release -- --seed glider

# Physarum slime mould
cargo run --release -- --mode physarum

# Physarum as desktop wallpaper
cargo run --release -- --mode physarum --wallpaper

# Game of Life as desktop wallpaper
cargo run --release -- --wallpaper --seed r-pentomino
```

## Keyboard Controls

| Key | Action |
|-----|--------|
| Arrow Up | Increase simulation speed |
| Arrow Down | Decrease simulation speed |
| Escape | Quit |

Speed cycles through: 1, 2, 5, 10, 20, 30, 60, 120 steps/sec.

Game of Life starts at 10 steps/sec. Physarum starts at 30 steps/sec.

## Wallpaper Mode

The `--wallpaper` flag renders the simulation as a live desktop wallpaper:

- Window is placed at the desktop level, behind all other windows
- Borderless, full-screen, spanning the main display
- Appears on all Spaces and is invisible to Cmd-Tab / Exposé
- Grid size automatically scales to match screen resolution
- Responds to `SIGTERM` for clean shutdown (useful with launchd)

To stop a wallpaper instance, send SIGTERM or press Escape (the window is behind other windows, so you may need to use "Show Desktop" to focus it).

## Project Structure

| File | Purpose |
|------|---------|
| `src/main.rs` | CLI, event loops for both modes |
| `src/grid.rs` | Game of Life logic and seed patterns |
| `src/physarum.rs` | Physarum CPU reference implementation |
| `src/metal_renderer.rs` | Metal device, pipelines, and renderers |
| `src/wallpaper.rs` | macOS desktop wallpaper integration |
| `src/shaders/game_of_life.metal` | GoL compute + render shaders |
| `src/shaders/physarum.metal` | Physarum compute + render shaders |

## Testing

```sh
cargo test
```

Tests include CPU logic unit tests and GPU integration tests that cross-validate Metal shader output against the Rust CPU reference implementations.
