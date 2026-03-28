## ADDED Requirements

### Requirement: Agent representation
The system SHALL represent each Physarum agent as a `float4` value containing `(x, y, heading, species_id)`, stored in a single `StorageModeShared` MTLBuffer.

#### Scenario: Agent buffer allocation
- **WHEN** Physarum mode is selected at startup
- **THEN** the system allocates a shared agent buffer sized to hold `grid_width × grid_height × 2.5` agents (rounded to nearest integer), each as `float4`

#### Scenario: Agent initialisation
- **WHEN** the agent buffer is first allocated
- **THEN** each agent is initialised with a random position within the grid bounds, a random heading in `[0, 2π)`, and a species ID cycling through `{0, 1, 2}`

### Requirement: Agent step kernel
The system SHALL execute a Metal compute kernel `agent_step` each frame that updates every agent by sensing the trail map, rotating, moving, and depositing trail.

#### Scenario: Sense phase
- **WHEN** the agent step kernel executes for an agent with heading `θ` and species `s`
- **THEN** the kernel samples the trail map at three probe positions — `(x + d·cos(θ - α), y + d·sin(θ - α))`, `(x + d·cos(θ), y + d·sin(θ))`, `(x + d·cos(θ + α), y + d·sin(θ + α))` — where `d` is sensor distance and `α` is sensor angle, reading only species `s`'s trail layer

#### Scenario: Rotate phase
- **WHEN** the left probe value is highest
- **THEN** the agent's heading decreases by turn speed
- **WHEN** the right probe value is highest
- **THEN** the agent's heading increases by turn speed
- **WHEN** the centre probe value is highest (or tied)
- **THEN** the heading is unchanged

#### Scenario: Move phase
- **WHEN** the agent has updated its heading
- **THEN** the agent's position advances by `(move_speed · cos(heading), move_speed · sin(heading))` with toroidal wrapping at grid boundaries

#### Scenario: Deposit phase
- **WHEN** the agent has moved to its new position
- **THEN** the agent adds `DEPOSIT_AMOUNT` to its species' trail layer at the nearest grid cell

### Requirement: Trail map representation
The system SHALL maintain a trail map as a double-buffered `StorageModeShared` MTLBuffer of `float` values with planar layout: `[species_0: W×H][species_1: W×H][species_2: W×H]`.

#### Scenario: Trail buffer allocation
- **WHEN** Physarum mode is selected at startup
- **THEN** two trail map buffers are allocated, each sized `grid_width × grid_height × 3 × sizeof(float)` bytes

#### Scenario: Trail buffer initialisation
- **WHEN** the trail buffers are first allocated
- **THEN** all trail values are initialised to `0.0`

### Requirement: Diffuse and decay kernel
The system SHALL execute a Metal compute kernel `diffuse_decay` each frame that blurs and decays the trail map.

#### Scenario: 3×3 box blur diffusion
- **WHEN** the diffuse kernel processes cell `(x, y)` for species `s`
- **THEN** the output value is the mean of the 3×3 neighbourhood in species `s`'s trail layer of the source buffer, with toroidal wrapping at boundaries

#### Scenario: Exponential decay
- **WHEN** the diffused value has been computed
- **THEN** it is multiplied by `DECAY_FACTOR` (a constant less than 1.0) before writing to the destination buffer

### Requirement: Fixed simulation parameters
The system SHALL define all Physarum parameters as compile-time constants in the Metal shader, not as runtime-configurable uniforms.

#### Scenario: Parameter values
- **WHEN** the Physarum shader is compiled
- **THEN** the following constants are defined: `SENSOR_ANGLE` (radians), `SENSOR_DIST` (pixels), `TURN_SPEED` (radians), `MOVE_SPEED` (pixels/step), `DEPOSIT_AMOUNT` (float), `DECAY_FACTOR` (float, < 1.0)

### Requirement: Three species with separation
The system SHALL create agents distributed equally across 3 species (IDs 0, 1, 2), where each species senses only its own trail layer during the sense phase.

#### Scenario: Equal species distribution
- **WHEN** agents are initialised
- **THEN** approximately one-third of agents are assigned to each species

#### Scenario: Species-specific sensing
- **WHEN** an agent with species `s` executes the sense phase
- **THEN** it reads only from trail plane `s` in the trail map, ignoring other species' trails

### Requirement: Compute dispatch order
The system SHALL execute compute kernels in the order: `agent_step` (reads trail src, writes agent positions and trail dst), then `diffuse_decay` (blurs and decays trail dst), then swap trail buffer pointers.

#### Scenario: Per-frame compute sequence
- **WHEN** a frame begins
- **THEN** `agent_step` runs first using trail_src as input, then `diffuse_decay` runs on the deposited trail, then trail_src and trail_dst pointers swap

### Requirement: CPU reference implementation
The system SHALL provide Rust functions that replicate the agent_step and diffuse_decay logic on `&[f32]` slices and `&[[f32; 4]]` agent arrays for GPU integration testing.

#### Scenario: CPU agent step matches GPU
- **WHEN** the same initial agents and trail map are processed by CPU and GPU
- **THEN** the resulting agent positions and trail values match within epsilon tolerance (1e-4)

#### Scenario: CPU diffuse-decay matches GPU
- **WHEN** the same trail map is processed by CPU and GPU diffuse_decay
- **THEN** the resulting trail values match within epsilon tolerance (1e-4)
