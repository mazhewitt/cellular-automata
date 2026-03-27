## Context

This is a greenfield Rust project on macOS / Apple Silicon. The codebase currently contains only a `fn main()` printing "Hello, world!". We need to establish the Metal rendering foundation — device, window, layer, command queue, and a basic render loop — before any Game of Life simulation or GPU compute work can begin.

The project targets Apple Silicon exclusively and uses `metal-rs` (raw Metal bindings) rather than `wgpu` to maintain full visibility into UMA-specific APIs like `StorageModeShared`. Window management uses `winit` for cross-platform-style event handling, with platform-specific Metal layer attachment through `raw-window-metal`.

Key reference: [ARCHITECTURE.md](../../ARCHITECTURE.md) and [RULES.md](../../RULES.md).

## Goals / Non-Goals

**Goals:**
- A macOS window (1024x1024 logical pixels) that opens, renders, and responds to input
- A fully initialized Metal pipeline: device, command queue, and `CAMetalLayer` attached to the window
- A render loop that clears the drawable to a solid color each frame (proves the pipeline works end-to-end)
- Proper handling of window close (Esc key, window close button), resize (layer drawable size update), and macOS lifecycle events
- Integration tests that validate Metal device availability and `StorageModeShared` buffer allocation from Rust
- Module structure that separates Metal concerns from application logic (preparing for Change 2)

**Non-Goals:**
- Game of Life simulation logic (Change 2)
- Compute shaders or compute pipelines (Change 2)
- Fragment/vertex shaders beyond a clear-color pass (Change 2)
- User input beyond window close/resize (Phase 2)
- Performance measurement or optimization (Phase 3)
- Any cross-platform support

## Decisions

### 1. winit for windowing, raw metal-rs for GPU

**Decision**: Use `winit` for window creation and the event loop, `metal-rs` for all Metal API calls.

**Alternatives considered**:
- **wgpu**: Abstracts Metal away. Would hide `StorageModeShared` and other UMA-specific APIs. Rejected because the learning goal is UMA visibility.
- **raw AppKit via objc**: Maximum control but enormous boilerplate for window management. `winit` provides the same event model with far less code.
- **SDL2**: Would work but less idiomatic in the Rust ecosystem and heavier dependency.

**Rationale**: `winit` handles the window lifecycle and event loop cleanly. Metal pipeline setup is done through `metal-rs`, while `raw-window-metal` provides a small, focused helper for `CAMetalLayer` attachment. This keeps full control over Metal resources while minimizing custom unsafe interop.

### 2. CAMetalLayer attachment via raw-window-metal

**Decision**: After `winit` creates the window, obtain the `NSView` via `raw_window_handle`, then use `raw-window-metal` to create and attach the `CAMetalLayer` to the view. Configure the layer through `metal-rs` (`set_device`, `set_pixel_format`, `set_framebuffer_only`, `set_drawable_size`).

**Rationale**: `winit` doesn't natively expose a `CAMetalLayer`. `raw-window-metal` centralizes the AppKit attachment mechanics while preserving direct `metal-rs` control over layer configuration. This avoids brittle macro-level objc integration in app code and keeps the layer setup concise.

### 3. StorageModeShared for all buffers

**Decision**: All `MTLBuffer` allocations use `MTLResourceStorageModeShared`.

**Alternatives considered**:
- **StorageModePrivate**: GPU-only memory. Faster for GPU-only data but CPU can't read/write. Defeats the UMA learning goal.
- **StorageModeManaged**: Explicit CPU/GPU sync via `didModifyRange`. Unnecessary on Apple Silicon where all memory is physically unified.

**Rationale**: `StorageModeShared` is the UMA-native mode — CPU and GPU access the same physical memory with no copies, no sync calls, no staging buffers. This is the entire point of the project. Even in this bootstrap change where we only allocate a test buffer, we establish the pattern.

### 4. Module structure: bootstrap split, refactor-friendly

**Decision**: For bootstrap, `main.rs` owns event-loop orchestration, layer setup, and frame dispatch while `metal_renderer.rs` encapsulates device and command queue creation.

```
src/
├── main.rs              ← winit window + event loop + layer setup + frame dispatch
└── metal_renderer.rs    ← MetalRenderer struct: device + command queue
```

**Rationale**: This split keeps bring-up simple and explicit while preserving a clean seam for Change 2 refactoring. As compute and render complexity increases, layer ownership and render orchestration can move into a richer renderer/app-state abstraction.

### 5. Render loop: acquire-clear-present per frame

**Decision**: Each frame: acquire next drawable from `CAMetalLayer`, create a command buffer, encode a render pass that clears to a solid color, present the drawable, commit the command buffer.

**Rationale**: This is the minimal proof that the entire Metal pipeline works. If the window fills with a solid color, every component is functioning: device, queue, layer, drawable, command buffer, render pass descriptor, and presentation. This clear-color pass will be replaced by the full-screen quad render pass in Change 2.

## Risks / Trade-offs

- **[Risk] metal-rs API changes** → The `metal` crate has been relatively stable, but it wraps Apple's Objective-C Metal framework. Pin the dependency version in Cargo.toml.
- **[Risk] Layer lifetime management is manual** → The bootstrap implementation intentionally holds layer lifetime for process duration. Mitigation: in the planned refactor, move owned layer/application state into a struct with explicit drop semantics.
- **[Risk] Retina / HiDPI scaling** → macOS windows have a scale factor (typically 2x on Retina). The `CAMetalLayer` drawable size must be set in physical pixels, not logical. Mitigation: use `winit`'s `scale_factor()` and `inner_size()` to compute physical size, update on resize.
- **[Trade-off] macOS-only** → Acceptable for this project. No abstraction layer needed.
- **[Trade-off] No render content yet** → The clear-color pass produces a visually boring result, but it validates the entire pipeline. Real rendering comes in Change 2.
