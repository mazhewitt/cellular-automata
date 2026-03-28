## Context

The application currently creates a standard 1024×1024 winit window with a Metal `CAMetalLayer` attached to its `NSView`. The window uses default `NSWindowLevel` (normal), default style mask (title bar, close/minimize/resize buttons), and appears in Exposé and the Dock. To render as a live desktop wallpaper, the underlying `NSWindow` must be reconfigured to sit at the desktop level, span the full screen borderlessly, and opt out of system UI features like Mission Control and the App Switcher. Winit does not expose these macOS-specific APIs, so raw Objective-C FFI is required.

The Metal rendering pipeline (compute pass → render pass → present) and grid buffer architecture are resolution-independent by design — the fragment shader already uses uniform-based cell sizing. Changing the drawable size from 1024×1024 to a full screen resolution requires only updating the Metal layer's `drawableSize` and the uniform buffer, which the existing resize handler already does.

## Goals / Non-Goals

**Goals:**
- Run the Game of Life simulation as a desktop wallpaper layer behind all windows via `--wallpaper` CLI flag
- Borderless, fullscreen, non-activating window at `kCGDesktopWindowLevel`
- Invisible to Exposé, Mission Control, and the App Switcher
- Handles display resolution changes (e.g., switching monitors, clamshell mode)
- Default windowed mode remains completely unchanged
- Clean shutdown via Escape key and SIGTERM

**Non-Goals:**
- Multi-monitor support (render on all displays) — future scope, out of this change
- System Preferences wallpaper integration or screen saver framework — this is a standalone app
- Launch agent / launchd plist creation — manual `cargo run --release -- --wallpaper` for now
- Window decoration or transparency effects — the wallpaper is fully opaque

## Decisions

### 1. Access NSWindow via winit's raw window handle

**Decision:** Use `HasWindowHandle` → `RawWindowHandle::AppKit` to get the `NSView` pointer, then call `[nsView window]` via `objc2` to obtain the `NSWindow`. Configure window properties through `objc2` message sends.

**Alternatives considered:**
- *Create NSWindow manually without winit*: Would bypass winit entirely, losing the event loop, keyboard handling, and resize infrastructure. Too much reimplementation.
- *Use a fork of winit with level support*: Fragile, maintenance burden.

**Rationale:** The existing `raw_window_handle` pattern already works for Metal layer setup. Extending it to reach `NSWindow` is one additional `objc` call. All winit event loop infrastructure remains intact.

### 2. Window configuration strategy

**Decision:** After winit creates the window, apply wallpaper-mode overrides:
1. `[window setLevel: kCGDesktopWindowLevel]` — render behind all windows
2. `[window setStyleMask: NSWindowStyleMaskBorderless]` — no title bar
3. `[window setCollectionBehavior: NSWindowCollectionBehaviorCanJoinAllSpaces | NSWindowCollectionBehaviorStationary | NSWindowCollectionBehaviorIgnoresCycle]` — appear on all Spaces, excluded from Exposé/Cmd-Tab
4. `[window setFrame: screenFrame display: YES]` — fill the main screen
5. `[window setHasShadow: NO]` — no shadow for a wallpaper
6. `[window setIgnoresMouseEvents: YES]` — clicks pass through to Finder icons

**Alternatives considered:**
- *Set properties before showing*: winit shows the window during creation; post-creation override is the only option without custom window creation.

**Rationale:** Each property addresses a specific wallpaper requirement. The combination produces a window that behaves identically to macOS's own desktop wallpaper layer.

### 3. CLI flag in the existing argument parser

**Decision:** Extend `parse_seed_name()` to also parse `--wallpaper` as a boolean flag. Return a config struct (or two values) from the parser.

**Alternatives considered:**
- *Full CLI framework (clap)*: Overkill for two flags (`--seed`, `--wallpaper`).
- *Environment variable*: Less discoverable than a CLI flag.

