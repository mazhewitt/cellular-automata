# Purpose

Define selection between Game of Life and Physarum modes and how startup behavior branches by mode.

## Requirements

### Requirement: Mode CLI flag
The system SHALL accept a `--mode` command-line argument with values `gol` (default) or `physarum` to select the active simulation.

#### Scenario: Default mode is Game of Life
- **WHEN** the application starts without `--mode`
- **THEN** the Game of Life simulation runs with existing behaviour

#### Scenario: Physarum mode selected
- **WHEN** the application starts with `--mode physarum`
- **THEN** the Physarum slime mould simulation runs

#### Scenario: Invalid mode rejected
- **WHEN** the application starts with `--mode unknown`
- **THEN** the application exits with an error message listing valid modes

### Requirement: Pipeline branching at startup
The system SHALL create only the Metal pipelines and buffers required by the selected mode. GoL buffers and pipelines are not allocated when running in Physarum mode, and vice versa.

#### Scenario: GoL mode allocates GoL resources only
- **WHEN** the application starts in `gol` mode
- **THEN** only the GoL compute pipeline, render pipeline, and `u8` grid buffers are allocated

#### Scenario: Physarum mode allocates Physarum resources only
- **WHEN** the application starts in `physarum` mode
- **THEN** only the Physarum compute pipelines (agent_step, diffuse_decay), render pipeline, agent buffer, and float trail map buffers are allocated

### Requirement: Mode-agnostic event loop
The system SHALL run the same winit event loop regardless of mode, dispatching to mode-specific step and render functions.

#### Scenario: Tick rate applies to both modes
- **WHEN** the user presses arrow keys to change tick rate
- **THEN** the simulation speed changes for whichever mode is active

#### Scenario: Wallpaper mode works with both simulations
- **WHEN** `--wallpaper` is combined with `--mode physarum`
- **THEN** Physarum renders as a desktop wallpaper with the same window configuration as GoL wallpaper mode