## MODIFIED Requirements

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
