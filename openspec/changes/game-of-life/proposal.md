## Why

The window bootstrap is complete — Metal device, CAMetalLayer, and a clear-screen render pass are working. The project's core purpose is exploring Apple Silicon UMA by running Conway's Game of Life with the CPU and GPU sharing the same physical memory. This change implements the actual simulation and rendering, which is the heart of the UMA learning objective.

## What Changes

- Add pure Rust GoL logic (`grid.rs`) operating on `&[u8]` slices — birth, death, fade, toroidal wrapping
- Add a Metal compute shader (`game_of_life.metal`) that implements the same GoL + fade rules on the GPU
- Add a Metal render pipeline: full-screen quad with a fragment shader that reads the grid buffer directly (zero-copy UMA read) and maps cell values to colors
- Allocate two `StorageModeShared` grid buffers for double-buffering; swap roles each frame
- Add a uniform buffer for render parameters (grid dimensions, cell size)
- Add seed patterns (blinker, glider, r-pentomino) with a default random seed
- Restructure the frame loop: compute pass → render pass → present → swap buffers
- Add pure-logic tests (GoL rules, wrapping, fade) and GPU integration tests (shader output matches CPU output)

### UMA Relevance

This is where unified memory becomes tangible. The compute shader writes cell state to a shared buffer; the fragment shader reads that same buffer in the render pass — no copies, no staging, no transfer. The CPU can also read the buffer directly after `waitUntilCompleted` for integration testing. Double-buffering demonstrates the coordination challenge: CPU and GPU must agree on which buffer is "current" without data races.

## Capabilities

### New Capabilities
- `cell-simulation`: Pure Rust GoL rules and Metal compute shader — cell lifecycle (alive 255 / dying 1–254 / dead 0), neighbor counting with toroidal wrap, fade decay, double-buffer swap
- `grid-rendering`: Full-screen quad rendered via vertex + fragment shader — reads grid buffer, maps cell values to colors (bright → fade → dark), optional grid lines
- `seed-patterns`: Named seed patterns (still lifes, oscillators, spaceships) placed onto the grid buffer from CPU before the first frame

### Modified Capabilities
- `metal-init`: Adding compute pipeline state, render pipeline state, double-buffer allocation, and uniform buffer to the Metal context

## Impact

- **New files**: `src/grid.rs`, `src/shaders/game_of_life.metal`
- **Modified files**: `src/metal_renderer.rs` (pipelines, buffers), `src/main.rs` (frame loop restructure)
- **New test files**: `tests/grid_tests.rs`, `tests/buffer_tests.rs` (pure logic); extended `tests/gpu_integration.rs` (shader correctness)
- **Dependencies**: No new crate dependencies expected (existing `metal`, `winit`, `objc` suffice)
- **No breaking changes** to the existing window bootstrap behavior
