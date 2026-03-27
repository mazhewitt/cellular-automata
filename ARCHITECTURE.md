# Architecture Principles

## Project Goal

A Game of Life implementation that explores Apple Silicon Unified Memory Architecture (UMA) by splitting work between CPU and GPU using Rust and `metal-rs`. Secondary goal: make it the smallest, most efficient GoL possible.

## Core Principle: Unified Memory is the Architecture

Every design decision flows from UMA. There are no buffer copies, no staging buffers, no upload/download cycles. CPU and GPU operate on the same physical memory. The interesting problem is **coordination**, not data transfer.

```
            ┌──────────────────────┐
            │    Shared Memory      │
            │  ┌────────────────┐  │
            │  │  Grid Buffers  │  │
            │  │  (one copy)    │  │
            │  └────┬─────┬────┘  │
            │       │     │       │
            └───────┼─────┼───────┘
                    │     │
               ┌────┘     └────┐
               ▼               ▼
          ┌─────────┐   ┌──────────┐
          │   CPU   │   │   GPU    │
          │ cores   │   │ cores    │
          └─────────┘   └──────────┘
```

## Task Split Strategy

| Role | CPU | GPU |
|------|-----|-----|
| Window & events | Yes | — |
| Input handling | Yes | — |
| Stats / FPS | Yes | — |
| Sim control | Yes | — |
| Cell update (GoL rules) | — | Compute shader |
| Rendering | — | Render pass |

The CPU sets up and submits work. The GPU executes compute and render. Both access shared buffers with zero copy.

## Double Buffering

Two `MTLBuffer` objects allocated with `StorageModeShared`. The compute shader reads from one, writes to the other. After each frame, the roles swap (pointer swap, not memcpy).

```
  Frame N                    Frame N+1
  Buffer A ──read──▶ GPU     Buffer B ──read──▶ GPU
  Buffer B ◀─write──┘        Buffer A ◀─write──┘
            ▲ swap ▲
```

## Rendering Pipeline

Option 1 (chosen): Full-screen quad with fragment shader.

- Compute pass writes cell state to shared buffer
- Render pass draws a full-screen quad
- Fragment shader samples the grid buffer using nearest-neighbor filtering
- Fragment shader maps cell values to colors (alive/dying/dead gradient)
- Grid lines rendered in the fragment shader (pixel position vs cell boundary)

This teaches both compute and render pipelines and demonstrates the UMA advantage: the render pass reads directly from the compute output with zero copy.

## Grid Representation

- Grid size: 256x256 (configurable, start here)
- Cell type: `u8` per cell
  - `255` = alive
  - `1..254` = dying (fading out, decremented each frame)
  - `0` = dead
- Buffer size: `width × height` bytes (64 KB for 256x256)
- Total memory: ~128 KB (two buffers) + uniforms

## Phase Roadmap

### Phase 1: Foundation (two changes)
- **Change 1** `metal-window-bootstrap`: winit window + Metal device + CAMetalLayer + clear-screen render pass
- **Change 2** `gpu-game-of-life`: Compute shader (GoL + fade), render pipeline (quad + fragment), double-buffer, seed patterns, frame loop

### Phase 2: User Interaction
- Click to spawn cells
- CPU writes to shared buffer → synchronization challenge
- Explore: frame-boundary writes, triple buffering, MTLEvent/MTLFence

### Phase 3: Optimization
- Bit-packing (64 cells per u64)
- Threadgroup optimization
- Benchmark CPU vs GPU crossover point
- Minimize everything

## Key Dependencies

| Crate | Purpose |
|-------|---------|
| `metal` (metal-rs) | Metal API bindings |
| `winit` | Window creation + event loop |
| `objc` / `core-graphics-types` | CAMetalLayer + macOS interop |

## Non-Goals (Phase 1)

- Cross-platform support (Metal-only, macOS/Apple Silicon)
- wgpu abstraction (we want raw Metal for UMA learning)
- HashLife or algorithmic optimization (Phase 3)
- Networking or multiplayer
