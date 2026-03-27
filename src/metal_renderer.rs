// Metal rendering context: device, command queue, pipelines, and buffers.

use metal::{
    Buffer, CommandQueue, ComputePipelineState, Device, Library,
    MTLResourceOptions, RenderPipelineState,
};

use crate::grid::{GRID_HEIGHT, GRID_SIZE, GRID_WIDTH};

/// Must match the Uniforms struct in game_of_life.metal.
#[repr(C)]
pub struct Uniforms {
    pub grid_width: u32,
    pub grid_height: u32,
    pub cell_width: f32,
    pub cell_height: f32,
}

pub struct MetalRenderer {
    device: Device,
    command_queue: CommandQueue,
    _library: Library,
    pub compute_pipeline: ComputePipelineState,
    pub render_pipeline: RenderPipelineState,
    pub grid_buffers: [Buffer; 2],
    pub uniform_buffer: Buffer,
    pub current_buffer: usize,
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

fn allocate_grid_buffers(device: &Device) -> [Buffer; 2] {
    let buf_size = GRID_SIZE as u64;
    let a = device.new_buffer(buf_size, MTLResourceOptions::StorageModeShared);
    let b = device.new_buffer(buf_size, MTLResourceOptions::StorageModeShared);
    unsafe {
        std::ptr::write_bytes(a.contents() as *mut u8, 0, GRID_SIZE);
        std::ptr::write_bytes(b.contents() as *mut u8, 0, GRID_SIZE);
    }
    [a, b]
}

fn allocate_uniform_buffer(device: &Device) -> Buffer {
    let buffer = device.new_buffer(
        std::mem::size_of::<Uniforms>() as u64,
        MTLResourceOptions::StorageModeShared,
    );
    let initial = Uniforms {
        grid_width: GRID_WIDTH as u32,
        grid_height: GRID_HEIGHT as u32,
        cell_width: 1.0,
        cell_height: 1.0,
    };
    unsafe {
        std::ptr::write(buffer.contents() as *mut Uniforms, initial);
    }
    buffer
}

impl MetalRenderer {
    pub fn new() -> Result<Self, String> {
        let device = Device::system_default().ok_or_else(|| {
            "No Metal-capable GPU available. Metal is required on macOS/Apple Silicon.".to_string()
        })?;
        let command_queue = device.new_command_queue();
        let library = compile_shader_library(&device)?;
        let compute_pipeline = create_compute_pipeline(&device, &library)?;
        let render_pipeline = create_render_pipeline(&device, &library)?;
        let grid_buffers = allocate_grid_buffers(&device);
        let uniform_buffer = allocate_uniform_buffer(&device);

        Ok(MetalRenderer {
            device,
            command_queue,
            _library: library,
            compute_pipeline,
            render_pipeline,
            grid_buffers,
            uniform_buffer,
            current_buffer: 0,
        })
    }

    pub fn device(&self) -> &Device {
        &self.device
    }

    pub fn command_queue(&self) -> &CommandQueue {
        &self.command_queue
    }

    /// Update the uniform buffer with new cell dimensions after resize.
    pub fn update_uniforms(&self, drawable_width: f64, drawable_height: f64) {
        let uniforms = Uniforms {
            grid_width: GRID_WIDTH as u32,
            grid_height: GRID_HEIGHT as u32,
            cell_width: (drawable_width / GRID_WIDTH as f64) as f32,
            cell_height: (drawable_height / GRID_HEIGHT as f64) as f32,
        };
        unsafe {
            let ptr = self.uniform_buffer.contents() as *mut Uniforms;
            std::ptr::write(ptr, uniforms);
        }
    }

    /// Get a mutable slice view of grid buffer[0] for CPU seeding.
    pub fn grid_buffer_slice_mut(&self, index: usize) -> &mut [u8] {
        unsafe {
            std::slice::from_raw_parts_mut(
                self.grid_buffers[index].contents() as *mut u8,
                GRID_SIZE,
            )
        }
    }
}

