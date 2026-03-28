## MODIFIED Requirements

### Requirement: CAMetalLayer attachment
The system SHALL create a `CAMetalLayer`, configure it with the Metal device and `BGRA8Unorm` pixel format, and attach it to the window's `NSView`. The layer's `framebufferOnly` property SHALL be set to `true`.

#### Scenario: Layer is configured correctly
- **WHEN** the Metal device and window are initialized
- **THEN** a `CAMetalLayer` is created with the device set, pixel format set to `BGRA8Unorm`, and framebuffer-only set to true

#### Scenario: Layer drawable size matches window physical size
- **WHEN** the `CAMetalLayer` is attached to the window
- **THEN** the layer's `drawableSize` is set to the window's physical pixel dimensions

## ADDED Requirements

### Requirement: Compute pipeline state
The system SHALL create a `MTLComputePipelineState` from the `update_cells` kernel function in the compiled Metal shader library.

#### Scenario: Compute pipeline creation succeeds
- **WHEN** the Metal device and shader library are initialized
- **THEN** a `MTLComputePipelineState` is created from the `update_cells` function without error

### Requirement: Render pipeline state
The system SHALL create a `MTLRenderPipelineState` configured with the `fullscreen_quad_vertex` vertex function and `grid_fragment` fragment function, with pixel format `BGRA8Unorm`.

#### Scenario: Render pipeline creation succeeds
- **WHEN** the Metal device and shader library are initialized
- **THEN** a `MTLRenderPipelineState` is created with the vertex and fragment functions and correct pixel format

### Requirement: Double-buffer allocation
The system SHALL allocate exactly two `MTLBuffer` objects of size `256 * 256` bytes using `StorageModeShared` for the grid double-buffer.

#### Scenario: Two grid buffers allocated
- **WHEN** the Metal renderer initializes
- **THEN** two `StorageModeShared` buffers of 65,536 bytes each are allocated

### Requirement: Uniform buffer allocation
The system SHALL allocate one `MTLBuffer` using `StorageModeShared` for render uniforms (grid width, grid height, cell pixel size).

#### Scenario: Uniform buffer allocated
- **WHEN** the Metal renderer initializes
- **THEN** one `StorageModeShared` uniform buffer is allocated with capacity for render parameters

### Requirement: Shader library loading
The system SHALL load the Metal shader source from `src/shaders/game_of_life.metal` at runtime using `device.new_library_with_source()`.

#### Scenario: Library compiles successfully
- **WHEN** the shader source is loaded
- **THEN** the Metal library compiles without errors and contains the `update_cells`, `fullscreen_quad_vertex`, and `grid_fragment` functions
