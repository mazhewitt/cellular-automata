// Game of Life GPU renderer: compute + render pipelines for the GoL simulation.

use metal::{
    Buffer, ComputePipelineState, Device, Library,
    MetalLayerRef, MTLResourceOptions, RenderPipelineState,
};
use rand::RngExt;
use std::sync::atomic::Ordering;
use std::time::Instant;
use winit::event::{ElementState, Event, WindowEvent};
use winit::event_loop::{EventLoop, ControlFlow};
use winit::keyboard::{Key, NamedKey};
use winit::window::Window;
use core_graphics_types::geometry::CGSize;

use crate::app::{TICK_RATES, SIGTERM_RECEIVED};
use crate::game_of_life::{self, GridConfig};
use crate::metal_context::{MetalContext, Uniforms};
use crate::wallpaper;

const SHADER_SOURCE: &str = include_str!("shaders/game_of_life.metal");

fn compile_shader_library(device: &Device) -> Result<Library, String> {
    let opts = metal::CompileOptions::new();
    device
        .new_library_with_source(SHADER_SOURCE, &opts)
        .map_err(|e| format!("Shader compile error: {}", e))
}

fn create_compute_pipeline(
    device: &Device,
    library: &Library,
) -> Result<ComputePipelineState, String> {
    let update_fn = library
        .get_function("update_cells", None)
        .map_err(|e| format!("Missing update_cells function: {}", e))?;
    device
        .new_compute_pipeline_state_with_function(&update_fn)
        .map_err(|e| format!("Compute pipeline error: {}", e))
}

fn create_render_pipeline(
    device: &Device,
    library: &Library,
) -> Result<RenderPipelineState, String> {
    let vertex_fn = library
        .get_function("fullscreen_quad_vertex", None)
        .map_err(|e| format!("Missing vertex function: {}", e))?;
    let fragment_fn = library
        .get_function("grid_fragment", None)
        .map_err(|e| format!("Missing fragment function: {}", e))?;

    let desc = metal::RenderPipelineDescriptor::new();
    desc.set_vertex_function(Some(&vertex_fn));
    desc.set_fragment_function(Some(&fragment_fn));
    desc.color_attachments()
        .object_at(0)
        .unwrap()
        .set_pixel_format(metal::MTLPixelFormat::BGRA8Unorm);

    device
        .new_render_pipeline_state(&desc)
        .map_err(|e| format!("Render pipeline error: {}", e))
}

fn allocate_grid_buffers(device: &Device, grid_config: &GridConfig) -> [Buffer; 2] {
    let buf_size = grid_config.size() as u64;
    let a = device.new_buffer(buf_size, MTLResourceOptions::StorageModeShared);
    let b = device.new_buffer(buf_size, MTLResourceOptions::StorageModeShared);
    unsafe {
        std::ptr::write_bytes(a.contents() as *mut u8, 0, grid_config.size());
        std::ptr::write_bytes(b.contents() as *mut u8, 0, grid_config.size());
    }
    [a, b]
}

pub struct GolRenderer {
    ctx: MetalContext,
    _library: Library,
    compute_pipeline: ComputePipelineState,
    render_pipeline: RenderPipelineState,
    grid_buffers: [Buffer; 2],
    uniform_buffer: Buffer,
    current_buffer: usize,
    grid_config: GridConfig,
}

impl GolRenderer {
    pub fn new(grid_config: GridConfig) -> Result<Self, String> {
        let ctx = MetalContext::new()?;
        let library = compile_shader_library(ctx.device())?;
        let compute_pipeline = create_compute_pipeline(ctx.device(), &library)?;
        let render_pipeline = create_render_pipeline(ctx.device(), &library)?;
        let grid_buffers = allocate_grid_buffers(ctx.device(), &grid_config);
        let uniform_buffer = MetalContext::allocate_uniform_buffer(
            ctx.device(),
            grid_config.width as u32,
            grid_config.height as u32,
        );

        Ok(GolRenderer {
            ctx,
            _library: library,
            compute_pipeline,
            render_pipeline,
            grid_buffers,
            uniform_buffer,
            current_buffer: 0,
            grid_config,
        })
    }

    pub fn device(&self) -> &Device {
        self.ctx.device()
    }

    pub fn grid_config(&self) -> &GridConfig {
        &self.grid_config
    }

    pub fn current_buffer(&self) -> usize {
        self.current_buffer
    }

    /// Update the uniform buffer with new cell dimensions after resize.
    pub fn update_uniforms(&self, drawable_width: f64, drawable_height: f64) {
        let uniforms = Uniforms {
            grid_width: self.grid_config.width as u32,
            grid_height: self.grid_config.height as u32,
            cell_width: (drawable_width / self.grid_config.width as f64) as f32,
            cell_height: (drawable_height / self.grid_config.height as f64) as f32,
        };
        unsafe {
            let ptr = self.uniform_buffer.contents() as *mut Uniforms;
            std::ptr::write(ptr, uniforms);
        }
    }

