## 1. Refactoring

- [x] 1.1 Move `encode_compute_pass`, `encode_render_pass`, and `render_frame` from free functions in `main.rs` into methods on `MetalRenderer` (e.g. `gol_step()`, `gol_render()`). Keep the same logic, just relocate. Test: existing 34 tests still pass; `cargo run` renders GoL identically.
- [x] 1.2 Extract GoL-specific event-loop logic (glider spawning, `seed_grid`, `next_spawn` timer) into a `GoLState` struct or helper, separate from generic window/input handling in the event loop. Test: existing 34 tests still pass.
- [x] 1.3 Move `setup_metal_layer` from `main.rs` into `metal_renderer.rs` as an associated function or into a shared `window_utils` module. Test: existing 34 tests still pass.
- [x] 1.4 Export `physarum` module from `src/lib.rs` (initially empty or with `PhysarumConfig` only). Test: `cargo build` succeeds.

## 2. CLI and Mode Selection

- [x] 2.1 Add `SimMode` enum (`GameOfLife`, `Physarum`) and `--mode` flag to `parse_args()` in `main.rs`; add `mode: SimMode` to `AppConfig`. Test: unit test that `--mode physarum` parses correctly, default is `gol`, `--mode unknown` returns error.
- [x] 2.2 Wire `SimMode` match in `main()` to branch buffer/pipeline creation (stubbed — GoL path unchanged, Physarum path panics with `todo!()`). Test: existing 34 tests still pass; `cargo run -- --mode physarum` panics with clear message.

## 3. CPU Reference Implementation

- [x] 2.1 Create `src/physarum.rs` with `PhysarumConfig` struct (sensor_angle, sensor_dist, turn_speed, move_speed, deposit_amount, decay_factor, width, height, num_species) using the same constant values as the shader will use.
- [x] 2.2 Implement `cpu_agent_step(agents: &mut [[f32; 4]], trail_src: &[f32], trail_dst: &mut [f32], config: &PhysarumConfig)` — sense/rotate/move/deposit with toroidal wrap. Test: pure-logic unit test with a single agent at known position, verify heading and position change.
- [x] 2.3 Implement `cpu_diffuse_decay(trail_src: &[f32], trail_dst: &mut [f32], config: &PhysarumConfig)` — 3×3 box blur + multiply by decay factor, toroidal boundary. Test: pure-logic unit test with a single non-zero cell, verify blur spreads to neighbours and value decays.
- [x] 2.4 Implement `init_agents(width: u32, height: u32, num_agents: usize, seed: u64) -> Vec<[f32; 4]>` — random positions, random headings, cycling species. Test: unit test checking agent count, species distribution (~1/3 each), positions within bounds.

## 3. Physarum Metal Shader

- [x] 3.1 Create `src/shaders/physarum.metal` with fixed parameter constants (`SENSOR_ANGLE`, `SENSOR_DIST`, `TURN_SPEED`, `MOVE_SPEED`, `DEPOSIT_AMOUNT`, `DECAY_FACTOR`) matching the CPU reference values. Add `agent_step` kernel stub (writes agents unchanged). Test: shader compiles without errors via `cargo build`.
- [x] 3.2 Implement full `agent_step` kernel — sense (sample 3 probes from species plane), rotate, move with toroidal wrap, deposit `DEPOSIT_AMOUNT` to trail_dst at nearest cell. Test: GPU integration test — seed 1 agent at known position, run 1 step, compare output with CPU reference (epsilon 1e-4).
- [x] 3.3 Implement `diffuse_decay` kernel — 3×3 box blur on each species plane with toroidal wrap, multiply by `DECAY_FACTOR`, write to destination buffer. Test: GPU integration test — seed trail map with single non-zero cell, run 1 step, compare with CPU reference (epsilon 1e-4).
- [x] 3.4 Add `fullscreen_quad_vertex_physarum` vertex function and `physarum_fragment` shader — read 3 species trail planes, multiply by palette (cyan: `0,0.8,0.9`; magenta: `0.9,0.1,0.6`; gold: `0.9,0.8,0.0`), additive blend, clamp to `[0,1]`, black background. Test: shader compiles; visual check deferred to smoke test.

