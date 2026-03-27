## ADDED Requirements

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
The system SHALL allocate `MTLBuffer` objects using `StorageModeShared` so that CPU and GPU access the same physical memory with no copies.

#### Scenario: Buffer is writable from CPU
- **WHEN** a `StorageModeShared` buffer is allocated
- **THEN** the CPU can write data to the buffer via `contents()` pointer

#### Scenario: Buffer data persists for GPU access
- **WHEN** the CPU writes data to a shared buffer
- **THEN** the same data is accessible by the GPU without any explicit transfer or synchronization call

#### Scenario: Buffer is readable from CPU after GPU write
- **WHEN** the GPU writes data to a shared buffer and the command buffer completes
- **THEN** the CPU can read the GPU-written data directly from the same buffer via `contents()` pointer

### Requirement: CAMetalLayer attachment
The system SHALL create a `CAMetalLayer`, configure it with the Metal device and `BGRA8Unorm` pixel format, and attach it to the window's `NSView`.

#### Scenario: Layer is configured correctly
- **WHEN** the Metal device and window are initialized
- **THEN** a `CAMetalLayer` is created with the device set, pixel format set to `BGRA8Unorm`, and framebuffer-only set to true

#### Scenario: Layer drawable size matches window physical size
- **WHEN** the `CAMetalLayer` is attached to the window
- **THEN** the layer's `drawableSize` is set to the window's physical pixel dimensions (logical size x scale factor)

### Requirement: Clear-color render pass
The system SHALL encode a render pass each frame that clears the drawable to a solid background color.

#### Scenario: Frame renders solid color
- **WHEN** a new frame begins and a drawable is acquired from the layer
- **THEN** a command buffer is created, a render pass clears the drawable to a dark background color, the drawable is presented, and the command buffer is committed
