## MODIFIED Requirements

### Requirement: Metal device initialization
The system SHALL obtain the default Metal device (`MTLDevice`) at startup via `MetalContext::new()` and fail with a descriptive error if no Metal-capable GPU is available. `MetalContext` SHALL be defined in a dedicated `metal_context` module.

#### Scenario: Successful device creation on Apple Silicon
- **WHEN** the application starts on a macOS system with Apple Silicon
- **THEN** `MetalContext::new()` returns a valid context holding a `MTLDevice` instance

#### Scenario: No Metal device available
- **WHEN** the application starts on a system without Metal support
- **THEN** `MetalContext::new()` returns an error indicating Metal is required

### Requirement: CAMetalLayer attachment
The system SHALL provide `MetalContext::setup_metal_layer()` as the single entry point for creating and configuring a `CAMetalLayer`. Per-mode renderer wrappers for layer setup SHALL NOT exist.

#### Scenario: Layer is configured correctly
- **WHEN** the Metal device and window are initialized
- **THEN** `MetalContext::setup_metal_layer()` creates a `CAMetalLayer` with the device set, pixel format set to `BGRA8Unorm`, and framebuffer-only set to true

#### Scenario: No duplicate layer setup methods
- **WHEN** `GolRenderer` or `PhysarumRenderer` needs a Metal layer
- **THEN** the caller uses `MetalContext::setup_metal_layer()` directly; neither renderer exposes its own `setup_metal_layer()` method

### Requirement: Compute pipeline state
The system SHALL create mode-specific `MTLComputePipelineState` objects within each renderer's module. GoL pipelines SHALL be created in `gol_renderer`, Physarum pipelines in `physarum_renderer`. Shader compilation helper functions SHALL be module-private.

#### Scenario: GoL compute pipeline creation succeeds
- **WHEN** `GolRenderer::new()` is called with a valid `MetalContext`
- **THEN** a `MTLComputePipelineState` is created from the `update_cells` function without error

#### Scenario: Physarum compute pipelines creation succeeds
- **WHEN** `PhysarumRenderer::new()` is called with a valid `MetalContext`
- **THEN** `MTLComputePipelineState` objects are created from `agent_step` and `diffuse_decay` functions without error

#### Scenario: Shader compilation is private
- **WHEN** shader library compilation functions exist in `gol_renderer` and `physarum_renderer`
- **THEN** they are module-private (`fn`, not `pub fn`) and not accessible from outside their module

### Requirement: Render pipeline state
The system SHALL create mode-specific `MTLRenderPipelineState` objects within each renderer's module. `GolRenderer` uses `fullscreen_quad_vertex` + `grid_fragment`. `PhysarumRenderer` uses `fullscreen_quad_vertex` + `physarum_fragment`.

#### Scenario: GoL render pipeline creation succeeds
- **WHEN** `GolRenderer::new()` is called
- **THEN** a `MTLRenderPipelineState` is created with GoL vertex and fragment functions

#### Scenario: Physarum render pipeline creation succeeds
- **WHEN** `PhysarumRenderer::new()` is called
- **THEN** a `MTLRenderPipelineState` is created with Physarum vertex and fragment functions

### Requirement: Shader library loading
The system SHALL load Metal shader sources at runtime via `device.new_library_with_source()`. Each renderer module SHALL load only its own shader source. The GoL shader source is `src/shaders/game_of_life.metal`. The Physarum shader source is `src/shaders/physarum.metal`.

#### Scenario: GoL library compiles successfully
- **WHEN** `GolRenderer::new()` loads the shader source
- **THEN** the Metal library compiles and contains `update_cells`, `fullscreen_quad_vertex`, and `grid_fragment`

#### Scenario: Physarum library compiles successfully
- **WHEN** `PhysarumRenderer::new()` loads the shader source
- **THEN** the Metal library compiles and contains `agent_step`, `diffuse_decay`, `fullscreen_quad_vertex`, and `physarum_fragment`