## 4. MetalRenderer Physarum Pipeline

- [x] 4.1 Add Physarum buffer allocation to `MetalRenderer`: agent buffer (`float4 × num_agents`, shared), two trail map buffers (`float × W×H×3`, shared), uniforms for grid dimensions. Test: buffer test — allocate buffers, verify sizes and `StorageModeShared`.
- [x] 4.2 Create Physarum compute pipeline states: `agent_step_pipeline` and `diffuse_decay_pipeline` from `physarum.metal` library. Test: pipeline creation succeeds on Metal device.
- [x] 4.3 Create Physarum render pipeline state: vertex + fragment from `physarum.metal`. Test: pipeline creation succeeds on Metal device.
- [x] 4.4 Implement `physarum_step(&mut self)` method — encode `agent_step` compute pass, then `diffuse_decay` compute pass, then swap trail buffer index. Test: GPU integration test — run 1 step with known agents, verify trail buffer contents match CPU after both kernels.

## 5. Main Loop Integration

- [x] 5.1 Replace `todo!()` stub in `SimMode::Physarum` branch: initialise agents, allocate Physarum buffers/pipelines, wire `physarum_step()` + Physarum render pass into the `AboutToWait` event handler. Test: `cargo run -- --mode physarum` opens a window without panicking.
- [x] 5.2 Ensure `--wallpaper` flag works with `--mode physarum` — same `configure_wallpaper()` call, same window setup. Test: `cargo run -- --mode physarum --wallpaper` renders Physarum as desktop wallpaper.
- [x] 5.3 Ensure tick-rate input (arrow keys) applies to Physarum mode. Test: manual verification — arrow keys change Physarum simulation speed.

## 6. GPU Integration Tests

- [x] 6.1 Add `test_gpu_physarum_agent_step_matches_cpu` — seed 100 agents at random-but-deterministic positions, blank trail, run 1 GPU step, compare all agent positions and trail deposits with CPU reference (epsilon 1e-4).
- [x] 6.2 Add `test_gpu_physarum_diffuse_decay_matches_cpu` — seed trail map with known pattern, run 1 GPU diffuse_decay step, compare with CPU reference (epsilon 1e-4).
- [x] 6.3 Add `test_gpu_physarum_full_frame_matches_cpu` — seed agents + blank trail, run 1 full frame (agent_step + diffuse_decay), compare final trail map with CPU (epsilon 1e-4).

## 7. Smoke Test

- [x] 7.1 Build release binary and run `--mode physarum` — verify colored trails appear, 3 species visible, organic vein-like patterns form within 10 seconds. Manual verification.
- [x] 7.2 Run `--mode physarum --wallpaper` — verify wallpaper mode with Physarum renders correctly behind desktop icons. Manual verification.

## 8. Post-Implementation Refactoring

- [x] 8.1 Extract shared Metal boilerplate (device init, command queue, uniform buffer, layer setup) into a base `MetalContext` struct used by both GoL and Physarum renderers. Test: all existing + new tests pass; no duplicated device/queue setup.
- [x] 8.2 Consolidate the fullscreen quad vertex shader — if `fullscreen_quad_vertex` in `game_of_life.metal` and `physarum.metal` are identical, move to a shared `common.metal` included by both. Test: `cargo build` succeeds; both modes render correctly.
- [x] 8.3 Reduce `pub` field exposure on `MetalRenderer` — replace direct field access (`renderer.compute_pipeline`, `renderer.grid_buffers`) with methods. Update `main.rs` and tests to use the new API. Test: all tests pass; no `pub` fields remain except where required by test crate access.
- [x] 8.4 Update `ARCHITECTURE.md` to document the dual-mode architecture, new files (`physarum.rs`, `physarum.metal`), and the `SimMode` branching strategy. Test: review only — no code change.
- [x] 8.5 Clean up any `#[allow(...)]` suppressions or `todo!()` stubs left from incremental implementation. Test: `cargo clippy` reports no warnings.
