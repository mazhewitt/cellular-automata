mod app;
mod game_of_life;
mod gol_renderer;
mod metal_context;
mod physarum;
mod physarum_renderer;
mod wallpaper;

use app::SIGTERM_RECEIVED;
use std::sync::atomic::Ordering;
use winit::event_loop::EventLoop;
use winit::window::Window;

extern "C" fn sigterm_handler(_sig: libc::c_int) {
    SIGTERM_RECEIVED.store(true, Ordering::Relaxed);
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum SimMode {
    GameOfLife,
    Physarum,
}

struct AppConfig {
    seed: String,
    wallpaper: bool,
    mode: SimMode,
}

fn parse_args() -> AppConfig {
    let args: Vec<String> = std::env::args().collect();
    let mut seed = "r-pentomino".to_string();
    let mut wallpaper = false;
    let mut mode = SimMode::GameOfLife;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--wallpaper" => {
                wallpaper = true;
            }
            "--mode" => {
                if i + 1 < args.len() {
                    match args[i + 1].as_str() {
                        "gol" => mode = SimMode::GameOfLife,
                        "physarum" => mode = SimMode::Physarum,
                        other => {
                            eprintln!("Unknown mode '{}'. Available: gol, physarum", other);
                            std::process::exit(1);
                        }
                    }
                    i += 1;
                } else {
                    eprintln!("--mode requires a value. Available: gol, physarum");
                    std::process::exit(1);
                }
            }
            "--seed" => {
                if i + 1 < args.len() {
                    let name = args[i + 1].clone();
                    match name.as_str() {
                        "blinker" | "glider" | "r-pentomino" => seed = name,
                        _ => {
                            eprintln!("Unknown seed '{}'. Available: blinker, glider, r-pentomino", name);
                            std::process::exit(1);
                        }
                    }
                    i += 1;
                } else {
                    eprintln!("--seed requires a value. Available: blinker, glider, r-pentomino");
                    std::process::exit(1);
                }
            }
            _ => {}
        }
        i += 1;
    }
    AppConfig { seed, wallpaper, mode }
}

fn main() {
    let config = parse_args();

    // Register SIGTERM handler for daemon/launchd use.
    unsafe { libc::signal(libc::SIGTERM, sigterm_handler as *const () as libc::sighandler_t) };

    let event_loop = EventLoop::new().expect("Failed to create event loop");

    #[allow(deprecated)]
    let window = event_loop
        .create_window(
            Window::default_attributes()
                .with_inner_size(winit::dpi::LogicalSize::new(1024.0_f64, 1024.0_f64))
                .with_title("Game of Life — Unified Memory"),
        )
        .expect("Failed to create window");

    if config.wallpaper {
        wallpaper::configure_wallpaper(&window);
    }

    match config.mode {
        SimMode::GameOfLife => gol_renderer::run(&config.seed, config.wallpaper, window, event_loop),
        SimMode::Physarum => physarum_renderer::run(config.wallpaper, window, event_loop),
    }
}
