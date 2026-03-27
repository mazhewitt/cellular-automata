## 1. Pure Rust Grid Logic

- [x] 1.1 Create `src/grid.rs` with constants (`GRID_WIDTH: usize = 256`, `GRID_HEIGHT: usize = 256`, `ALIVE: u8 = 255`) and `index(x, y) -> usize` function (`y * GRID_WIDTH + x`). Wire into `main.rs` with `mod grid`. _(Test: pure logic)_
- [x] 1.2 Implement `count_alive_neighbors(grid: &[u8], x: usize, y: usize) -> u8` with toroidal wrapping. Only cells with value `255` count as alive. _(Test: pure logic — corner, edge, interior cells)_
- [x] 1.3 Implement `step(src: &[u8], dst: &mut [u8])` applying birth/death/fade rules for all cells. Birth: dead cell + 3 alive neighbors → 255. Survival: alive cell + 2|3 alive neighbors → 255. Death: alive cell otherwise → 254. Dying: value 1–254 with 3 alive neighbors → 255 (rebirth), otherwise decrement. _(Test: pure logic)_
- [x] 1.4 Add `tests/grid_tests.rs` with required test cases: still life (block), oscillator (blinker period-2), glider (4-step movement), edge wrapping at `(0,0)`, birth, survival, death, fade decrement, dying rebirth, empty grid stays empty. _(Test: pure logic)_

## 2. Seed Patterns

- [x] 2.1 Add seed pattern functions in `grid.rs`: `seed_blinker(grid: &mut [u8], cx, cy)`, `seed_glider(grid: &mut [u8], cx, cy)`, `seed_r_pentomino(grid: &mut [u8], cx, cy)`. Each writes cell values relative to center position. _(Test: pure logic — pattern placement)_
- [x] 2.2 Add `--seed <name>` CLI argument parsing in `main.rs` (use `std::env::args`). Default to `r-pentomino` at grid center. Error with available names on unknown seed. _(Test: pure logic — argument parsing)_

## 3. Metal Shader

- [x] 3.1 Create `src/shaders/game_of_life.metal` with `update_cells` compute kernel: reads from buffer 0, writes to buffer 1, implements GoL birth/death/fade/wrapping rules matching Rust `step()`. Threadgroup size `(16, 16, 1)`. _(Test: GPU integration — shader compiles)_
- [x] 3.2 Add `fullscreen_quad_vertex` vertex function generating 6 vertices (2 triangles) from `vertex_id`, outputting clip-space position and UV coordinates. _(Test: GPU integration — pipeline creation)_
- [x] 3.3 Add `grid_fragment` fragment function: takes UV, reads grid buffer at `floor(uv * grid_size)`, maps cell value to brightness (`value / 255.0`), outputs color. _(Test: GPU integration — pipeline creation)_

## 4. Metal Pipeline Setup

- [x] 4.1 Add shader loading to `MetalRenderer`: load `game_of_life.metal` source at runtime via `device.new_library_with_source()`, store the library. _(Test: GPU integration — library compiles)_
- [x] 4.2 Create `MTLComputePipelineState` from the `update_cells` function. Store on `MetalRenderer`. _(Test: GPU integration — pipeline state creation)_
- [x] 4.3 Create `MTLRenderPipelineState` with `fullscreen_quad_vertex` and `grid_fragment`, pixel format `BGRA8Unorm`. Store on `MetalRenderer`. _(Test: GPU integration — pipeline state creation)_
- [x] 4.4 Allocate two `StorageModeShared` grid buffers (256×256 = 65,536 bytes each) and one uniform buffer on `MetalRenderer`. Add `current_buffer: usize` index. _(Test: GPU integration — buffer allocation)_

## 5. Uniform Buffer & Resize

- [x] 5.1 Define a `Uniforms` struct (grid_width: u32, grid_height: u32, cell_width: f32, cell_height: f32). Write initial values to the uniform buffer after allocation. _(Test: buffer — struct layout)_
- [x] 5.2 Update uniform buffer on window resize: recalculate cell pixel size from new drawable dimensions / grid dimensions. _(Test: smoke — resize doesn't crash)_

## 6. Frame Loop Integration

- [x] 6.1 Seed the initial pattern into grid buffer[0] from CPU via `contents()` pointer before the first frame. _(Test: GPU integration — seeded buffer contents)_
- [x] 6.2 Restructure `render_frame` → `render_frame(renderer, layer, current_buffer)`: encode compute pass (bind read buffer, write buffer, dispatch `(16, 16, 1)` threadgroups over 256×256), then encode render pass (bind newly-written grid buffer + uniform buffer, draw 6 vertices). Present, commit. _(Test: smoke — window shows grid)_
- [x] 6.3 After commit, swap `current_buffer ^= 1`. Wire frame loop to pass and update the current buffer index each frame. _(Test: smoke — animation runs)_

## 7. Cross-Validation Tests

- [x] 7.1 Add GPU integration test: seed blinker in shared buffer, dispatch compute shader for 1 step, `waitUntilCompleted`, read output buffer from CPU, compare byte-for-byte against Rust `step()` output. _(Test: GPU integration — critical: shader matches CPU)_
- [x] 7.2 Add GPU integration test: repeat cross-validation for glider pattern over 4 steps. _(Test: GPU integration)_

## 8. Buffer Management Tests

- [x] 8.1 Add `tests/buffer_tests.rs` with tests: `index(x, y)` correctness, double-buffer swap alternates 0/1, seed patterns place cells at expected positions. _(Test: pure logic)_
