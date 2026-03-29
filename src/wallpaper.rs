// macOS wallpaper mode: configure NSWindow to render behind all windows at desktop level.

use objc2_foundation::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{
    NSScreen, NSView, NSWindowCollectionBehavior, NSWindowStyleMask,
};
use objc2_foundation::NSRect;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

unsafe extern "C" {
    fn CGWindowLevelForKey(key: i32) -> i32;
}
const K_CG_DESKTOP_WINDOW_LEVEL_KEY: i32 = 2;

/// Returns (screen_width, screen_height) in points for the main screen.
pub fn main_screen_size() -> (f64, f64) {
    let mtm = MainThreadMarker::new()
        .expect("main_screen_size must be called from the main thread");
    let screen = NSScreen::mainScreen(mtm)
        .expect("no main screen available");
    let frame = screen.frame();
    (frame.size.width, frame.size.height)
}

/// Configure the window as a desktop wallpaper layer.
pub fn configure_wallpaper(window: &Window) {
    let ns_view = get_ns_view(window);
    let ns_window = ns_view
        .window()
        .expect("NSView has no parent NSWindow");

    let desktop_level = unsafe { CGWindowLevelForKey(K_CG_DESKTOP_WINDOW_LEVEL_KEY) };

    // Render behind all other windows.
    ns_window.setLevel(desktop_level as _);

    // Borderless — no title bar, no chrome.
    ns_window.setStyleMask(NSWindowStyleMask::Borderless);

    // Appear on all Spaces, stay stationary, invisible to Exposé/Cmd-Tab.
    ns_window.setCollectionBehavior(
        NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::Stationary
            | NSWindowCollectionBehavior::IgnoresCycle,
    );

    // No shadow for a wallpaper.
    ns_window.setHasShadow(false);

    // Clicks pass through to Finder / desktop icons.
    ns_window.setIgnoresMouseEvents(true);

    // Fill the main screen.
    let mtm = MainThreadMarker::new().unwrap();
    let screen = NSScreen::mainScreen(mtm).expect("no main screen");
    let screen_frame: NSRect = screen.frame();
    ns_window.setFrame_display(screen_frame, true);
}

fn get_ns_view(window: &Window) -> Retained<NSView> {
    let ns_view_ptr = match window
        .window_handle()
        .expect("window handle")
        .as_raw()
    {
        RawWindowHandle::AppKit(h) => h.ns_view,
        _ => panic!("expected AppKit window handle on macOS"),
    };

    // Safety: The pointer comes from winit's valid AppKit window handle.
    unsafe { Retained::retain(ns_view_ptr.as_ptr() as *mut NSView) }
        .expect("NSView pointer is null")
}
