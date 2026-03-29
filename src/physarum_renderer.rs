// Physarum slime mould GPU renderer: agent step, diffuse/decay, and render pipelines.

use metal::{
    Buffer, ComputePipelineState, Device, Library,
    MetalLayerRef, MTLResourceOptions, RenderPipelineState,
};
use std::sync::atomic::Ordering;
use std::time::Instant;
use winit::event::{ElementState, Event, WindowEvent};
use winit::event_loop::{EventLoop, ControlFlow};
use winit::keyboard::{Key, NamedKey};
use winit::window::Window;
use core_graphics_types::geometry::CGSize;

use crate::app::{TICK_RATES, SIGTERM_RECEIVED};
use crate::game_of_life::GridConfig;
use crate::metal_context::{MetalContext, Uniforms};
use crate::physarum;
use crate::wallpaper;

const PHYSARUM_SHADER_SOURCE: &str = include_str!("shaders/physarum.metal");

fn compile_physarum_library(device: &Device) -> Result<Library, String> {
    let opts = metal::CompileOptions::new();
    device
        .new_library_with_source(PHYSARUM_SHADER_SOURCE, &opts)
        .map_err(|e| format!("Physarum shader compile error: {}", e))
}

pub struct PhysarumRenderer {
    ctx: MetalContext,
    _library: Library,
    agent_step_pipeline: ComputePipelineState,
    diffuse_decay_pipeline: ComputePipelineState,
    render_pipeline: RenderPipelineState,
    agent_buffer: Buffer,
    trail_buffers: [Buffer; 2],
    uniform_buffer: Buffer,
    num_agents_buffer: Buffer,
    current_trail: usize,
    num_agents: u32,
    width: u32,
    height: u32,
}

impl PhysarumRenderer {
    pub fn new(width: u32, height: u32, num_agents: u32) -> Result<Self, String> {
        let ctx = MetalContext::new()?;
        let library = compile_physarum_library(ctx.device())?;

        // Compute pipelines
        let agent_step_fn = library
            .get_function("agent_step", None)
            .map_err(|e| format!("Missing agent_step: {}", e))?;
        let agent_step_pipeline = ctx.device()
            .new_compute_pipeline_state_with_function(&agent_step_fn)
            .map_err(|e| format!("agent_step pipeline error: {}", e))?;

        let diffuse_decay_fn = library
            .get_function("diffuse_decay", None)
            .map_err(|e| format!("Missing diffuse_decay: {}", e))?;
        let diffuse_decay_pipeline = ctx.device()
            .new_compute_pipeline_state_with_function(&diffuse_decay_fn)
            .map_err(|e| format!("diffuse_decay pipeline error: {}", e))?;

        // Render pipeline
        let vertex_fn = library
            .get_function("fullscreen_quad_vertex", None)
            .map_err(|e| format!("Missing vertex function: {}", e))?;
        let fragment_fn = library
            .get_function("physarum_fragment", None)
            .map_err(|e| format!("Missing fragment function: {}", e))?;
        let desc = metal::RenderPipelineDescriptor::new();
        desc.set_vertex_function(Some(&vertex_fn));
        desc.set_fragment_function(Some(&fragment_fn));
        desc.color_attachments()
            .object_at(0)
            .unwrap()
            .set_pixel_format(metal::MTLPixelFormat::BGRA8Unorm);
        let render_pipeline = ctx.device()
            .new_render_pipeline_state(&desc)
            .map_err(|e| format!("Physarum render pipeline error: {}", e))?;

        // Buffers
        let device = ctx.device();
        let agent_buf_size = (num_agents as u64).max(1) * 4 * std::mem::size_of::<f32>() as u64;
        let agent_buffer = device.new_buffer(agent_buf_size, MTLResourceOptions::StorageModeShared);

        let plane_size = (width as u64) * (height as u64);
        let trail_buf_size = plane_size * 3 * std::mem::size_of::<f32>() as u64;
        let trail_a = device.new_buffer(trail_buf_size, MTLResourceOptions::StorageModeShared);
        let trail_b = device.new_buffer(trail_buf_size, MTLResourceOptions::StorageModeShared);
        unsafe {
            std::ptr::write_bytes(trail_a.contents() as *mut u8, 0, trail_buf_size as usize);
            std::ptr::write_bytes(trail_b.contents() as *mut u8, 0, trail_buf_size as usize);
        }

        let uniform_buffer = MetalContext::allocate_uniform_buffer(device, width, height);

        let num_agents_buffer = device.new_buffer(
            std::mem::size_of::<u32>() as u64,
            MTLResourceOptions::StorageModeShared,
        );
        unsafe {
            std::ptr::write(num_agents_buffer.contents() as *mut u32, num_agents);
        }

        Ok(PhysarumRenderer {
            ctx,
            _library: library,
            agent_step_pipeline,
            diffuse_decay_pipeline,
            render_pipeline,
            agent_buffer,
            trail_buffers: [trail_a, trail_b],
            uniform_buffer,
            num_agents_buffer,
            current_trail: 0,
            num_agents,
            width,
            height,
        })
    }

