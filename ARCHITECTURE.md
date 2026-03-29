# Architecture Principles

## Project Goal

A cellular automata playground that explores Apple Silicon Unified Memory Architecture (UMA) by splitting work between CPU and GPU using Rust and `metal-rs`. Supports two simulation modes: Game of Life and Physarum slime mould. Secondary goal: make it the smallest, most efficient implementation possible.

## Core Principle: Unified Memory is the Architecture

Every design decision flows from UMA. There are no buffer copies, no staging buffers, no upload/download cycles. CPU and GPU operate on the same physical memory. The interesting problem is **coordination**, not data transfer.

```
            ┌──────────────────────┐
            │    Shared Memory      │
            │  ┌────────────────┐  │
            │  │ Grid / Trail   │  │
            │  │ Buffers        │  │
            │  │ (one copy)     │  │
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

## Dual-Mode Architecture

The binary supports two simulation modes selected via `--mode gol` (default) or `--mode physarum`:

```
  main()
    ├── parse_args() → AppConfig { mode: SimMode, ... }
    ├── SimMode::GameOfLife → gol_renderer::run()
    └── SimMode::Physarum   → physarum_renderer::run()
```

Both modes share `MetalContext` (device + command queue + layer setup) and the `Uniforms` struct. Each mode has its own renderer, shader, and event loop.

## File Layout

| File | Purpose |
|------|---------|
| `src/main.rs` | CLI parsing, `SimMode` enum, window creation, mode dispatch (~100 lines) |
| `src/app.rs` | Shared constants: `TICK_RATES`, `SIGTERM_RECEIVED` |
| `src/game_of_life.rs` | GoL grid logic, seed patterns, `GridConfig` (CPU reference) |
| `src/physarum.rs` | Physarum CPU reference: config, agent step, diffuse/decay, agent init |
| `src/metal_context.rs` | `MetalContext` (device, queue, layer setup), `Uniforms`, `allocate_uniform_buffer()` |
| `src/gol_renderer.rs` | `GolRenderer` (GoL compute + render pipelines), `GoLState`, `run()` event loop |
| `src/physarum_renderer.rs` | `PhysarumRenderer` (Physarum compute + render pipelines), `run()` event loop |
| `src/wallpaper.rs` | macOS desktop wallpaper mode |
| `src/shaders/game_of_life.metal` | GoL compute + render shaders |
| `src/shaders/physarum.metal` | Physarum compute (agent_step, diffuse_decay) + render shaders |
| `tests/gpu_integration.rs` | GPU ↔ CPU cross-validation tests for both modes |

## Task Split Strategy

| Role | CPU | GPU |
|------|-----|-----|
| Window & events | Yes | — |
| Input handling | Yes | — |
| Sim control | Yes | — |
| GoL cell update | — | Compute shader (`update_cells`) |
| Physarum agent step | — | Compute shader (`agent_step`) |
| Physarum diffuse/decay | — | Compute shader (`diffuse_decay`) |
| Rendering (both modes) | — | Render pass (fullscreen quad + fragment shader) |

CPU reference implementations exist for both modes in pure Rust (`game_of_life.rs`, `physarum.rs`) and are used by GPU integration tests for cross-validation.

## Double Buffering

Both modes use double-buffered `StorageModeShared` buffers. After each frame the roles swap (pointer swap, not memcpy).

### Game of Life
Two grid buffers (`u8 × W×H`). Compute reads one, writes the other.

### Physarum
- **Agent buffer** (`float4 × N`): updated in-place each step (position, heading, species).
- **Two trail buffers** (`float × W×H×3` — three species planes each). Agent step deposits into the current buffer in-place (atomic adds). Diffuse/decay reads current, writes to alternate. Then swap.

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

### Phase 1: Foundation (two changes) ✅
- **Change 1** `metal-window-bootstrap`: winit window + Metal device + CAMetalLayer + clear-screen render pass
- **Change 2** `gpu-game-of-life`: Compute shader (GoL + fade), render pipeline (quad + fragment), double-buffer, seed patterns, frame loop

### Phase 1.5: Physarum Slime Mould ✅
- **Change 3** `physarum-slime-mould`: Second simulation mode — multi-species Physarum agent model with GPU compute (agent step + diffuse/decay), CPU reference, shared Metal context, `--mode physarum` CLI

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
