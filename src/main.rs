mod grid;
mod metal_renderer;
mod wallpaper;

use metal::foreign_types::ForeignType;
use metal::{CommandBufferRef, MetalLayerRef, MTLPixelFormat};
use metal_renderer::MetalRenderer;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
use rand::Rng;
use winit::event::{ElementState, Event, WindowEvent};
use winit::event_loop::{EventLoop, ControlFlow};
use winit::keyboard::{Key, NamedKey};
use winit::window::Window;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use core_graphics_types::geometry::CGSize;

const TICK_RATES: &[u64] = &[1, 2, 5, 10, 20, 30, 60, 120];

static SIGTERM_RECEIVED: AtomicBool = AtomicBool::new(false);

extern "C" fn sigterm_handler(_sig: libc::c_int) {
    SIGTERM_RECEIVED.store(true, Ordering::Relaxed);
}

pub struct AppConfig {
    pub seed: String,
    pub wallpaper: bool,
}

fn parse_args() -> AppConfig {
    let args: Vec<String> = std::env::args().collect();
    let mut seed = "r-pentomino".to_string();
    let mut wallpaper = false;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--wallpaper" => {
                wallpaper = true;
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
    AppConfig { seed, wallpaper }
}

fn seed_grid(renderer: &MetalRenderer, seed_name: &str) {
    let buf = renderer.grid_buffer_slice_mut(0);
    let gc = &renderer.grid_config;
    let cx = gc.width / 2;
    let cy = gc.height / 2;
    match seed_name {
        "blinker" => grid::seed_blinker(buf, cx, cy),
        "glider" => grid::seed_glider(buf, cx, cy),
        _ => grid::seed_r_pentomino(buf, cx, cy),
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

    let grid_config = if config.wallpaper {
        let (sw, sh) = wallpaper::main_screen_size();
        grid::GridConfig::for_screen(sw, sh)
    } else {
        grid::GridConfig::default()
    };

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

    let mut renderer = MetalRenderer::new(grid_config).expect("Failed to initialize Metal renderer");
    seed_grid(&renderer, &config.seed);

    let metal_layer = setup_metal_layer(&window, &renderer);
    sync_drawable_size(&window, metal_layer, &renderer);

    let mut tick_index: usize = 3; // start at 10 steps/sec
    let mut last_step = Instant::now();
    let mut redraw_pending = false;
    let mut rng = rand::thread_rng();
    let mut next_spawn = Instant::now()
        + std::time::Duration::from_secs(rng.gen_range(10..=30));

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
                        if Instant::now() >= next_spawn {
                            let gc = &renderer.grid_config;
                            let cx = rng.gen_range(0..gc.width);
                            let cy = rng.gen_range(0..gc.height);
                            let rotation = rng.gen_range(0..4);
                            let buf = renderer.grid_buffer_slice_mut(renderer.current_buffer);
                            grid::spawn_glider_wh(buf, cx, cy, rotation, gc.width, gc.height);
                            next_spawn = Instant::now()
                                + std::time::Duration::from_secs(rng.gen_range(10..=30));
                        }
                        render_frame(&mut renderer, metal_layer, true);
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

fn setup_metal_layer<'a>(window: &Window, renderer: &MetalRenderer) -> &'a MetalLayerRef {
    let ns_view_ptr = match window
        .window_handle()
        .expect("window handle")
        .as_raw()
    {
        RawWindowHandle::AppKit(h) => h.ns_view,
        _ => panic!("expected AppKit window handle on macOS"),
    };

    let rwm_layer = unsafe { raw_window_metal::Layer::from_ns_view(ns_view_ptr) };
    let layer_ptr = rwm_layer.into_raw();
    let layer = unsafe {
        metal::MetalLayer::from_ptr(layer_ptr.as_ptr() as *mut metal::CAMetalLayer)
    };

    layer.set_device(renderer.device());
    layer.set_pixel_format(MTLPixelFormat::BGRA8Unorm);
    layer.set_framebuffer_only(true);

    let raw = layer.as_ptr();
    std::mem::forget(layer);
    unsafe { &*(raw as *const MetalLayerRef) }
}

fn render_frame(renderer: &mut MetalRenderer, layer: &MetalLayerRef, step: bool) {
    let Some(drawable) = layer.next_drawable() else { return };
    let cmd_buffer = renderer.command_queue().new_command_buffer();

    let cur = renderer.current_buffer;
    let nxt = 1 - cur;

    if step {
        encode_compute_pass(cmd_buffer, renderer, cur, nxt);
        encode_render_pass(cmd_buffer, renderer, drawable.texture(), nxt);
    } else {
        encode_render_pass(cmd_buffer, renderer, drawable.texture(), cur);
    }

    cmd_buffer.present_drawable(drawable);
    cmd_buffer.commit();

    if step {
        renderer.current_buffer = nxt;
    }
}

fn encode_compute_pass(
    cmd_buffer: &CommandBufferRef,
    renderer: &MetalRenderer,
    read_idx: usize,
    write_idx: usize,
) {
    let encoder = cmd_buffer.new_compute_command_encoder();
    encoder.set_compute_pipeline_state(&renderer.compute_pipeline);
    encoder.set_buffer(0, Some(&renderer.grid_buffers[read_idx]), 0);
    encoder.set_buffer(1, Some(&renderer.grid_buffers[write_idx]), 0);
    encoder.set_buffer(2, Some(&renderer.uniform_buffer), 0);

    let threadgroup_size = metal::MTLSize::new(16, 16, 1);
    let grid_size = metal::MTLSize::new(
        renderer.grid_config.width as u64,
        renderer.grid_config.height as u64,
        1,
    );
    encoder.dispatch_threads(grid_size, threadgroup_size);
    encoder.end_encoding();
}

fn encode_render_pass(
    cmd_buffer: &CommandBufferRef,
    renderer: &MetalRenderer,
    target_texture: &metal::TextureRef,
    grid_idx: usize,
) {
    let pass_desc = metal::RenderPassDescriptor::new();
    let attachment = pass_desc.color_attachments().object_at(0).expect("color attachment 0");
    attachment.set_texture(Some(target_texture));
    attachment.set_load_action(metal::MTLLoadAction::Clear);
    attachment.set_clear_color(metal::MTLClearColor::new(0.0, 0.0, 0.0, 1.0));
    attachment.set_store_action(metal::MTLStoreAction::Store);

    let encoder = cmd_buffer.new_render_command_encoder(pass_desc);
    encoder.set_render_pipeline_state(&renderer.render_pipeline);
    encoder.set_fragment_buffer(0, Some(&renderer.grid_buffers[grid_idx]), 0);
    encoder.set_fragment_buffer(1, Some(&renderer.uniform_buffer), 0);
    encoder.draw_primitives(metal::MTLPrimitiveType::Triangle, 0, 6);
    encoder.end_encoding();
}