    /// Get a mutable slice view of grid buffer[index] for CPU seeding.
    /// Safety: Metal's StorageModeShared buffers provide a stable CPU-visible pointer.
    #[allow(clippy::mut_from_ref)]
    pub fn grid_buffer_slice_mut(&self, index: usize) -> &mut [u8] {
        unsafe {
            std::slice::from_raw_parts_mut(
                self.grid_buffers[index].contents() as *mut u8,
                self.grid_config.size(),
            )
        }
    }

    /// Encode a GoL compute pass: read from grid_buffers[read_idx], write to grid_buffers[write_idx].
    pub fn encode_compute_pass(
        &self,
        cmd_buffer: &metal::CommandBufferRef,
        read_idx: usize,
        write_idx: usize,
    ) {
        let encoder = cmd_buffer.new_compute_command_encoder();
        encoder.set_compute_pipeline_state(&self.compute_pipeline);
        encoder.set_buffer(0, Some(&self.grid_buffers[read_idx]), 0);
        encoder.set_buffer(1, Some(&self.grid_buffers[write_idx]), 0);
        encoder.set_buffer(2, Some(&self.uniform_buffer), 0);

        let threadgroup_size = metal::MTLSize::new(16, 16, 1);
        let grid_size = metal::MTLSize::new(
            self.grid_config.width as u64,
            self.grid_config.height as u64,
            1,
        );
        encoder.dispatch_threads(grid_size, threadgroup_size);
        encoder.end_encoding();
    }

    /// Encode a GoL render pass: draw grid_buffers[grid_idx] to the target texture.
    pub fn encode_render_pass(
        &self,
        cmd_buffer: &metal::CommandBufferRef,
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
        encoder.set_render_pipeline_state(&self.render_pipeline);
        encoder.set_fragment_buffer(0, Some(&self.grid_buffers[grid_idx]), 0);
        encoder.set_fragment_buffer(1, Some(&self.uniform_buffer), 0);
        encoder.draw_primitives(metal::MTLPrimitiveType::Triangle, 0, 6);
        encoder.end_encoding();
    }

    /// Run one GoL frame: optionally step (compute + render + swap), or just render current state.
    pub fn render_frame(&mut self, layer: &MetalLayerRef, step: bool) {
        let Some(drawable) = layer.next_drawable() else { return };
        let cmd_buffer = self.ctx.command_queue().new_command_buffer();

        let cur = self.current_buffer;
        let nxt = 1 - cur;

        if step {
            self.encode_compute_pass(cmd_buffer, cur, nxt);
            self.encode_render_pass(cmd_buffer, drawable.texture(), nxt);
        } else {
            self.encode_render_pass(cmd_buffer, drawable.texture(), cur);
        }

        cmd_buffer.present_drawable(drawable);
        cmd_buffer.commit();

        if step {
            self.current_buffer = nxt;
        }
    }
}

// ── GoL application state ──────────────────────────────────────────────

struct GoLState {
    rng: rand::rngs::ThreadRng,
    next_spawn: Instant,
}

impl GoLState {
    fn new() -> Self {
        let mut rng = rand::rng();
        let next_spawn = Instant::now()
            + std::time::Duration::from_secs(rng.random_range(10..=30));
        GoLState { rng, next_spawn }
    }

    fn seed_grid(renderer: &GolRenderer, seed_name: &str) {
        let buf = renderer.grid_buffer_slice_mut(0);
        let gc = renderer.grid_config();
        let cx = gc.width / 2;
        let cy = gc.height / 2;
        match seed_name {
            "blinker" => game_of_life::seed_blinker(buf, cx, cy, gc.width, gc.height),
            "glider" => game_of_life::seed_glider(buf, cx, cy, gc.width, gc.height),
            _ => game_of_life::seed_r_pentomino(buf, cx, cy, gc.width, gc.height),
        }
    }

    fn maybe_spawn_glider(&mut self, renderer: &GolRenderer) {
        if Instant::now() >= self.next_spawn {
            let gc = renderer.grid_config();
            let cx = self.rng.random_range(0..gc.width);
            let cy = self.rng.random_range(0..gc.height);
            let rotation = self.rng.random_range(0..4);
            let buf = renderer.grid_buffer_slice_mut(renderer.current_buffer());
            game_of_life::spawn_glider(buf, cx, cy, rotation, gc.width, gc.height);
            self.next_spawn = Instant::now()
                + std::time::Duration::from_secs(self.rng.random_range(10..=30));
        }
    }
}

fn sync_drawable_size(window: &Window, layer: &MetalLayerRef, renderer: &GolRenderer) {
    let size = window.inner_size();
    layer.set_drawable_size(CGSize::new(size.width as f64, size.height as f64));
    renderer.update_uniforms(size.width as f64, size.height as f64);
}

/// Run the Game of Life event loop.
pub fn run(seed: &str, is_wallpaper: bool, window: Window, event_loop: EventLoop<()>) {
    let grid_config = if is_wallpaper {
        let (sw, sh) = wallpaper::main_screen_size();
        GridConfig::for_screen(sw, sh)
    } else {
        GridConfig::default()
    };

    let mut renderer = GolRenderer::new(grid_config).expect("Failed to initialize Metal renderer");
    GoLState::seed_grid(&renderer, seed);

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
