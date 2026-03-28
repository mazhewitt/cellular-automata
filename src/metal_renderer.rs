// Metal rendering context: device, command queue, pipelines, and buffers.

use metal::{
    Buffer, CommandQueue, ComputePipelineState, Device, Library,
    MetalLayerRef, MTLPixelFormat, MTLResourceOptions, RenderPipelineState,
};
use metal::foreign_types::ForeignType;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

use crate::grid::GridConfig;

/// Must match the Uniforms struct in game_of_life.metal and physarum.metal.
#[repr(C)]
pub struct Uniforms {
    pub grid_width: u32,
    pub grid_height: u32,
    pub cell_width: f32,
    pub cell_height: f32,
}

// ── Shared Metal context ───────────────────────────────────────────────

/// Shared Metal device + command queue used by all renderers.
pub struct MetalContext {
    device: Device,
    command_queue: CommandQueue,
}

impl MetalContext {
    pub fn new() -> Result<Self, String> {
        let device = Device::system_default().ok_or_else(|| {
            "No Metal-capable GPU available. Metal is required on macOS/Apple Silicon.".to_string()
        })?;
        let command_queue = device.new_command_queue();
        Ok(MetalContext { device, command_queue })
    }

    pub fn device(&self) -> &Device {
        &self.device
    }

    pub fn command_queue(&self) -> &CommandQueue {
        &self.command_queue
    }

    /// Create and configure a Metal layer on the given window.
    pub fn setup_metal_layer<'a>(window: &Window, device: &Device) -> &'a MetalLayerRef {
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

        layer.set_device(device);
        layer.set_pixel_format(MTLPixelFormat::BGRA8Unorm);
        layer.set_framebuffer_only(true);

        let raw = layer.as_ptr();
        std::mem::forget(layer);
        unsafe { &*(raw as *const MetalLayerRef) }
    }

    /// Allocate a uniform buffer with initial grid/cell dimensions.
    pub fn allocate_uniform_buffer(device: &Device, width: u32, height: u32) -> Buffer {
        let buffer = device.new_buffer(
            std::mem::size_of::<Uniforms>() as u64,
            MTLResourceOptions::StorageModeShared,
        );
        let initial = Uniforms {
            grid_width: width,
            grid_height: height,
            cell_width: 1.0,
            cell_height: 1.0,
        };
        unsafe {
            std::ptr::write(buffer.contents() as *mut Uniforms, initial);
        }
        buffer
    }
}

pub struct MetalRenderer {
    ctx: MetalContext,
    _library: Library,
    compute_pipeline: ComputePipelineState,
    render_pipeline: RenderPipelineState,
    grid_buffers: [Buffer; 2],
    uniform_buffer: Buffer,
    current_buffer: usize,
    grid_config: GridConfig,
}

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

fn allocate_uniform_buffer_gol(device: &Device, grid_config: &GridConfig) -> Buffer {
    MetalContext::allocate_uniform_buffer(device, grid_config.width as u32, grid_config.height as u32)
}

impl MetalRenderer {
    pub fn new(grid_config: GridConfig) -> Result<Self, String> {
        let ctx = MetalContext::new()?;
        let library = compile_shader_library(ctx.device())?;
        let compute_pipeline = create_compute_pipeline(ctx.device(), &library)?;
        let render_pipeline = create_render_pipeline(ctx.device(), &library)?;
        let grid_buffers = allocate_grid_buffers(ctx.device(), &grid_config);
        let uniform_buffer = allocate_uniform_buffer_gol(ctx.device(), &grid_config);

        Ok(MetalRenderer {
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

    pub fn command_queue(&self) -> &CommandQueue {
        self.ctx.command_queue()
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

    /// Create and configure a Metal layer on the given window.
    pub fn setup_metal_layer<'a>(window: &Window, device: &Device) -> &'a MetalLayerRef {
        MetalContext::setup_metal_layer(window, device)
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

// ── Physarum Renderer ──────────────────────────────────────────────────

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
        let agent_buf_size = (num_agents as u64) * 4 * std::mem::size_of::<f32>() as u64;
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

    /// Create and configure a Metal layer on the given window.
    pub fn setup_metal_layer<'a>(window: &Window, device: &Device) -> &'a MetalLayerRef {
        MetalContext::setup_metal_layer(window, device)
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
                encoder.set_buffer(1, Some(&self.trail_buffers[src]), 0); // sense
                encoder.set_buffer(2, Some(&self.trail_buffers[src]), 0); // deposit (same buffer)
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

