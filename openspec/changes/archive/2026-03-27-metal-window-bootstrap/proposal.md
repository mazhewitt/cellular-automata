## Why

This is the foundational change for the um-game-of-life project. Before we can run Game of Life simulations on the GPU, we need a working Metal rendering context: a window, a Metal device, a command queue, and a layer that can present GPU-rendered content to the screen. This is the platform plumbing that everything else builds on — compute shaders, render pipelines, and shared UMA buffers all require a functional Metal pipeline to begin with.

## UMA Relevance

This change establishes the Metal device and shared-mode buffer allocation pattern that underpins the entire UMA learning journey. Even the integration tests in this change demonstrate UMA: allocating a `StorageModeShared` buffer, writing from the CPU, and verifying the same memory is accessible — a trivial operation that would require explicit upload/download on discrete GPU architectures. The `CAMetalLayer` setup also prepares the drawable surface that later changes will render into directly from GPU compute output with zero copies.

## What Changes

- Add `metal`, `winit`, `objc`, and `core-graphics-types` as Cargo dependencies
- Create a macOS window (1024x1024) using `winit` with a standard event loop
- Initialize `MTLDevice`, `MTLCommandQueue`, and attach a `CAMetalLayer` to the window
- Implement a minimal render loop that clears the screen to a solid color each frame
- Handle window close (Esc / window X), resize (update layer drawable size), and basic event processing
- Add integration tests validating Metal device availability and shared buffer allocation

## Capabilities

### New Capabilities
- `metal-init`: Metal device initialization, command queue creation, and shared-mode buffer allocation
- `windowing`: macOS window lifecycle via winit — creation, event loop, resize handling, and clean shutdown

### Modified Capabilities

_None — this is a greenfield project._

## Impact

- **Dependencies**: Adds `metal`, `winit`, `objc`, `core-graphics-types` to Cargo.toml
- **Code**: New modules `src/metal_renderer.rs` (Metal setup + render loop) and updated `src/main.rs` (window + event loop entry point)
- **Tests**: New `tests/gpu_integration.rs` for Metal device and shared buffer validation
- **Platform**: macOS-only — will not compile on Linux/Windows (Metal is Apple-only)
