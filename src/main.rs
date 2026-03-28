mod grid;
mod metal_renderer;
mod physarum;
mod wallpaper;

use metal::MetalLayerRef;
use metal_renderer::{MetalContext, MetalRenderer, PhysarumRenderer};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
use rand::Rng;
use winit::event::{ElementState, Event, WindowEvent};
use winit::event_loop::{EventLoop, ControlFlow};
use winit::keyboard::{Key, NamedKey};
use winit::window::Window;
use core_graphics_types::geometry::CGSize;

const TICK_RATES: &[u64] = &[1, 2, 5, 10, 20, 30, 60, 120];

static SIGTERM_RECEIVED: AtomicBool = AtomicBool::new(false);

extern "C" fn sigterm_handler(_sig: libc::c_int) {
    SIGTERM_RECEIVED.store(true, Ordering::Relaxed);
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SimMode {
    GameOfLife,
    Physarum,
}

pub struct AppConfig {
    pub seed: String,
    pub wallpaper: bool,
    pub mode: SimMode,
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

struct GoLState {
    rng: rand::rngs::ThreadRng,
    next_spawn: Instant,
}

impl GoLState {
    fn new() -> Self {
        let mut rng = rand::thread_rng();
        let next_spawn = Instant::now()
            + std::time::Duration::from_secs(rng.gen_range(10..=30));
        GoLState { rng, next_spawn }
    }

    fn seed_grid(renderer: &MetalRenderer, seed_name: &str) {
        let buf = renderer.grid_buffer_slice_mut(0);
        let gc = renderer.grid_config();
        let cx = gc.width / 2;
        let cy = gc.height / 2;
        match seed_name {
            "blinker" => grid::seed_blinker(buf, cx, cy),
            "glider" => grid::seed_glider(buf, cx, cy),
            _ => grid::seed_r_pentomino(buf, cx, cy),
        }
    }

    fn maybe_spawn_glider(&mut self, renderer: &MetalRenderer) {
        if Instant::now() >= self.next_spawn {
            let gc = renderer.grid_config();
            let cx = self.rng.gen_range(0..gc.width);
            let cy = self.rng.gen_range(0..gc.height);
            let rotation = self.rng.gen_range(0..4);
            let buf = renderer.grid_buffer_slice_mut(renderer.current_buffer());
            grid::spawn_glider_wh(buf, cx, cy, rotation, gc.width, gc.height);
            self.next_spawn = Instant::now()
                + std::time::Duration::from_secs(self.rng.gen_range(10..=30));
        }
    }
}

fn sync_drawable_size(window: &Window, layer: &MetalLayerRef, renderer: &MetalRenderer) {
    let size = window.inner_size();
    layer.set_drawable_size(CGSize::new(size.width as f64, size.height as f64));
    renderer.update_uniforms(size.width as f64, size.height as f64);
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
        SimMode::GameOfLife => run_gol(config, window, event_loop),
        SimMode::Physarum => run_physarum(config, window, event_loop),
    }
}

fn run_gol(config: AppConfig, window: Window, event_loop: EventLoop<()>) {
    let grid_config = if config.wallpaper {
        let (sw, sh) = wallpaper::main_screen_size();
        grid::GridConfig::for_screen(sw, sh)
    } else {
        grid::GridConfig::default()
    };

    let mut renderer = MetalRenderer::new(grid_config).expect("Failed to initialize Metal renderer");
    GoLState::seed_grid(&renderer, &config.seed);

    let metal_layer = MetalContext::setup_metal_layer(&window, renderer.device());
    sync_drawable_size(&window, metal_layer, &renderer);

    let mut tick_index: usize = 3; // start at 10 steps/sec
    let mut last_step = Instant::now();
    let mut redraw_pending = false;
    let mut gol_state = GoLState::new();

    #[allow(deprecated)]
    let _ = event_loop.run(move |event, window_target| {
        let tick_duration =
            std::time::Duration::from_micros(1_000_000 / TICK_RATES[tick_index]);

        match event {
            Event::WindowEvent { event, window_id } if window_id == window.id() => {
                match event {
                    WindowEvent::CloseRequested => window_target.exit(),
                    WindowEvent::KeyboardInput { event, .. }
                        if event.state == ElementState::Pressed =>
                    {
                        match event.logical_key {
                            Key::Named(NamedKey::Escape) => window_target.exit(),
                            Key::Named(NamedKey::ArrowUp) => {
                                if tick_index + 1 < TICK_RATES.len() {
                                    tick_index += 1;
                                }
                                eprintln!("Speed: {} steps/sec", TICK_RATES[tick_index]);
                            }
                            Key::Named(NamedKey::ArrowDown) => {
                                tick_index = tick_index.saturating_sub(1);
                                eprintln!("Speed: {} steps/sec", TICK_RATES[tick_index]);
                            }
                            _ => {}
                        }
                    }
                    WindowEvent::Resized(_) => {
                        sync_drawable_size(&window, metal_layer, &renderer);
                    }
                    WindowEvent::RedrawRequested => {
                        redraw_pending = false;
                        gol_state.maybe_spawn_glider(&renderer);
                        renderer.render_frame(metal_layer, true);
                        last_step = Instant::now();
                    }
                    _ => {}
                }
            }
            Event::AboutToWait => {
                if SIGTERM_RECEIVED.load(Ordering::Relaxed) {
                    window_target.exit();
                    return;
                }
                if !redraw_pending && last_step.elapsed() >= tick_duration {
                    redraw_pending = true;
                    window.request_redraw();
                }
            }
            _ => {}
        }
        window_target.set_control_flow(
            ControlFlow::WaitUntil(last_step + tick_duration),
        );
    });
}

fn sync_physarum_drawable_size(window: &Window, layer: &MetalLayerRef, renderer: &PhysarumRenderer) {
    let size = window.inner_size();
    layer.set_drawable_size(CGSize::new(size.width as f64, size.height as f64));
    renderer.update_uniforms(size.width as f64, size.height as f64);
}

fn run_physarum(config: AppConfig, window: Window, event_loop: EventLoop<()>) {
    let (width, height) = if config.wallpaper {
        let (sw, sh) = wallpaper::main_screen_size();
        let gc = grid::GridConfig::for_screen(sw, sh);
        (gc.width as u32, gc.height as u32)
    } else {
        (256, 256)
    };

    let num_agents = ((width as f64) * (height as f64) * 0.3) as u32;
    let mut renderer = PhysarumRenderer::new(width, height, num_agents)
        .expect("Failed to initialize Physarum renderer");

    // Initialise agents
    let agents = physarum::init_agents(width, height, num_agents as usize, 42);
    renderer.upload_agents(&agents);

    let metal_layer = MetalContext::setup_metal_layer(&window, renderer.device());
    sync_physarum_drawable_size(&window, metal_layer, &renderer);

    let mut tick_index: usize = 5; // start at 30 steps/sec
    let mut last_step = Instant::now();
    let mut redraw_pending = false;

    #[allow(deprecated)]
    let _ = event_loop.run(move |event, window_target| {
        let tick_duration =
            std::time::Duration::from_micros(1_000_000 / TICK_RATES[tick_index]);

        match event {
            Event::WindowEvent { event, window_id } if window_id == window.id() => {
                match event {
                    WindowEvent::CloseRequested => window_target.exit(),
                    WindowEvent::KeyboardInput { event, .. }
                        if event.state == ElementState::Pressed =>
                    {
                        match event.logical_key {
                            Key::Named(NamedKey::Escape) => window_target.exit(),
                            Key::Named(NamedKey::ArrowUp) => {
                                if tick_index + 1 < TICK_RATES.len() {
                                    tick_index += 1;
                                }
                                eprintln!("Speed: {} steps/sec", TICK_RATES[tick_index]);
                            }
                            Key::Named(NamedKey::ArrowDown) => {
                                tick_index = tick_index.saturating_sub(1);
                                eprintln!("Speed: {} steps/sec", TICK_RATES[tick_index]);
                            }
                            _ => {}
                        }
                    }
                    WindowEvent::Resized(_) => {
                        sync_physarum_drawable_size(&window, metal_layer, &renderer);
                    }
                    WindowEvent::RedrawRequested => {
                        redraw_pending = false;
                        renderer.render_frame(metal_layer, true);
                        last_step = Instant::now();
                    }
                    _ => {}
                }
            }
            Event::AboutToWait => {
                if SIGTERM_RECEIVED.load(Ordering::Relaxed) {
                    window_target.exit();
                    return;
                }
                if !redraw_pending && last_step.elapsed() >= tick_duration {
                    redraw_pending = true;
                    window.request_redraw();
                }
            }
            _ => {}
        }
        window_target.set_control_flow(
            ControlFlow::WaitUntil(last_step + tick_duration),
        );
    });
}
