## 1. Add `rand` Dependency

- [x] 1.1 Add `rand` crate to `Cargo.toml` dependencies and verify `cargo build` succeeds

## 2. Glider Rotation Variants

- [x] 2.1 Add four glider rotation offset tables to `grid.rs` as constants (N/E/S/W orientations) and a `spawn_glider(grid, cx, cy, rotation)` function that calls `place()` with the selected rotation
- [x] 2.2 Add pure-logic unit tests in `tests/grid_tests.rs` verifying each of the 4 rotations produces a valid 5-cell pattern and that edge-wrapping works (e.g., spawn at (255, 255))

## 3. Spawn Timer and RNG Integration

- [x] 3.1 Add spawn timer state to the event loop in `main.rs`: `next_spawn: Instant` initialised to `now + rand(10..=30)` seconds, using `rand::thread_rng()` and `Uniform` distribution
- [x] 3.2 In `RedrawRequested`, before encoding the compute pass: check if `now >= next_spawn`, and if so call `spawn_glider()` on `renderer.grid_buffer_slice_mut(current_buffer)` with random position and rotation, then reset `next_spawn`

## 4. GPU Integration Test

- [x] 4.1 Add a GPU integration test in `tests/gpu_integration.rs` that writes a glider into a shared buffer via CPU, runs one compute step on GPU, reads back the result, and verifies it matches CPU `step()` output (validates frame-boundary write safety)

## 5. Smoke Test

- [ ] 5.1 Run the application (`cargo run --release`) and confirm gliders appear at random positions over a 60-second observation window, with varied orientations and no visual tearing or corruption (READY — release binary built)
