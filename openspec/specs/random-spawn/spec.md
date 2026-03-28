# Purpose

Define periodic random glider spawning behavior, including timing, positioning, orientation, and buffer write safety.

## Requirements

### Requirement: Periodic random glider spawning
The system SHALL spawn a single glider at a random grid position at random intervals between 10 and 30 seconds (inclusive, uniform distribution).

#### Scenario: First glider spawns within 30 seconds of startup
- **WHEN** the application has been running for 30 seconds
- **THEN** at least one glider has been spawned at a random position on the grid

#### Scenario: Spawn interval is randomised
- **WHEN** a glider has just been spawned
- **THEN** the next spawn is scheduled at a uniformly random time between 10 and 30 seconds in the future

#### Scenario: Spawning continues indefinitely
- **WHEN** the application is running and the simulation is active
- **THEN** gliders continue to be spawned at the random interval without stopping

### Requirement: Random glider position
The system SHALL place each spawned glider at a uniformly random (x, y) position within the grid bounds (0..GRID_WIDTH, 0..GRID_HEIGHT), with wrapping for cells that extend beyond edges.

#### Scenario: Glider placed at random coordinates
- **WHEN** a glider spawn is triggered
- **THEN** the glider centre position is chosen uniformly at random from (0..256, 0..256)

#### Scenario: Glider near grid edge wraps correctly
- **WHEN** a glider is spawned with centre at (255, 255)
- **THEN** glider cells that exceed grid bounds wrap to the opposite edge using modular arithmetic

### Requirement: Random glider orientation
The system SHALL randomly select one of four glider rotations (0°, 90°, 180°, 270°) for each spawn, with uniform probability.

#### Scenario: All four rotations are possible
- **WHEN** many gliders are spawned over time
- **THEN** all four rotation variants appear (statistically)

#### Scenario: Each rotation produces a valid glider
- **WHEN** a glider is spawned with any of the four rotations
- **THEN** the placed cells form a valid 5-cell glider pattern that evolves correctly under Game of Life rules

### Requirement: Frame-boundary buffer safety
The system SHALL write spawned glider cells into the shared grid buffer only at frame boundaries — after the previous frame's GPU command buffer has been committed and before the current frame's command buffer is created.

#### Scenario: No tearing or corruption from CPU write
- **WHEN** a glider is spawned into the grid buffer
- **THEN** the write occurs before the compute pass command buffer is encoded for the current frame

#### Scenario: Spawned cells are visible in the next simulation step
- **WHEN** a glider is written to the source buffer at a frame boundary
- **THEN** the compute shader reads the newly written alive cells on the next dispatch and applies GoL rules to them
