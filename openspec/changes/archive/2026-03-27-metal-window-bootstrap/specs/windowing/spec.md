## ADDED Requirements

### Requirement: Window creation
The system SHALL create a macOS window of 1024x1024 logical pixels with the title "Game of Life — Unified Memory".

#### Scenario: Window appears on launch
- **WHEN** the application starts
- **THEN** a 1024x1024 window appears on screen with the correct title

#### Scenario: Window is resizable
- **WHEN** the window is created
- **THEN** the user can resize the window by dragging its edges

### Requirement: Event loop
The system SHALL run a `winit` event loop that processes window events each frame and drives the Metal render loop.

#### Scenario: Render loop runs continuously
- **WHEN** the window is open and not paused
- **THEN** the event loop requests a redraw each frame and triggers the Metal render pass

#### Scenario: Event loop processes system events
- **WHEN** macOS delivers window events (focus, minimize, etc.)
- **THEN** the event loop handles them without blocking the render loop

### Requirement: Window close
The system SHALL close cleanly when the user presses Escape or clicks the window close button.

#### Scenario: Close via Escape key
- **WHEN** the user presses the Escape key
- **THEN** the event loop exits and the application terminates cleanly

#### Scenario: Close via window button
- **WHEN** the user clicks the window's close button (red circle)
- **THEN** the event loop exits and the application terminates cleanly

### Requirement: Resize handling
The system SHALL update the `CAMetalLayer` drawable size when the window is resized, using the window's scale factor to compute physical pixel dimensions.

#### Scenario: Resize updates drawable size
- **WHEN** the user resizes the window to a new logical size
- **THEN** the `CAMetalLayer` drawable size is updated to `(new_width * scale_factor, new_height * scale_factor)`

#### Scenario: HiDPI scaling is respected
- **WHEN** the window is displayed on a Retina display (scale factor 2.0)
- **THEN** the drawable size is double the logical window size in each dimension
