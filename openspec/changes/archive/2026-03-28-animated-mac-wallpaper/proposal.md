## Why

The Game of Life simulation is already optimised for ambient display — low CPU (1.4%), minimal GPU usage, and continuous random glider spawning that keeps the grid alive indefinitely. The natural next step is to render it as a live desktop wallpaper layer on macOS, sitting behind all windows. This transforms a learning project into a practical daily-driver tool and exercises a new macOS API surface (NSWindow levels, borderless fullscreen, Spaces integration) while preserving the existing UMA rendering pipeline unchanged.

## What Changes

- Add a `--wallpaper` CLI flag that switches the window into desktop wallpaper mode
- Set the NSWindow level to `kCGDesktopWindowLevel` so the window renders behind all other windows, directly on the desktop
- Make the window borderless (`NSWindowStyleMaskBorderless`), non-activating, and spanning the full screen
- Exclude the window from Exposé / Mission Control / App Switcher
- Size the Metal drawable to the full screen resolution (physical pixels) and update on display change
- Resize the simulation grid to match the screen's aspect ratio so cells remain square (e.g., 410×256 on a 16:10 display instead of 256×256)
- Keep the default windowed mode unchanged when `--wallpaper` is not passed
- Graceful termination: Escape key still exits, plus respond to SIGTERM for launchd/daemon use

## Capabilities

### New Capabilities
- `wallpaper-mode`: Desktop wallpaper window configuration — window level, borderless style, fullscreen sizing, system UI exclusion, and CLI flag

### Modified Capabilities
- `windowing`: Window creation gains a conditional wallpaper path (borderless, fullscreen, desktop level) alongside the existing 1024×1024 windowed path

## Impact

- **Code**: `src/main.rs` — CLI parsing extended with `--wallpaper`, window creation branched, raw `objc` calls to configure NSWindow properties not exposed by winit
- **Dependencies**: May need `objc2` or direct `objc` FFI for NSWindow level/style APIs (evaluate whether existing `objc` dep suffices)
- **Metal layer**: Drawable size changes from 1024×1024 to screen resolution — uniform buffer recalculation already handled by resize path
- **Grid rendering**: Grid dimensions adapt to screen aspect ratio in wallpaper mode (e.g., 410×256 on 16:10) — `GRID_WIDTH` / `GRID_HEIGHT` become runtime values, grid buffers are allocated accordingly, and the uniform buffer + compute dispatch use the actual dimensions. Fragment shader and compute kernel already read dimensions from the uniform buffer, so no shader changes are needed
- **No breaking changes**: Default behaviour (no `--wallpaper` flag) is identical to current

## UMA Relevance

Running as a desktop wallpaper proves the UMA pipeline can sustain a permanently-visible rendering surface at near-zero resource cost. The shared buffer architecture means desktop-resolution rendering adds no extra memory copies — the GPU reads the same 64 KB grid buffers regardless of output resolution. This validates UMA for always-on ambient workloads where power efficiency matters.
