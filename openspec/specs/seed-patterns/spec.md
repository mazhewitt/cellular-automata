# Purpose

Define seed pattern selection and placement for initializing the Game of Life grid.

## Requirements

### Requirement: Named seed patterns
The system SHALL provide at least the following seed patterns: blinker, glider, and r-pentomino.

#### Scenario: Blinker seed
- **WHEN** the blinker pattern is seeded at position `(cx, cy)`
- **THEN** cells `(cx-1, cy)`, `(cx, cy)`, `(cx+1, cy)` are set to `255` (alive) and all other cells remain `0`

#### Scenario: Glider seed
- **WHEN** the glider pattern is seeded at position `(cx, cy)`
- **THEN** the five standard glider cells are set to `255` relative to `(cx, cy)`

#### Scenario: R-pentomino seed
- **WHEN** the r-pentomino pattern is seeded at position `(cx, cy)`
- **THEN** the five r-pentomino cells are set to `255` relative to `(cx, cy)`

### Requirement: Default seed
The system SHALL seed the grid with r-pentomino at the center by default if no `--seed` flag is provided.

#### Scenario: No seed flag uses default
- **WHEN** the application starts without a `--seed` argument
- **THEN** an r-pentomino is seeded at the center of the grid

### Requirement: Seed via CLI flag
The system SHALL accept a `--seed <name>` command-line argument to select the seed pattern.

#### Scenario: Selecting blinker via CLI
- **WHEN** the application starts with `--seed blinker`
- **THEN** a blinker pattern is seeded at the center of the grid

#### Scenario: Unknown seed name
- **WHEN** the application starts with `--seed unknown-name`
- **THEN** the application exits with an error listing available seed names

### Requirement: Seed placement from CPU
The system SHALL write seed patterns to the grid buffer from the CPU via the shared memory `contents()` pointer before the first compute dispatch.

#### Scenario: CPU writes seed to shared buffer
- **WHEN** a seed pattern is applied
- **THEN** the CPU writes cell values directly to the `StorageModeShared` buffer via `contents()` pointer — no GPU involvement
