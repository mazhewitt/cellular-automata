# Purpose

Define Metal initialization, pipeline setup, buffer allocation, and frame clear behavior for the macOS path.

## Requirements

### Requirement: Metal device initialization
The system SHALL obtain the default Metal device (`MTLDevice`) at startup and fail with a descriptive error if no Metal-capable GPU is available.

#### Scenario: Successful device creation on Apple Silicon
- **WHEN** the application starts on a macOS system with Apple Silicon
- **THEN** the system obtains a valid `MTLDevice` instance from `MTLCreateSystemDefaultDevice`

#### Scenario: No Metal device available
- **WHEN** the application starts on a system without Metal support
- **THEN** the system exits with an error message indicating Metal is required

### Requirement: Command queue creation
The system SHALL create a `MTLCommandQueue` from the Metal device at startup.

#### Scenario: Command queue is operational
- **WHEN** the Metal device has been initialized
- **THEN** the system creates a `MTLCommandQueue` capable of submitting command buffers

### Requirement: Shared-mode buffer allocation
The system SHALL allocate `MTLBuffer` objects using `StorageModeShared` so that CPU and GPU access the same physical memory with no copies. The CPU MAY write to the source grid buffer at frame boundaries during the event loop, not only at startup.

#### Scenario: Buffer is writable from CPU
- **WHEN** a `StorageModeShared` buffer is allocated
- **THEN** the CPU can write data to the buffer via `contents()` pointer

#### Scenario: Buffer data persists for GPU access
- **WHEN** the CPU writes data to a shared buffer
- **THEN** the same data is accessible by the GPU without any explicit transfer or synchronization call

#### Scenario: Buffer is readable from CPU after GPU write
- **WHEN** the GPU writes data to a shared buffer and the command buffer completes
- **THEN** the CPU can read the GPU-written data directly from the same buffer via `contents()` pointer

#### Scenario: CPU writes at frame boundaries are safe
- **WHEN** the CPU writes glider cells into the source grid buffer after the previous frame's commit and before the next command buffer creation
- **THEN** the GPU reads the updated data on the next compute dispatch without corruption

### Requirement: CAMetalLayer attachment
The system SHALL create a `CAMetalLayer`, configure it with the Metal device and `BGRA8Unorm` pixel format, and attach it to the window's `NSView`. The layer's `framebufferOnly` property SHALL be set to `true`.

#### Scenario: Layer is configured correctly
- **WHEN** the Metal device and window are initialized
- **THEN** a `CAMetalLayer` is created with the device set, pixel format set to `BGRA8Unorm`, and framebuffer-only set to true

#### Scenario: Layer drawable size matches window physical size
- **WHEN** the `CAMetalLayer` is attached to the window
- **THEN** the layer's `drawableSize` is set to the window's physical pixel dimensions

### Requirement: Clear-color render pass
The system SHALL encode a render pass each frame that clears the drawable to a solid background color.

#### Scenario: Frame renders solid color
- **WHEN** a new frame begins and a drawable is acquired from the layer
- **THEN** a command buffer is created, a render pass clears the drawable to a dark background color, the drawable is presented, and the command buffer is committed

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