## Why

The codebase has accumulated naming and structural debt across two phases of development. `grid.rs` contains Game of Life-specific rules despite its generic name. `metal_renderer.rs` bundles two independent renderers (600+ lines) with duplicated boilerplate. Module visibility is inconsistent (`wallpaper` missing from `lib.rs`). These issues make it harder to add new simulations and obscure the clean separation of concerns that was a founding principle.

## What Changes

- **Rename** `grid.rs` → `game_of_life.rs` to match its actual content (GoL rules, seeds, neighbor counting)
- **Split** `metal_renderer.rs` into three focused modules: `metal_context.rs` (shared Metal boilerplate), `gol_renderer.rs` (GoL pipelines + rendering), `physarum_renderer.rs` (Physarum pipelines + rendering)
- **Remove** duplicated `setup_metal_layer()` wrappers from both renderers — callers use `MetalContext` directly
- **Remove** hardcoded-dimension functions (`index()`, `count_alive_neighbors()`, `step()`) that silently depend on `GRID_WIDTH`/`GRID_HEIGHT` constants, keeping only the explicit `_wh` variants
- **Extract** event loops from `main.rs` — move `run_gol()` into `gol_renderer.rs` and `run_physarum()` into `physarum_renderer.rs`, leaving `main.rs` as a thin CLI + dispatch layer. Move `GoLState` into `game_of_life.rs` where it belongs.
- **Fix** module visibility: export `wallpaper` from `lib.rs`, make internal helpers (`compile_shader_library`, `compile_physarum_library`) private
- **Update** `ARCHITECTURE.md`, `RULES.md`, and `README.md` to reflect new file layout

## UMA Relevance

No change to UMA mechanics — this is a pure structural cleanup. The `MetalContext` extraction makes the shared-memory pattern more visible: one `MetalContext` owns the device and queue, both renderers borrow it for zero-copy buffer access.

## Capabilities

### New Capabilities

_(none — this change restructures existing code without adding new behavior)_

### Modified Capabilities

- `grid-rendering`: Fragment shader requirement scenarios reference "grid buffer" — update to reflect per-mode buffer naming (GoL grid buffer vs Physarum trail buffer)
- `metal-init`: File references change (`metal_renderer.rs` → `metal_context.rs` + per-mode renderers); shared-mode buffer allocation moves to `MetalContext`
- `simulation-mode`: Event loop ownership moves from `main.rs` into each renderer module; `main.rs` retains only CLI parsing and dispatch

## Impact

- All `use` / `mod` paths in `main.rs`, `lib.rs`, and `tests/gpu_integration.rs` change
- `grid::` prefix throughout codebase becomes `game_of_life::`
- `metal_renderer::MetalRenderer` → `gol_renderer::GolRenderer`
- `metal_renderer::PhysarumRenderer` → `physarum_renderer::PhysarumRenderer`
- `metal_renderer::MetalContext` → `metal_context::MetalContext`
- `run_gol()` moves from `main.rs` → `gol_renderer::run()`; `run_physarum()` moves from `main.rs` → `physarum_renderer::run()`
- `GoLState` moves from `main.rs` → `game_of_life.rs`
- `main.rs` shrinks from ~325 lines to ~80 lines (CLI + dispatch only)
- No behavioral changes — all existing tests must pass with only import path updates
