## ADDED Requirements

### Requirement: Cell state representation
The system SHALL represent each cell as a `u8` value where `255` means alive, `1–254` means dying (fading), and `0` means dead.

#### Scenario: Alive cell value
- **WHEN** a cell is born or survives
- **THEN** its value is set to `255`

#### Scenario: Dying cell fades each generation
- **WHEN** a cell has value `v` in range `1–255` and dies (fewer than 2 or more than 3 alive neighbors)
- **THEN** its value becomes `v - 1` in the next generation

#### Scenario: Dead cell stays dead without birth
- **WHEN** a cell has value `0` and does not have exactly 3 alive neighbors
- **THEN** its value remains `0`

### Requirement: Birth rule
The system SHALL set a dead cell (value `0`) to alive (`255`) when it has exactly 3 alive neighbors (value `255`).

#### Scenario: Dead cell with exactly 3 alive neighbors
- **WHEN** a dead cell has exactly 3 neighbors with value `255`
- **THEN** the cell becomes `255` (alive) in the next generation

#### Scenario: Dead cell with 2 alive neighbors
- **WHEN** a dead cell has 2 neighbors with value `255`
- **THEN** the cell remains `0` (dead)

### Requirement: Survival rule
The system SHALL keep an alive cell (`255`) alive when it has 2 or 3 alive neighbors.

#### Scenario: Alive cell with 2 alive neighbors survives
- **WHEN** an alive cell (value `255`) has exactly 2 neighbors with value `255`
- **THEN** the cell remains `255` in the next generation

#### Scenario: Alive cell with 3 alive neighbors survives
- **WHEN** an alive cell (value `255`) has exactly 3 neighbors with value `255`
- **THEN** the cell remains `255` in the next generation

### Requirement: Death rule
The system SHALL begin dying an alive cell when it has fewer than 2 or more than 3 alive neighbors.

#### Scenario: Alive cell with 0 or 1 alive neighbors dies
- **WHEN** an alive cell (value `255`) has fewer than 2 neighbors with value `255`
- **THEN** the cell becomes `254` (begin dying) in the next generation

#### Scenario: Alive cell with 4+ alive neighbors dies
- **WHEN** an alive cell (value `255`) has 4 or more neighbors with value `255`
- **THEN** the cell becomes `254` (begin dying) in the next generation

### Requirement: Dying cell continues fading
The system SHALL decrement a dying cell's value by 1 each generation until it reaches 0.

#### Scenario: Dying cell decrements
- **WHEN** a cell has value `v` where `1 ≤ v ≤ 254`
- **THEN** its value becomes `v - 1` in the next generation regardless of neighbor count

#### Scenario: Dying cell with 3 alive neighbors is reborn
- **WHEN** a dying cell (value `1–254`) has exactly 3 alive neighbors (value `255`)
- **THEN** the cell becomes `255` (alive) in the next generation

### Requirement: Toroidal wrapping
The system SHALL treat the grid as a torus — cells on edges wrap to the opposite side when counting neighbors.

#### Scenario: Top-left corner neighbor count
- **WHEN** counting neighbors of cell `(0, 0)` on a grid of size `W×H`
- **THEN** the system includes cells at `(W-1, H-1)`, `(0, H-1)`, `(1, H-1)`, `(W-1, 0)`, `(1, 0)`, `(W-1, 1)`, `(0, 1)`, and `(1, 1)`

#### Scenario: Right edge wraps to left
- **WHEN** counting neighbors of cell `(W-1, y)`
- **THEN** the neighbor at `(W, y)` is resolved as `(0, y)`

### Requirement: Grid indexing
The system SHALL use row-major indexing where cell `(x, y)` maps to buffer offset `y * width + x`.

#### Scenario: Index calculation
- **WHEN** accessing cell at column `x`, row `y` in a grid of width `W`
- **THEN** the buffer offset is `y * W + x`

### Requirement: Double-buffer swap
The system SHALL maintain two grid buffers and swap read/write roles each frame with no memory copy.

#### Scenario: Frame N reads buffer A, writes buffer B
- **WHEN** the current buffer index is `0`
- **THEN** the compute pass reads from buffer `0` and writes to buffer `1`

#### Scenario: Frame N+1 reads buffer B, writes buffer A
- **WHEN** the buffer index is toggled after frame N
- **THEN** the compute pass reads from buffer `1` and writes to buffer `0`

### Requirement: Compute shader implements GoL rules
The system SHALL provide a Metal compute shader that implements the same birth/death/fade/wrapping rules as the pure Rust implementation.

#### Scenario: Compute shader output matches CPU output
- **WHEN** the same initial grid state is processed for one generation by both the CPU (Rust) and GPU (compute shader)
- **THEN** the resulting grid buffers are byte-for-byte identical

#### Scenario: Compute shader dispatches over full grid
- **WHEN** the compute shader is dispatched with threadgroup size `(16, 16, 1)`
- **THEN** every cell in the 256×256 grid is processed (no missed cells at edges)
