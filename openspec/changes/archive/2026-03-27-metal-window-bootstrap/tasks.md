## 1. Dependencies & Project Setup

- [x] 1.1 Add `metal`, `winit`, `objc`, and `core-graphics-types` dependencies to Cargo.toml. Pin versions. Verify `cargo check` passes. _(Test: pure logic — compilation)_
- [x] 1.2 Create `src/metal_renderer.rs` module with a placeholder `MetalRenderer` struct. Wire it into `main.rs` with `mod metal_renderer`. Verify `cargo check` passes. _(Test: pure logic — compilation)_

## 2. Metal Device & Command Queue

- [x] 2.1 Implement `MetalRenderer::new()` that calls `Device::system_default()` to obtain the Metal device, failing with a descriptive error if unavailable. Create a `CommandQueue` from the device. _(Test: GPU integration — `MTLDevice::system_default()` returns `Some`)_
- [x] 2.2 Add integration test `tests/gpu_integration.rs` that verifies Metal device creation and command queue creation succeed. _(Test: GPU integration)_

## 3. Shared Buffer Validation

- [x] 3.1 Add a test in `tests/gpu_integration.rs` that allocates a `StorageModeShared` buffer, writes data from the CPU via `contents()`, reads it back, and asserts equality. This validates the UMA shared memory path. _(Test: GPU integration)_

## 4. Window Creation & Event Loop

- [x] 4.1 In `main.rs`, create a `winit` event loop and a 1024x1024 window titled "Game of Life — Unified Memory". Handle `CloseRequested` and Escape key to exit cleanly. _(Test: smoke — window appears and closes on Esc)_
- [x] 4.2 Handle window resize events: log or store the new size. Prepare for layer drawable size updates. _(Test: smoke — resize doesn't crash)_

## 5. CAMetalLayer Setup

- [x] 5.1 After the window is created, obtain the `NSView` via `raw-window-handle`, create a `CAMetalLayer`, set its device to the Metal device, set pixel format to `BGRA8Unorm`, set `framebufferOnly` to `true`, and attach it to the view. Set the initial `drawableSize` to the window's physical pixel dimensions (logical size × scale factor). _(Test: GPU integration — layer creation doesn't panic; smoke — window background changes)_
- [x] 5.2 On window resize, update the `CAMetalLayer` `drawableSize` to the new physical pixel dimensions. _(Test: smoke — resize updates correctly, no distortion)_

## 6. Clear-Color Render Pass

- [x] 6.1 Each frame (on `RedrawRequested`): acquire the next drawable from the `CAMetalLayer`, create a command buffer, create a `RenderPassDescriptor` that clears to a dark background color (e.g., `(0.05, 0.05, 0.05, 1.0)`), encode an empty render pass, present the drawable, and commit the command buffer. _(Test: smoke — window fills with solid dark color)_
- [x] 6.2 Request a redraw each frame to keep the render loop running continuously. _(Test: smoke — window stays rendered, doesn't flicker or go blank)_
