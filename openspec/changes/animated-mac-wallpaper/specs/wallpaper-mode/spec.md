## ADDED Requirements

### Requirement: Wallpaper CLI flag
The system SHALL accept a `--wallpaper` command-line flag that activates desktop wallpaper mode. When the flag is absent, the application SHALL behave identically to the current windowed mode.

#### Scenario: Wallpaper mode activated
- **WHEN** the application starts with `--wallpaper`
- **THEN** the window is configured as a desktop wallpaper layer

#### Scenario: Default windowed mode preserved
- **WHEN** the application starts without `--wallpaper`
- **THEN** the window appears as the standard 1024×1024 resizable window with title bar

### Requirement: Desktop window level
The system SHALL set the NSWindow level to `kCGDesktopWindowLevel` in wallpaper mode so the window renders behind all other windows, directly on the desktop.

#### Scenario: Window renders behind all applications
- **WHEN** wallpaper mode is active and other windows are open
- **THEN** the Game of Life window is visible only where no other window overlaps, appearing as the desktop background

#### Scenario: Window remains behind newly opened windows
- **WHEN** wallpaper mode is active and the user opens a new application
- **THEN** the new application's windows appear above the wallpaper window

### Requirement: Borderless fullscreen
The system SHALL configure the NSWindow with `NSWindowStyleMaskBorderless` in wallpaper mode and size it to fill the main screen's frame.

#### Scenario: Window fills the entire screen
- **WHEN** wallpaper mode is active
- **THEN** the window frame matches `[NSScreen mainScreen].frame` with no title bar, borders, or rounded corners

#### Scenario: No window shadow
- **WHEN** wallpaper mode is active
- **THEN** the window has no shadow (`setHasShadow: NO`)

### Requirement: System UI exclusion
The system SHALL configure the NSWindow collection behavior to appear on all Spaces, remain stationary, and be excluded from Exposé, Mission Control, and the App Switcher.

#### Scenario: Window appears on all Spaces
- **WHEN** wallpaper mode is active and the user switches to a different Space
- **THEN** the wallpaper window is visible on every Space

#### Scenario: Window excluded from Mission Control
- **WHEN** wallpaper mode is active and the user activates Mission Control
- **THEN** the wallpaper window does not appear as a selectable window

#### Scenario: Window excluded from Cmd-Tab
- **WHEN** wallpaper mode is active and the user presses Cmd-Tab
- **THEN** the application does not appear in the App Switcher

### Requirement: Mouse passthrough
The system SHALL set `ignoresMouseEvents` to `YES` on the NSWindow in wallpaper mode so mouse clicks pass through to the Finder and desktop icons.

#### Scenario: Clicks reach desktop icons
- **WHEN** wallpaper mode is active and the user clicks on a desktop icon
- **THEN** the click passes through the wallpaper window to the Finder

#### Scenario: Right-click shows Finder context menu
- **WHEN** wallpaper mode is active and the user right-clicks on the desktop
- **THEN** the Finder context menu appears as normal

### Requirement: Aspect-ratio grid sizing
The system SHALL compute grid dimensions at runtime in wallpaper mode so that cells render as perfect squares. The grid height SHALL remain 256, and the grid width SHALL be `round(256 × screen_width / screen_height)`.

#### Scenario: 16:10 display produces wider grid
- **WHEN** wallpaper mode is active on a 2560×1600 display
- **THEN** the grid is 410×256 and each cell occupies the same physical width and height on screen

#### Scenario: 16:9 display produces wider grid
- **WHEN** wallpaper mode is active on a 2560×1440 display
- **THEN** the grid is 455×256 and cells are square

#### Scenario: Windowed mode retains default grid
- **WHEN** the application starts without `--wallpaper`
- **THEN** the grid dimensions remain the compile-time defaults (256×256)

### Requirement: Wallpaper termination
The system SHALL exit cleanly in wallpaper mode when the Escape key is pressed or when SIGTERM is received.

#### Scenario: Escape key exits wallpaper
- **WHEN** wallpaper mode is active and the user presses Escape
- **THEN** the application exits cleanly

#### Scenario: SIGTERM exits wallpaper
- **WHEN** wallpaper mode is active and the process receives SIGTERM
- **THEN** the application exits cleanly without hanging
