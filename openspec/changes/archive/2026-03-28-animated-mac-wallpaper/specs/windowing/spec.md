## MODIFIED Requirements

### Requirement: Window creation
The system SHALL create a macOS window whose initial configuration depends on the launch mode:
- **Default (windowed):** 1024×1024 logical pixels with the title "Game of Life — Unified Memory"
- **Wallpaper (`--wallpaper`):** Borderless fullscreen at `kCGDesktopWindowLevel`, sized to `[NSScreen mainScreen].frame`

#### Scenario: Window appears on launch (windowed)
- **WHEN** the application starts without `--wallpaper`
- **THEN** a 1024×1024 window appears on screen with the correct title

#### Scenario: Window is resizable (windowed)
- **WHEN** the window is created in windowed mode
- **THEN** the user can resize the window by dragging its edges

#### Scenario: Window fills screen at desktop level (wallpaper)
- **WHEN** the application starts with `--wallpaper`
- **THEN** a borderless window fills the main screen at `kCGDesktopWindowLevel`, behind all other windows
