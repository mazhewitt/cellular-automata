## Why

The simulation currently seeds once at startup and then evolves until it fizzles out or reaches a steady state. For a wallpaper-style ambient display, the grid needs continuous injection of new life. Randomly spawning gliders at random positions on a 10–30 second interval keeps the grid alive indefinitely and creates emergent collisions between patterns.

This is also the first feature that requires the CPU to **write into a shared buffer that the GPU is actively reading** — a direct exploration of the UMA coordination problem outlined in Phase 2 of ARCHITECTURE.md.

## What Changes

- Add a random glider spawner that places a glider at a random grid position every 10–30 seconds (uniform random interval)
- Glider orientation is randomised (one of four rotations) to create varied collision patterns
- CPU writes into the active grid buffer, requiring frame-boundary synchronisation to avoid tearing
- Add `rand` crate dependency for random number generation

## Capabilities

### New Capabilities
- `random-spawn`: Timer-driven random placement of gliders into the live grid, with CPU→GPU buffer coordination at frame boundaries

### Modified Capabilities
- `metal-init`: Buffer access patterns change — CPU now writes to shared grid buffers during the event loop, not just at startup

## Impact

- **`src/main.rs`**: New spawn timer state, glider placement logic in the event loop at frame boundaries (between GPU commits)
- **`src/grid.rs`**: May need additional `seed_glider_rotated()` variants or a rotation parameter on `seed_glider()`
- **`Cargo.toml`**: New `rand` dependency
- **No shader changes**: Spawning is pure CPU write into the shared buffer; the compute shader sees alive cells as usual

## UMA Relevance

This is the canonical UMA coordination challenge: the CPU must write cell data into a `StorageModeShared` buffer that the GPU's compute shader reads from. Because both share the same physical memory, there are no copies — but writes must happen **outside** the GPU's command buffer execution window (between `commit()` and the next `waitUntilCompleted()` / next frame). This teaches safe CPU↔GPU interleaving on unified memory without fences or staging buffers.