**Rationale:** The hand-rolled parser handles the current `--seed` flag. Adding `--wallpaper` keeps it minimal and consistent.

### 4. Dependency: objc2 crate for NSWindow FFI

**Decision:** Use the `objc2` crate (already a transitive dependency of winit) for type-safe Objective-C calls. Import `objc2-app-kit` for `NSWindow`, `NSScreen`, and related types.

**Alternatives considered:**
- *Raw `objc` crate*: Less safe, string-based selectors, no compile-time checks.
- *`cocoa` crate*: Deprecated in favour of `objc2`.

**Rationale:** `objc2` provides typed bindings and is the direction the Rust macOS ecosystem is moving. Using it directly avoids duplicating the `objc` + `cocoa` pattern.

### 5. Screen size and display changes

**Decision:** On launch, query `[NSScreen mainScreen].frame` for the full screen size. On `WindowEvent::Resized` or display change, re-query and update the drawable. The existing `sync_drawable_size()` function handles Metal layer and uniform buffer updates.

**Rationale:** The resize path already works. Wallpaper mode just needs to set the initial size to the screen rather than 1024×1024.

### 6. Aspect-ratio grid sizing for square cells

**Decision:** In wallpaper mode, compute `GRID_WIDTH` and `GRID_HEIGHT` at runtime from the screen's aspect ratio so that cells render as perfect squares. Fix one dimension (height = 256) and derive width: `width = round(256 × screen_width / screen_height)`. For example, a 2560×1600 display (16:10) yields a 410×256 grid. The `grid::GRID_WIDTH` and `grid::GRID_HEIGHT` constants become initial defaults; wallpaper mode overrides them with computed values that are threaded through to buffer allocation, uniform setup, and compute dispatch.

**Alternatives considered:**
- *Letterbox / pillarbox*: Keep the grid square and black-bar the edges. Wastes screen space, looks wrong for a wallpaper.
- *Non-uniform cell size*: Let cells stretch. Visually unpleasant, breaks the Game of Life aesthetic.
- *Large square grid clamped to max dimension*: Wastes compute on off-screen cells.

**Rationale:** The shaders already read grid dimensions from the uniform buffer. The compute kernel dispatches over `(grid_width, grid_height)` threads. Making these values runtime costs nothing at the shader level. The only Rust-side change is moving grid dimensions from compile-time constants to a config struct passed through at init.

## Risks / Trade-offs

- **[Risk] macOS version compatibility** → The `kCGDesktopWindowLevel`, collection behaviors, and `ignoresMouseEvents` APIs have been stable since macOS 10.5. Low risk on any supported Apple Silicon Mac (macOS 11+). Mitigation: test on current macOS.
- **[Risk] winit may fight back on window properties** → winit internally manages some window state. Setting level/style after creation could be overridden by winit's own handlers. Mitigation: set properties after window creation and verify they stick. If needed, re-apply in `AboutToWait`.
- **[Risk] Full-screen resolution increases GPU work** → Rendering a wider grid (e.g., 410×256) to a 3456×2234 (14" MBP) drawable means more fragment shader invocations. Mitigation: the fragment shader is trivial (one buffer read, one colour output) — GPU load should remain negligible. Validate with Activity Monitor.
- **[Risk] Grid dimension change ripples through codebase** → Moving from compile-time constants to runtime grid dimensions touches buffer allocation, uniform setup, compute dispatch, seeding, and spawning. Mitigation: pass a `GridConfig { width, height }` struct; all hot paths already read dimensions from the uniform buffer (shaders) or function parameters (Rust). The compile-time constants remain as defaults for the windowed path.
- **[Trade-off] Mouse passthrough means no window interaction** → In wallpaper mode, the window ignores all mouse events. The only way to quit is Escape key or SIGTERM/kill. This is intentional for a wallpaper.
- **[Trade-off] Single display only** → This change targets the main screen. Multi-monitor would need one window per screen, deferred to a future change.
