## Context

The codebase grew across three phases (metal-window-bootstrap → gpu-game-of-life → physarum-slime-mould). Each phase was additive — new code landed in existing files rather than restructuring. The result: `grid.rs` contains GoL-specific rules despite its generic name, `metal_renderer.rs` bundles two independent renderers in 600+ lines, and module visibility is inconsistent.

Current file map:

```
src/
├── main.rs              ← CLI, both event loops, SimMode
├── grid.rs              ← GoL rules + seeds (misnamed)
├── physarum.rs           ← Physarum CPU reference
├── metal_renderer.rs     ← MetalContext + MetalRenderer + PhysarumRenderer (too big)
├── wallpaper.rs          ← macOS wallpaper mode (not exported from lib.rs)
├── lib.rs                ← pub mod grid, metal_renderer, physarum (incomplete)
└── shaders/
    ├── game_of_life.metal
    └── physarum.metal
```

Target file map:

```
src/
├── main.rs               ← CLI parsing, SimMode, dispatch only (~80 lines)
├── game_of_life.rs        ← GoL rules, seeds, GoLState (renamed from grid.rs)
├── physarum.rs            ← Physarum CPU reference (unchanged)
├── metal_context.rs       ← MetalContext (device, queue, layer setup, uniforms)
├── gol_renderer.rs        ← GolRenderer + run() event loop
├── physarum_renderer.rs   ← PhysarumRenderer + run() event loop
├── wallpaper.rs           ← macOS wallpaper mode (now exported from lib.rs)
├── lib.rs                 ← all pub mods
└── shaders/
    ├── game_of_life.metal
    └── physarum.metal
```

## Goals / Non-Goals

**Goals:**
- File names match their contents — no surprises
- Each module has a single responsibility
- Shared Metal boilerplate lives in one place, not three
- All public modules exported from `lib.rs`; internal helpers are private
- Hardcoded-dimension function variants removed
- Existing tests pass with only import path changes

**Non-Goals:**
- Trait-based renderer abstraction (premature — only two modes exist)
- Splitting `tests/gpu_integration.rs` into separate test files (cosmetic, not blocking)
- Changing any simulation logic, shader code, or buffer layouts
- Moving unit tests out of their source files (standard Rust convention)

## Decisions

### D1: Rename `grid.rs` → `game_of_life.rs`

**Rationale:** Every function in this file is GoL-specific (birth/death rules, alive-neighbor counting, seed patterns, glider spawning). The name `grid` implies a generic data structure; `game_of_life` accurately describes the module's scope and is symmetrical with `physarum.rs`.

**Alternative considered:** Split into `grid.rs` (indexing) + `game_of_life.rs` (rules). Rejected — the indexing functions (`index_wh`) are trivial one-liners not worth a separate module.

### D2: Split `metal_renderer.rs` → `metal_context.rs` + `gol_renderer.rs` + `physarum_renderer.rs`

**Rationale:** `MetalContext`, `GolRenderer`, and `PhysarumRenderer` have no shared mutable state after construction. They're already logically independent; the file is the only thing coupling them. Three focused files averaging ~200 lines each are easier to navigate than one 600-line file.

**Module layout:**

```
src/
├── metal_context.rs      ← MetalContext (device, queue), Uniforms, allocate_uniform_buffer()
├── gol_renderer.rs       ← GolRenderer (was MetalRenderer), compile_shader_library()
├── physarum_renderer.rs  ← PhysarumRenderer, compile_physarum_library()
```

### D3: Rename `MetalRenderer` → `GolRenderer`

**Rationale:** With two renderers in the codebase, the generic name `MetalRenderer` is ambiguous. `GolRenderer` is explicit and symmetrical with `PhysarumRenderer`.

### D4: Remove duplicated `setup_metal_layer()` wrappers

Both `MetalRenderer` and `PhysarumRenderer` have `setup_metal_layer()` methods that just delegate to `MetalContext::setup_metal_layer()`. Remove them — callers already have access to `MetalContext` (or the renderer's `.device()`) and should call `MetalContext::setup_metal_layer()` directly.

### D5: Remove hardcoded-dimension function variants

`grid.rs` has paired functions: `index()` / `index_wh()`, `count_alive_neighbors()` / `count_alive_neighbors_wh()`, `step()` / `step_wh()`. The non-`_wh` variants silently use `GRID_WIDTH` / `GRID_HEIGHT` constants. Remove them and rename the `_wh` variants to drop the suffix (they become the only version). Remove the `GRID_WIDTH`, `GRID_HEIGHT`, `GRID_SIZE` constants.

### D6: Make shader compilation functions private

`compile_shader_library()` and `compile_physarum_library()` are only called inside their respective renderer `new()` methods. They should be module-private (`fn`, not `pub fn`).

### D7: Export `wallpaper` from `lib.rs`

Currently `wallpaper` is declared as `mod wallpaper` in `main.rs` but not exported from `lib.rs`. This means integration tests can't access it and `cargo doc` doesn't document it. Add `pub mod wallpaper` to `lib.rs`.

### D8: Extract event loops from `main.rs` into renderer modules

**Rationale:** `main.rs` contains two 80-line event loop functions (`run_gol`, `run_physarum`) with nearly identical scaffolding (window events, keyboard input, tick rate, SIGTERM). This makes `main.rs` 325+ lines of mixed concerns: CLI parsing, application state, and rendering orchestration. Each event loop is tightly coupled to its renderer — `run_gol` only uses `GolRenderer`, `run_physarum` only uses `PhysarumRenderer`.

**Decision:** Move each event loop into its renderer module as a public `run()` function:
- `gol_renderer::run(config, window, event_loop)` — owns GoL initialization, tick loop, glider spawning
- `physarum_renderer::run(config, window, event_loop)` — owns Physarum initialization, tick loop

`main.rs` becomes thin: `parse_args()` → create window/event loop → match mode → call `gol_renderer::run()` or `physarum_renderer::run()`.

**`GoLState` moves to `game_of_life.rs`:** It contains GoL-specific logic (seeding, glider spawning). The renderer module uses it, but it belongs with the GoL domain logic.

**Shared utilities stay in `main.rs`:** `TICK_RATES`, `SIGTERM_RECEIVED`, and `sigterm_handler` are shared by both modes and used at the application level, so they remain in `main.rs` (exported as `pub`).

**Alternative considered:** A shared generic event loop with a trait callback. Rejected per non-goal — "Trait-based renderer abstraction (premature — only two modes exist)". The two event loops are similar but not identical (GoL has glider spawning, different default tick rate, different init). Forcing them into a trait adds complexity without clear benefit.

## Risks / Trade-offs

- **[Risk] Git blame disrupted for moved code** → Acceptable; `git log --follow` tracks renames. The clarity gain outweighs the history cost.
- **[Risk] External dependants break** → No external consumers exist; this is a standalone binary. The only consumer beyond `main.rs` is `tests/gpu_integration.rs`, which we update.
- **[Risk] Merge conflicts if other branches exist** → Only one branch (`main`). No risk.
- **[Trade-off] More files to navigate** → Three 200-line files vs one 600-line file. Net positive for discoverability.

## Open Questions

_(none — scope is well-defined and all decisions are internal refactoring)_
