## Context

The window bootstrap is complete: Metal device, command queue, CAMetalLayer, and a clear-color render pass are operational (`metal-window-bootstrap`, now archived). The codebase has `MetalRenderer` (device + queue), an event loop, and GPU integration tests proving `StorageModeShared` buffers work for CPU↔GPU data sharing.

This change adds the actual Game of Life — compute shader for cell updates, render pipeline for visualization, pure Rust logic for testing, and double-buffered shared memory to tie it all together.

Constraints from ARCHITECTURE.md and RULES.md:
- GoL logic must be pure Rust on `&[u8]` slices, separate from Metal
- Compute shader and Rust must implement identical rules, validated by cross-tests
- All grid buffers use `StorageModeShared` (the UMA learning point)
- Exactly 2 grid buffers, 1 uniform buffer, no per-frame allocation
- Grid: 256×256, `u8` per cell (255 alive, 1–254 dying/fading, 0 dead)
- Threadgroup size: `(16, 16, 1)`

## Goals / Non-Goals

**Goals:**
- Implement GoL cell update rules in both Rust and MSL compute shader
- Render the grid as a full-screen quad with a fragment shader reading directly from the shared buffer
- Double-buffer grid state with pointer swap (not memcpy)
- Provide seed patterns (blinker, glider, r-pentomino) selectable via CLI flags
- Validate GPU output matches CPU output via integration tests

**Non-Goals:**
- User interaction (clicking to spawn cells) — Phase 2
- Performance optimization (bit-packing, threadgroup tuning) — Phase 3
- Cross-platform support (Metal-only)
- Grid size configurability at runtime (hardcoded 256×256 for now)

## Decisions

### 1. Pure Rust grid module (`grid.rs`) for GoL logic

**Choice:** All GoL rules (neighbor counting, birth/death/fade, wrapping) live in `grid.rs` and operate on `&[u8]` / `&mut [u8]` slices.

**Rationale:** Enables fast, deterministic unit tests with `Vec<u8>` — no GPU required. The compute shader reimplements the same logic for production speed. Cross-validation tests (GPU output == CPU output) catch divergence.

**Alternative considered:** GoL logic only in the shader, CPU just seeds. Rejected because it makes testing much harder and loses the educational value of seeing the same algorithm in two languages.

### 2. Single MSL file with both compute and fragment functions

**Choice:** `src/shaders/game_of_life.metal` contains the compute kernel (`update_cells`), vertex function (`fullscreen_quad_vertex`), and fragment function (`grid_fragment`).

**Rationale:** These are tightly coupled (shared grid format, same buffer layout). A single file keeps the buffer layout defined once. Loaded at runtime via `device.new_library_with_source()` for simpler iteration during development.

**Alternative considered:** Separate `.metal` files per stage. Rejected — adds complexity for no benefit at this scale.

### 3. Full-screen quad via vertex shader (no vertex buffer)

**Choice:** The vertex shader generates a full-screen triangle or quad from `vertex_id` (0–5) with no vertex buffer. The fragment shader computes grid coordinates from fragment position.

**Rationale:** Zero vertex buffer allocation. The fragment shader does `floor(frag_coord / cell_pixel_size)` to find the grid cell, then reads the grid buffer at that index. This is the standard Metal/GL fullscreen-quad technique.

**Alternative considered:** Instanced rendering (one quad per cell). Rejected — 65,536 instances is wasteful when a single full-screen draw with buffer lookup is simpler and faster.

### 4. Uniform buffer for render parameters

**Choice:** A single `StorageModeShared` uniform buffer containing: grid width, grid height, cell pixel size (computed from window size / grid size), and color parameters.

**Rationale:** The fragment shader needs these to map pixel coordinates to grid cells. Updating on resize is a single CPU write to shared memory — no buffer recreation.

### 5. Double-buffer swap via index toggle

**Choice:** Two `MTLBuffer` objects stored in an array. A `current_buffer: usize` index (0 or 1) indicates the read buffer; `1 - current_buffer` is the write buffer. Swap is `current_buffer ^= 1` after each frame.

**Rationale:** Simplest possible coordination. No fence/event needed because compute and render are sequential within the same command buffer, and we swap after commit.

### 6. Frame loop: compute → render in one command buffer

**Choice:** Each frame encodes compute pass (read buffer[i], write buffer[1-i]) then render pass (read buffer[1-i]) in a single command buffer. Present and commit. Then swap index.

**Rationale:** Sequential encoding within one command buffer means Metal guarantees execution order. No explicit synchronization needed between compute and render. The render pass reads from the buffer that compute just wrote — the freshest state.

```
Command Buffer:
  ┌─ Compute Pass ──────────────────────┐
  │  read: grid_buffers[current]        │
  │  write: grid_buffers[1 - current]   │
  └─────────────────────────────────────┘
  ┌─ Render Pass ───────────────────────┐
  │  read: grid_buffers[1 - current]    │
  │  (the buffer compute just wrote)    │
  └─────────────────────────────────────┘
  present(drawable)
commit()
current ^= 1
```

## Risks / Trade-offs

- **[Shader/CPU divergence]** → Mitigated by cross-validation tests: seed a known pattern, run one step on GPU, compare to CPU output byte-for-byte.
- **[Runtime shader compilation latency]** → Acceptable for a learning project. First frame may stall while MSL compiles. Could pre-compile to a metallib later if needed.
- **[Fixed 256×256 grid]** → Keeps the first implementation simple. Grid size is a constant; making it configurable is a Phase 3 concern.
- **[No explicit GPU-CPU sync for double buffer]** → Safe because compute and render are in the same command buffer (sequential), and index swap happens after commit on CPU. If we later add CPU reads mid-frame, we'll need `waitUntilCompleted` or `MTLEvent`.
