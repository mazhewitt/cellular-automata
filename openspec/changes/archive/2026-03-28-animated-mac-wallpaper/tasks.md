## 1. CLI and Configuration

- [x] 1.1 Refactor `parse_seed_name()` into `parse_args()` returning a config struct with `seed: String` and `wallpaper: bool` — pure-logic unit test
- [x] 1.2 Parse `--wallpaper` flag in the new `parse_args()` — pure-logic unit test

## 2. Runtime Grid Dimensions

- [x] 2.1 Introduce a `GridConfig { width: usize, height: usize }` struct and thread it through `MetalRenderer::new()`, buffer allocation, and uniform setup — replacing compile-time `GRID_WIDTH`/`GRID_HEIGHT` for buffer sizing — buffer-layer test
- [x] 2.2 Compute aspect-ratio grid dimensions from screen size when `--wallpaper` is set (height=256, width=round(256 × screen_w / screen_h)); use 256×256 default otherwise — pure-logic unit test for the calculation

## 3. Dependencies

- [x] 3.1 Add `objc2` and `objc2-app-kit` to `Cargo.toml` and verify the project builds

## 4. Wallpaper Window Configuration

- [x] 4.1 Extract NSWindow pointer from winit's raw window handle via `objc2` (`[nsView window]`)
- [x] 4.2 Apply wallpaper-mode NSWindow properties: `setLevel:kCGDesktopWindowLevel`, `setStyleMask:Borderless`, `setCollectionBehavior:(CanJoinAllSpaces|Stationary|IgnoresCycle)`, `setHasShadow:NO`, `setIgnoresMouseEvents:YES`, `setFrame:screenFrame display:YES`
- [x] 4.3 Wire wallpaper configuration into `main()` — call after window creation when `wallpaper` flag is set

## 5. Screen Sizing

- [x] 5.1 Query `[NSScreen mainScreen].frame` via `objc2-app-kit` to get physical screen dimensions for grid computation and window frame
- [x] 5.2 Set initial drawable size to full-screen resolution in wallpaper mode and call `sync_drawable_size()` with the correct dimensions

## 6. SIGTERM Handling

- [x] 6.1 Register a SIGTERM handler (using `signal_hook` or raw `libc::signal`) that triggers clean exit — smoke test

## 7. Integration Testing

- [x] 7.1 GPU integration test: create a non-square grid (e.g., 32×20), run compute pass, verify output matches CPU `step()` — validates runtime grid dimensions in the shader pipeline
- [ ] 7.2 Manual smoke test: run `cargo run --release -- --wallpaper`, verify window sits behind all windows, cells are square, Escape exits cleanly (requires human verification)
