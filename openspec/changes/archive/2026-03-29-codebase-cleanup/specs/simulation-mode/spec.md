## MODIFIED Requirements

### Requirement: Mode-specific event loop ownership
The system SHALL provide a per-mode `run()` function in each renderer module (`gol_renderer::run()`, `physarum_renderer::run()`) that owns the winit event loop for that mode. `main.rs` SHALL dispatch to the correct `run()` based on the selected `SimMode`.

#### Scenario: GoL mode dispatches to gol_renderer::run
- **WHEN** the application starts in `gol` mode
- **THEN** `main.rs` calls `gol_renderer::run(config, window, event_loop)` and the GoL event loop handles all windowing, input, and rendering

#### Scenario: Physarum mode dispatches to physarum_renderer::run
- **WHEN** the application starts in `physarum` mode
- **THEN** `main.rs` calls `physarum_renderer::run(config, window, event_loop)` and the Physarum event loop handles all windowing, input, and rendering

#### Scenario: main.rs contains no event loop
- **WHEN** inspecting `main.rs`
- **THEN** it contains only CLI parsing (`parse_args`), window/event-loop creation, and a `match` dispatch — no `Event::WindowEvent` handling or `event_loop.run()` calls

### Requirement: Tick rate applies to both modes
The system SHALL share tick-rate constants (`TICK_RATES`) and SIGTERM handling (`SIGTERM_RECEIVED`, `sigterm_handler`) from `main.rs`, used by both renderer `run()` functions.

#### Scenario: Arrow keys change tick rate in GoL
- **WHEN** the user presses arrow keys in GoL mode
- **THEN** the GoL event loop adjusts the tick rate using the shared `TICK_RATES` array

#### Scenario: Arrow keys change tick rate in Physarum
- **WHEN** the user presses arrow keys in Physarum mode
- **THEN** the Physarum event loop adjusts the tick rate using the shared `TICK_RATES` array

#### Scenario: SIGTERM exits cleanly in both modes
- **WHEN** a SIGTERM signal is received in either mode
- **THEN** the event loop exits cleanly via the shared `SIGTERM_RECEIVED` flag