    pub fn device(&self) -> &Device {
        self.ctx.device()
    }

    pub fn current_trail(&self) -> usize {
        self.current_trail
    }

    /// Upload agent data from CPU slice.
    pub fn upload_agents(&self, agents: &[[f32; 4]]) {
        let byte_len = std::mem::size_of_val(agents);
        unsafe {
            std::ptr::copy_nonoverlapping(
                agents.as_ptr() as *const u8,
                self.agent_buffer.contents() as *mut u8,
                byte_len,
            );
        }
    }

    /// Get a mutable slice view of the agent buffer.
    #[allow(clippy::mut_from_ref)]
    pub fn agent_buffer_slice_mut(&self) -> &mut [[f32; 4]] {
        unsafe {
            std::slice::from_raw_parts_mut(
                self.agent_buffer.contents() as *mut [f32; 4],
                self.num_agents as usize,
            )
        }
    }

    /// Get a mutable slice view of trail buffer[index].
    #[allow(clippy::mut_from_ref)]
    pub fn trail_buffer_slice_mut(&self, index: usize) -> &mut [f32] {
        let plane_size = self.width as usize * self.height as usize;
        let trail_len = plane_size * 3;
        unsafe {
            std::slice::from_raw_parts_mut(
                self.trail_buffers[index].contents() as *mut f32,
                trail_len,
            )
        }
    }

    /// Update the uniform buffer with new cell dimensions after resize.
    pub fn update_uniforms(&self, drawable_width: f64, drawable_height: f64) {
        let uniforms = Uniforms {
            grid_width: self.width,
            grid_height: self.height,
            cell_width: (drawable_width / self.width as f64) as f32,
            cell_height: (drawable_height / self.height as f64) as f32,
        };
        unsafe {
            let ptr = self.uniform_buffer.contents() as *mut Uniforms;
            std::ptr::write(ptr, uniforms);
        }
    }

    /// Run compute passes only (agent_step + diffuse_decay + swap), without rendering.
    /// Used by GPU integration tests.
    pub fn compute_step(&mut self) {
        let cmd_buffer = self.ctx.command_queue().new_command_buffer();

        let src = self.current_trail;
        let dst = 1 - src;

        // --- Agent step: sense from src, deposit into src (in-place) ---
        {
            let encoder = cmd_buffer.new_compute_command_encoder();
            encoder.set_compute_pipeline_state(&self.agent_step_pipeline);
            encoder.set_buffer(0, Some(&self.agent_buffer), 0);
            encoder.set_buffer(1, Some(&self.trail_buffers[src]), 0);
            encoder.set_buffer(2, Some(&self.trail_buffers[src]), 0);
            encoder.set_buffer(3, Some(&self.uniform_buffer), 0);
            encoder.set_buffer(4, Some(&self.num_agents_buffer), 0);

            let threadgroup_size = metal::MTLSize::new(256, 1, 1);
            let grid_size = metal::MTLSize::new(self.num_agents as u64, 1, 1);
            encoder.dispatch_threads(grid_size, threadgroup_size);
            encoder.end_encoding();
        }

        // --- Diffuse + decay: read src (with deposits), write dst ---
        {
            let encoder = cmd_buffer.new_compute_command_encoder();
            encoder.set_compute_pipeline_state(&self.diffuse_decay_pipeline);
            encoder.set_buffer(0, Some(&self.trail_buffers[src]), 0);
            encoder.set_buffer(1, Some(&self.trail_buffers[dst]), 0);
            encoder.set_buffer(2, Some(&self.uniform_buffer), 0);

            let threadgroup_size = metal::MTLSize::new(16, 16, 1);
            let grid_size = metal::MTLSize::new(
                self.width as u64,
                self.height as u64,
                1,
            );
            encoder.dispatch_threads(grid_size, threadgroup_size);
            encoder.end_encoding();
        }

        cmd_buffer.commit();
        cmd_buffer.wait_until_completed();

        self.current_trail = dst;
    }

