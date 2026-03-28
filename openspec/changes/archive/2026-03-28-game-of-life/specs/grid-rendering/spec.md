## ADDED Requirements

### Requirement: Full-screen quad rendering
The system SHALL render a full-screen quad each frame using a vertex shader that generates vertices from `vertex_id` with no vertex buffer.

#### Scenario: Quad covers entire drawable
- **WHEN** the render pass executes
- **THEN** a quad covering the full drawable area is drawn using 6 vertices (two triangles) generated entirely in the vertex shader

### Requirement: Fragment shader reads grid buffer
The system SHALL provide a fragment shader that reads cell values directly from the grid buffer (zero-copy UMA access) and maps them to colors.

#### Scenario: Alive cell is bright
- **WHEN** the fragment shader reads a cell with value `255`
- **THEN** the pixel is rendered in a bright color (e.g., white or bright green)

#### Scenario: Dying cell fades
- **WHEN** the fragment shader reads a cell with value `v` where `1 ≤ v ≤ 254`
- **THEN** the pixel brightness is proportional to `v / 255.0` — higher values are brighter, lower values are dimmer

#### Scenario: Dead cell is dark
- **WHEN** the fragment shader reads a cell with value `0`
- **THEN** the pixel is rendered as the background color (dark, near-black)

### Requirement: Grid-to-pixel mapping
The system SHALL compute cell pixel size from the window dimensions and grid size, so the grid fills the window.

#### Scenario: Cell size calculation
- **WHEN** the window drawable is `W×H` pixels and the grid is `GW×GH` cells
- **THEN** each cell occupies `floor(W / GW)` by `floor(H / GH)` pixels

#### Scenario: Fragment maps pixel to cell
- **WHEN** the fragment shader processes a pixel at position `(px, py)`
- **THEN** it reads the cell at grid index `(floor(px / cell_width), floor(py / cell_height))`

### Requirement: Uniform buffer for render parameters
The system SHALL provide a `StorageModeShared` uniform buffer containing grid width, grid height, and cell pixel size, updated on window resize.

#### Scenario: Uniforms are set before first frame
- **WHEN** the application initializes
- **THEN** the uniform buffer contains correct grid dimensions and cell size

#### Scenario: Uniforms update on resize
- **WHEN** the window is resized
- **THEN** the cell pixel size in the uniform buffer is recalculated from the new drawable dimensions
