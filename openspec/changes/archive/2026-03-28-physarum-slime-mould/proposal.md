## Why

The project currently supports only Conway's Game of Life — a discrete cellular automaton with binary cell states. Adding a Physarum polycephalum (slime mould) simulation introduces an agent-based, continuous-valued model that produces organic, flowing visuals. This exercises new UMA patterns (large agent buffers, multi-pass compute, floating-point trail maps) and makes the wallpaper mode dramatically more visually striking with colored species trails on a black background.

## What Changes

- Add a `--mode physarum` CLI flag (default remains `gol`) to select the simulation at startup
- New Metal compute shader `physarum.metal` with two kernels: `agent_step` (sense/rotate/move/deposit) and `diffuse_decay` (3×3 blur + exponential decay on the trail map)
- New agent buffer: `float4` array (x, y, heading, species) for ~300k particles, allocated as `StorageModeShared`
- New trail map buffer: `float` array of `W × H × 3` (one layer per species), double-buffered
- New colored fragment shader that additively blends three species trails through a fixed palette (cyan, magenta, gold) onto a black background
- Fixed simulation parameters baked as constants in the shader (sensor angle, sensor distance, turn speed, move speed, deposit amount, decay factor) — no runtime tuning
- 3 agent species, each sensing only its own trail layer and repelled by others, producing interweaving colored network patterns

## Capabilities

### New Capabilities
- `physarum-simulation`: Agent-based Physarum model — agent lifecycle (sense/rotate/move/deposit), trail diffusion and decay, species separation, fixed parameter constants
- `simulation-mode`: CLI mode selection (`--mode gol|physarum`), startup pipeline branching, per-mode buffer allocation and kernel dispatch

### Modified Capabilities
- `grid-rendering`: Fragment shader gains a Physarum path that reads multi-species float trail maps and outputs additively blended color instead of grayscale brightness

## Impact

- **New files**: `src/shaders/physarum.metal`, `src/physarum.rs` (Rust-side agent init, CPU reference for testing)
- **Modified files**: `src/main.rs` (mode flag, pipeline branching), `src/metal_renderer.rs` (Physarum buffers + pipelines), `src/shaders/game_of_life.metal` (fragment shader extended or Physarum gets its own)
- **New dependency**: None — `rand` already available for agent initialisation
- **Buffer memory**: ~6 MB additional (4.8 MB agents + 1.4 MB trail map on a 455×256 grid) — trivial for Apple Silicon

## UMA Relevance

Physarum pushes UMA harder than GoL: agents are continuously written by GPU compute and the trail map is read/written across two compute passes per frame before the fragment shader samples it. The agent buffer is also a candidate for CPU-side inspection in tests (spawn agents at known positions, run one GPU step, read back positions). This exercises multi-buffer, multi-pass shared memory access patterns that GoL's single read-write swap doesn't reach.
