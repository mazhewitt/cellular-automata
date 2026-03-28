## MODIFIED Requirements

### Requirement: Fragment shader reads grid buffer
The system SHALL provide a fragment shader that reads cell values directly from the grid buffer (zero-copy UMA access) and maps them to colors.

#### Scenario: Alive cell is bright
- **WHEN** the fragment shader reads a cell with value `255` in GoL mode
- **THEN** the pixel is rendered in a bright color (e.g., white or bright green)

#### Scenario: Dying cell fades
- **WHEN** the fragment shader reads a cell with value `v` where `1 ≤ v ≤ 254` in GoL mode
- **THEN** the pixel brightness is proportional to `v / 255.0` — higher values are brighter, lower values are dimmer

#### Scenario: Dead cell is dark
- **WHEN** the fragment shader reads a cell with value `0` in GoL mode
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
