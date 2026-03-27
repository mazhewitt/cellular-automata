# Coding & Testing Rules

## Coding Rules

### Separation of Concerns

**GoL logic must be pure Rust, separate from Metal.** The Game of Life rules, grid indexing, neighbor counting, and seeding operate on `&[u8]` / `&mut [u8]` slices — never on `MTLBuffer` directly. At runtime, the slice comes from `MTLBuffer::contents()`; in tests, it comes from a `Vec<u8>`.

```
src/
├── grid.rs                ← Pure Rust: GoL rules, indexing, seeding
├── metal_renderer.rs      ← Metal: device, pipelines, command buffers
├── shaders/
│   └── game_of_life.metal ← MSL: compute + render shaders
└── main.rs                ← App entry: window, event loop, wiring
```

### Buffer Allocation

- All grid buffers use `MTLResourceStorageModeShared` — this is the UMA mode where CPU and GPU share the same physical memory with no copies
- Never use `StorageModePrivate` for grid data (defeats the learning purpose)
- Never use `StorageModeManaged` (unnecessary on Apple Silicon where all memory is unified)

### Minimal Allocation

- Exactly 2 grid buffers (double-buffer)
- 1 uniform buffer for render parameters (grid size, cell size, color params)
- No dynamic allocation per frame
- No `Vec` resizing at runtime

### Rust Conventions

- Rust 2024 edition
- No `unsafe` except where required for Metal/objc FFI — and keep those blocks as small as possible
- Prefer `metal-rs` safe wrappers over raw objc calls where available
- No `unwrap()` in production paths — use `expect()` with descriptive messages for Metal initialization that should never fail, propagate errors otherwise
- Keep `main.rs` thin: setup + event loop, delegate to modules

### Shader Conventions

- Metal Shading Language (MSL) files live in `src/shaders/`
- Compiled at build time or loaded as source at runtime via `device.new_library_with_source()`
- Compute shader threadgroup size: start with `(16, 16, 1)` — 256 threads, good default for 2D grids
- The compute shader and Rust `grid.rs` must implement identical GoL rules — they are validated against each other in tests

### Frame Loop Structure

```
loop {
    1. Poll window events (CPU)
    2. Create command buffer
    3. Encode compute pass (GPU: update cells)
    4. Encode render pass (GPU: draw grid)
    5. Present drawable
    6. Commit command buffer
    7. Swap buffer index
}
```

No work between commit and the next poll. CPU is idle while GPU executes.

---

## Testing Rules

### Layer Model

Tests are organized by what they exercise:

| Layer | What | How | Needs GPU? |
|-------|------|-----|------------|
| **Pure logic** | GoL rules, grid indexing, wrapping, fade | `cargo test` with `Vec<u8>` | No |
| **Buffer management** | Double-buffer swap, pixel-to-cell mapping | `cargo test` | No |
| **GPU integration** | Shader correctness, shared memory | `cargo test` with real `MTLDevice` (headless) | Yes |
| **Visual / smoke** | Window renders correctly, patterns animate | Manual with debug flags | Yes + display |

### Pure Logic Tests (Layer 1-2)

These are the foundation. Fast, deterministic, no GPU.

**Required test cases for GoL rules:**
- Still lifes: block (2x2), beehive, loaf remain unchanged after N generations
- Oscillators: blinker (period 2), toad (period 2) return to initial state
- Spaceships: glider moves diagonally over 4 generations
- Edge wrapping: cell at `(0,0)` correctly counts neighbors at `(width-1, height-1)` and adjacent edges
- Birth: dead cell with exactly 3 alive neighbors becomes alive (255)
- Survival: alive cell with 2 or 3 neighbors stays alive
- Death: alive cell with <2 or >3 neighbors begins dying (254)
- Fade: dying cell decrements each generation until reaching 0
- Empty grid stays empty
- Full grid follows rules correctly

**Required test cases for grid management:**
- `index(x, y)` returns `y * width + x`
- `pixel_to_cell(px, py, cell_size)` maps correctly
- Double-buffer swap alternates between 0 and 1
- Seed patterns place cells at expected positions

### GPU Integration Tests (Layer 3)

These run on macOS with Metal. No window required.

```
1. MTLDevice::system_default() succeeds
2. Allocate StorageModeShared buffer, write from CPU, read back → same data
3. Seed known pattern (blinker) in buffer A
4. Dispatch compute shader for 1 generation
5. Wait (commandBuffer.waitUntilCompleted)
6. Read buffer B from CPU
7. Assert buffer B matches CPU-computed expected output
```

**The critical test:** `gpu_step(pattern) == cpu_step(pattern)` for multiple known patterns. This validates:
- Shader compiles and dispatches without error
- Shader implements correct GoL rules
- Shared memory works (CPU reads GPU's output directly)
- Buffer layout agreement between CPU and GPU
- Threadgroup dispatch covers all cells (no missed cells at edges)

### Visual Smoke Tests (Layer 4)

Not automated. Supported by debug flags:

| Flag | Behavior |
|------|----------|
| `--seed blinker` | Seed a blinker at center |
| `--seed glider` | Seed a glider |
| `--seed r-pentomino` | Seed r-pentomino (chaos generator) |
| `--pause` | Start simulation paused |
| `--step` | Advance one frame per keypress (Space) |
| `--grid-lines` | Toggle grid line visibility |

Visual checks:
- Blinker blinks with period 2
- Glider moves down-right
- Grid lines align with cell boundaries
- Alive cells are bright, dying cells fade, dead cells are dark
- Window resizes correctly (Metal layer resizes, grid stays proportional)
- Esc closes cleanly

### Test File Organization

```
tests/
├── grid_tests.rs          ← Pure Rust: GoL rules, indexing, wrapping
├── buffer_tests.rs        ← Pure Rust: swap logic, seeding, mapping
└── gpu_integration.rs     ← Metal: shader correctness, shared memory
```

### What NOT to Test

- Metal API itself (Apple's problem)
- winit event loop mechanics (winit's problem)
- Visual pixel-perfect output (use eyes)
- Performance (Phase 3 concern — benchmark, don't unit test)
