## Context

The project is a Rust/Metal cellular automata viewer on macOS exploring Unified Memory Architecture. It currently runs Conway's Game of Life with a single compute kernel, double-buffered `u8` grid, and a grayscale fragment shader. The wallpaper mode (desktop-level, borderless, aspect-ratio grid) is already implemented. CLI parsing (`parse_args() → AppConfig`) supports `--seed` and `--wallpaper`.

Adding Physarum requires: a new agent buffer, a multi-layer float trail map, two new compute kernels, a colored fragment shader, and a startup branch that selects which pipeline runs. The existing GoL path must remain untouched.

## Goals / Non-Goals

**Goals:**
- Add `--mode physarum` CLI flag to select Physarum simulation at startup (default: `gol`)
- Implement the Jones 2010 Physarum agent model on GPU: sense → rotate → move → deposit per agent, plus trail diffuse + decay
- 3 species with fixed palette (cyan, magenta, gold) additively blended on black
- Fixed simulation parameters — no runtime tuning UI
- GPU integration tests that validate agent step and diffuse/decay kernels against CPU reference
- Wallpaper mode works with either simulation

**Non-Goals:**
- Runtime parameter tuning or GUI controls
- More than 3 species
- GoL visual changes (stays grayscale)
- Interaction (mouse clicks, agent spawning mid-run)
- Multi-display support changes

## Decisions

### Decision 1: Separate shader file for Physarum

**Choice:** New `src/shaders/physarum.metal` with `agent_step`, `diffuse_decay`, and `physarum_fragment` functions.

**Alternatives considered:**
- Extend `game_of_life.metal` — rejected: the two simulations share no compute logic; combining them bloats one file and complicates the pipeline state objects.
- Runtime shader selection via function constants — over-engineered for a compile-time mode switch.

**Rationale:** Clean separation. Each shader file compiles to its own `MTLLibrary`. The Rust side picks which library to load at startup.

### Decision 2: Startup branch via `SimMode` enum, not trait

**Choice:** `enum SimMode { GameOfLife, Physarum }` parsed from `--mode`. A `match` in `main()` selects which buffers to allocate and which step/render functions to call.

**Alternatives considered:**
- `trait Simulation` with dynamic dispatch — unnecessary abstraction for two modes with different buffer layouts. The trait boundary would leak buffer-type details.
- Feature flags — prevents running both modes from the same binary.

**Rationale:** Option C from exploration. Minimal abstraction. A match statement is clear, easy to extend, and avoids trait-object overhead.

### Decision 3: Trail map as `float` with species-interleaved layout

**Choice:** Trail map buffer layout `[species_0: W×H floats][species_1: W×H floats][species_2: W×H floats]` — three contiguous planes. Double-buffered (two buffers, pointer-swap each frame like GoL).

**Alternatives considered:**
- Interleaved `float3` per cell — worse cache locality for the diffuse kernel which processes one species plane at a time.
- `half` precision — saves memory but `float` atomics are simpler and 6 MB total is negligible.

**Rationale:** Planar layout matches the compute dispatch pattern: agent_step writes to its own species plane; diffuse_decay processes each plane independently. Same double-buffer swap pattern as GoL grid.

### Decision 4: Agent buffer as `float4` array

**Choice:** Each agent is `float4(x, y, heading, species_id)`. Single buffer, `StorageModeShared`, ~300k agents. Not double-buffered — agents are updated in-place.

**Alternatives considered:**
- Struct-of-arrays (separate x[], y[], heading[], species[]) — better for SIMD but more buffers to manage, and Metal compute threads process one agent each anyway.
- Double-buffered agents — unnecessary since each agent's update depends only on the trail map (read from previous frame), not on other agents.

**Rationale:** `float4` is naturally aligned for Metal. In-place update is safe because agents read the trail map (previous frame's buffer) and write only their own position. No agent-to-agent dependency.

### Decision 5: Fixed parameters as shader constants

**Choice:** All Physarum parameters (`SENSOR_ANGLE`, `SENSOR_DIST`, `TURN_SPEED`, `MOVE_SPEED`, `DEPOSIT_AMOUNT`, `DECAY_FACTOR`) defined as `constant float` in the Metal shader.

**Alternatives considered:**
- Uniforms buffer with runtime values — enables tuning but adds UI complexity we explicitly don't want.

**Rationale:** Known-good defaults from the Jones 2010 paper. Compiler can optimize with constants. Changing parameters requires a recompile, which is acceptable — this is wallpaper, not an interactive editor.

### Decision 6: Colored fragment shader with additive blending

**Choice:** Physarum fragment shader reads all 3 species trail intensities at each pixel, multiplies by fixed palette colors, sums additively, and clamps. Black background (no clear color needed — pixels default to `float4(0,0,0,1)`).

**Alternatives considered:**
- Separate render pass per species with hardware blend — more passes, worse performance for no visual gain.
- HSV mapping from single trail value — loses the multi-species interplay that makes Physarum visually interesting.

**Rationale:** Single-pass additive blend in the fragment shader is the simplest approach. Where trails overlap, colors mix naturally (cyan + magenta → white/purple), producing the luminous vein effect.

### Decision 7: CPU reference implementation for testing

**Choice:** `src/physarum.rs` contains pure-Rust equivalents of `agent_step` and `diffuse_decay` operating on `&[f32]` slices and `&[f32; 4]` agent arrays. GPU integration tests: seed known agents, run one GPU step, compare against CPU output (with epsilon tolerance for float).

**Rationale:** Follows the established GoL pattern — Rust CPU logic mirrors Metal GPU logic, GPU integration tests validate byte-for-byte (or epsilon-close) equivalence. The CPU code is only used in tests; production always runs on GPU.

## Risks / Trade-offs

- **[Float precision divergence]** GPU and CPU float math may differ slightly → Mitigation: use epsilon comparison (1e-4) in GPU integration tests instead of exact equality, and keep operations simple (no transcendentals beyond `sin`/`cos` for heading).
- **[Agent count tuning]** 300k agents may be too sparse or too dense depending on grid size → Mitigation: scale agent count proportionally to grid area (`agents = grid_width * grid_height * 2.5`). Start with this ratio and adjust if visual quality is poor.
- **[No in-place atomic writes for trail deposits]** Multiple agents may deposit to the same cell simultaneously → Mitigation: use `atomic_fetch_add_explicit` on the trail map buffer, or accept minor race conditions (visual artifacts are barely perceptible at 300k agents and add organic randomness).
- **[Shader compilation time]** Two Metal libraries instead of one → Mitigation: only compile the library for the selected mode. GoL mode never touches `physarum.metal`.
