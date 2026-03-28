## Context

The simulation currently seeds the grid once at startup (blinker, glider, or r-pentomino) and never modifies it again from the CPU side. The GPU compute shader handles all subsequent evolution. For a wallpaper-style ambient display, patterns eventually stabilise or die out, leaving a static or empty grid.

The grid uses double-buffered `StorageModeShared` MTLBuffers. The event loop uses `WaitUntil`-based timing — the CPU only wakes at tick boundaries (10 fps default). Between `commit()` returning and the next tick's command buffer submission, the CPU has a safe window to write into the "source" buffer that the GPU will read on the next frame.

## Goals / Non-Goals

**Goals:**
- Spawn a glider at a random position every 10–30 seconds to keep the grid alive
- Randomise glider rotation (4 orientations) for varied collision dynamics
- Write safely into the shared buffer at frame boundaries (after GPU commit, before next dispatch)
- Keep CPU overhead negligible — one RNG call + 5 cell writes per spawn

**Non-Goals:**
- Spawning other pattern types (r-pentomino, LWSS, etc.) — gliders only for now
- GPU-side spawning via a separate compute kernel
- User-configurable spawn interval (hardcoded 10–30s range)
- Click-to-spawn interaction (that's Phase 2 scope)

## Decisions

### 1. CPU-side placement into the source buffer

**Decision**: The CPU writes glider cells directly into `grid_buffers[current_buffer]` (the buffer the compute shader reads from next frame) during `RedrawRequested`, after the previous frame's command buffer has been committed.

**Rationale**: On UMA with `StorageModeShared`, there are no staging buffers or upload calls. The CPU just writes bytes. The safe window is between the previous `commit()` (which enqueues but doesn't block) and the next `waitUntilScheduled()`. Since our event loop only fires redraws at tick boundaries and we render synchronously within `RedrawRequested`, the write happens before the next command buffer is created.

**Alternative considered**: Writing in `AboutToWait` — rejected because `AboutToWait` doesn't guarantee the previous frame's GPU work is complete; placing writes at the start of the render path keeps the timing obvious.

### 2. Glider rotation via offset tables

**Decision**: Define four static offset arrays representing the four glider orientations (N, E, S, W) and select one randomly at spawn time. Reuse the existing `place()` function in `grid.rs`.

**Rationale**: Gliders have only 4 meaningful rotations. Static offset tables are zero-cost and trivially testable. No matrix math needed.

**Alternative considered**: A generic `rotate_pattern()` function — over-engineered for a 5-cell pattern with 4 fixed rotations.

### 3. `rand` crate with `thread_rng()`

**Decision**: Add the `rand` crate and use `thread_rng()` for both the spawn timer (uniform 10–30s) and position/rotation selection.

**Rationale**: `rand` is the standard Rust RNG crate. `thread_rng()` is cryptographically seeded, fast, and doesn't require manual seeding. The overhead is one call every 10–30 seconds — irrelevant.

**Alternative considered**: `fastrand` (smaller, no-std) — viable but `rand` has broader ecosystem support and we don't need no-std.

### 4. Spawn timer as `Instant` + random `Duration`

**Decision**: Track `next_spawn: Instant` initialised to `Instant::now() + random_duration(10..=30)`. On each tick, check if `now >= next_spawn`; if so, spawn a glider and reset `next_spawn` to a new random future time.

**Rationale**: This integrates cleanly with the existing `WaitUntil` event loop. No additional threads, no `tokio`, no timers. The check is one `Instant::elapsed()` comparison per frame (10 fps = 10 checks/sec).

## Risks / Trade-offs

- **[Race condition]** CPU writes into a buffer the GPU is reading → **Mitigation**: Write happens at the start of `RedrawRequested` before the command buffer is created. The previous frame's command buffer was committed on the prior tick. On Apple Silicon UMA, committed command buffers are scheduled immediately; by the next tick (100ms at 10fps), GPU work is long complete.

- **[Overwriting active cells]** Glider placed on top of existing alive cells → **Mitigation**: Acceptable — `place()` writes `255` (ALIVE) which is idempotent on alive cells. Writing onto dying/dead cells is the intended effect: injecting new life.

- **[Spawn clustering]** Multiple gliders spawning near each other → **Mitigation**: With a 256×256 grid and one spawn per 10–30s, the probability of overlap is low. No exclusion zone needed.
