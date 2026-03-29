## MODIFIED Requirements

### Requirement: Fragment shader reads grid buffer
The system SHALL provide per-mode fragment shaders that read cell/trail values directly from mode-specific buffers (zero-copy UMA access) and map them to colors.

#### Scenario: GoL alive cell is bright
- **WHEN** the GoL fragment shader reads a cell with value `255`
- **THEN** the pixel is rendered in a bright color (e.g., white or bright green)

#### Scenario: GoL dying cell fades
- **WHEN** the GoL fragment shader reads a cell with value `v` where `1 ≤ v ≤ 254`
- **THEN** the pixel brightness is proportional to `v / 255.0` — higher values are brighter, lower values are dimmer

#### Scenario: GoL dead cell is dark
- **WHEN** the GoL fragment shader reads a cell with value `0`
- **THEN** the pixel is rendered as the background color (dark, near-black)

#### Scenario: Physarum colored trail rendering
- **WHEN** the Physarum fragment shader processes a pixel at grid cell `(gx, gy)`
- **THEN** it reads trail intensities for all 3 species at that cell, multiplies each by its fixed palette color (species 0: cyan, species 1: magenta, species 2: gold), sums the results additively, and clamps to `[0, 1]`

#### Scenario: Physarum black background
- **WHEN** all species trail values at a pixel are `0.0`
- **THEN** the pixel is black `(0, 0, 0, 1)`

#### Scenario: Physarum color mixing at trail overlap
- **WHEN** two or more species have non-zero trail values at the same pixel
- **THEN** their palette contributions are additively blended, producing mixed hues (e.g., cyan + magenta → white/purple)

### Requirement: Uniform buffer for render parameters
The system SHALL provide a `StorageModeShared` uniform buffer containing grid width, grid height, and cell pixel size, updated on window resize. The uniform buffer SHALL be allocated via `MetalContext` and shared by both renderers.

#### Scenario: Uniforms are set before first frame
- **WHEN** the application initializes
- **THEN** the uniform buffer contains correct grid dimensions and cell size

#### Scenario: Uniforms update on resize
- **WHEN** the window is resized
- **THEN** the cell pixel size in the uniform buffer is recalculated from the new drawable dimensions
