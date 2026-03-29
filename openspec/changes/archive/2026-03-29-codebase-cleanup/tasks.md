## 1. Rename `grid.rs` → `game_of_life.rs`

- [x] 1.1 Rename `src/grid.rs` to `src/game_of_life.rs`. Update `mod grid` → `mod game_of_life` in `main.rs` and `lib.rs`. _(Test: `cargo check`)_
- [x] 1.2 Remove hardcoded-dimension function variants (`index()`, `count_alive_neighbors()`, `step()`) and constants (`GRID_WIDTH`, `GRID_HEIGHT`, `GRID_SIZE`). Rename `index_wh` → `index`, `count_alive_neighbors_wh` → `count_alive_neighbors`, `step_wh` → `step`, `spawn_glider_wh` → `spawn_glider`. Update all call sites. _(Test: `cargo check --tests`)_
- [ ] 1.3 Move `GoLState` struct and its `impl` from `main.rs` into `gol_renderer.rs` (deferred to task 3.1 — avoids circular dependency). _(Test: `cargo check`)_

## 2. Split `metal_renderer.rs` → three modules

- [x] 2.1 Create `src/metal_context.rs` with `MetalContext`, `Uniforms`, `allocate_uniform_buffer()`, and `setup_metal_layer()` extracted from `metal_renderer.rs`. Update `lib.rs` with `pub mod metal_context`. _(Test: `cargo check`)_
- [x] 2.2 Create `src/gol_renderer.rs` with `GolRenderer` (renamed from `MetalRenderer`) and private `compile_shader_library()`. It takes a `&MetalContext` reference. Remove its `setup_metal_layer()` wrapper. Update `lib.rs` with `pub mod gol_renderer`. _(Test: `cargo check`)_
- [x] 2.3 Create `src/physarum_renderer.rs` with `PhysarumRenderer` and private `compile_physarum_library()`. It takes a `&MetalContext` reference. Remove its `setup_metal_layer()` wrapper. Update `lib.rs` with `pub mod physarum_renderer`. _(Test: `cargo check`)_
- [x] 2.4 Delete `src/metal_renderer.rs` and remove `mod metal_renderer` from `main.rs` / `lib.rs`. _(Test: `cargo check`)_

## 3. Extract event loops from `main.rs`

- [x] 3.1 Add `pub fn run(config: AppConfig, window: Window, event_loop: EventLoop<()>)` to `gol_renderer.rs`, moving the body of `run_gol()` from `main.rs`. Import `TICK_RATES`, `SIGTERM_RECEIVED`, `AppConfig` from crate root. _(Test: `cargo check`)_
- [x] 3.2 Add `pub fn run(config: AppConfig, window: Window, event_loop: EventLoop<()>)` to `physarum_renderer.rs`, moving the body of `run_physarum()` from `main.rs`. Import shared constants from crate root. _(Test: `cargo check`)_
- [x] 3.3 Slim `main.rs` to: `parse_args()`, `AppConfig`, `SimMode`, `TICK_RATES`, `SIGTERM_RECEIVED`, `sigterm_handler`, `main()` (window creation + mode dispatch). Remove `run_gol()`, `run_physarum()`, `GoLState`, and per-mode sync helpers. Export `TICK_RATES` and `SIGTERM_RECEIVED` as `pub`. _(Test: `cargo check`)_

## 4. Fix module visibility

- [x] 4.1 Add `pub mod wallpaper` to `lib.rs`. _(Test: `cargo check`)_
- [x] 4.2 Verify shader compilation helpers are `fn` (not `pub fn`) in `gol_renderer.rs` and `physarum_renderer.rs`. _(Test: code review)_

## 5. Update imports in test file

- [x] 5.1 Update `tests/gpu_integration.rs` to use new module paths: `game_of_life::`, `metal_context::MetalContext`, `gol_renderer::GolRenderer`, `physarum_renderer::PhysarumRenderer`. _(Test: `cargo test`)_

## 6. Update documentation

- [x] 6.1 Update `ARCHITECTURE.md` with new file layout and module responsibilities. _(Test: review)_
- [x] 6.2 Update `RULES.md` if it references old file/module names. _(Test: review)_

## 7. Final validation

- [x] 7.1 Run `cargo clippy --tests` — zero warnings. _(Test: lint)_
- [x] 7.2 Run `cargo test` — all existing tests pass. (Pre-existing physarum agent_step float precision failure; not introduced by this change.) _(Test: green suite)_
- [x] 7.3 Run `cargo build --release` and manual smoke test both modes. _(Test: smoke)_