    /// Run one Physarum frame: agent_step, diffuse_decay, swap trail, render.
    pub fn render_frame(&mut self, layer: &MetalLayerRef, step: bool) {
        let Some(drawable) = layer.next_drawable() else { return };
        let cmd_buffer = self.ctx.command_queue().new_command_buffer();

        let src = self.current_trail;
        let dst = 1 - src;

        if step {
            // --- Agent step: sense from src, deposit into src (in-place) ---
            {
                let encoder = cmd_buffer.new_compute_command_encoder();
                encoder.set_compute_pipeline_state(&self.agent_step_pipeline);
                encoder.set_buffer(0, Some(&self.agent_buffer), 0);
                encoder.set_buffer(1, Some(&self.trail_buffers[src]), 0);
                encoder.set_buffer(2, Some(&self.trail_buffers[src]), 0);
                encoder.set_buffer(3, Some(&self.uniform_buffer), 0);
                encoder.set_buffer(4, Some(&self.num_agents_buffer), 0);

                let threadgroup_size = metal::MTLSize::new(256, 1, 1);
                let grid_size = metal::MTLSize::new(self.num_agents as u64, 1, 1);
                encoder.dispatch_threads(grid_size, threadgroup_size);
                encoder.end_encoding();
            }

            // --- Diffuse + decay: read src (with deposits), write dst ---
            {
                let encoder = cmd_buffer.new_compute_command_encoder();
                encoder.set_compute_pipeline_state(&self.diffuse_decay_pipeline);
                encoder.set_buffer(0, Some(&self.trail_buffers[src]), 0);
                encoder.set_buffer(1, Some(&self.trail_buffers[dst]), 0);
                encoder.set_buffer(2, Some(&self.uniform_buffer), 0);

                let threadgroup_size = metal::MTLSize::new(16, 16, 1);
                let grid_size = metal::MTLSize::new(
                    self.width as u64,
                    self.height as u64,
                    1,
                );
                encoder.dispatch_threads(grid_size, threadgroup_size);
                encoder.end_encoding();
            }
        }

        // --- Render ---
        let render_trail = if step { dst } else { src };
        {
            let pass_desc = metal::RenderPassDescriptor::new();
            let attachment = pass_desc.color_attachments().object_at(0).expect("color attachment");
            attachment.set_texture(Some(drawable.texture()));
            attachment.set_load_action(metal::MTLLoadAction::Clear);
            attachment.set_clear_color(metal::MTLClearColor::new(0.0, 0.0, 0.0, 1.0));
            attachment.set_store_action(metal::MTLStoreAction::Store);

            let encoder = cmd_buffer.new_render_command_encoder(pass_desc);
            encoder.set_render_pipeline_state(&self.render_pipeline);
            encoder.set_fragment_buffer(0, Some(&self.trail_buffers[render_trail]), 0);
            encoder.set_fragment_buffer(1, Some(&self.uniform_buffer), 0);
            encoder.draw_primitives(metal::MTLPrimitiveType::Triangle, 0, 6);
            encoder.end_encoding();
        }

        cmd_buffer.present_drawable(drawable);
        cmd_buffer.commit();

        if step {
            self.current_trail = dst;
        }
    }
}

// ── Physarum event loop ────────────────────────────────────────────────

fn sync_drawable_size(window: &Window, layer: &MetalLayerRef, renderer: &PhysarumRenderer) {
    let size = window.inner_size();
    layer.set_drawable_size(CGSize::new(size.width as f64, size.height as f64));
    renderer.update_uniforms(size.width as f64, size.height as f64);
}

/// Run the Physarum slime mould event loop.
pub fn run(is_wallpaper: bool, window: Window, event_loop: EventLoop<()>) {
    let (width, height) = if is_wallpaper {
        let (sw, sh) = wallpaper::main_screen_size();
        let gc = GridConfig::for_screen(sw, sh);
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
    sync_drawable_size(&window, metal_layer, &renderer);

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
                        sync_drawable_size(&window, metal_layer, &renderer);
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
