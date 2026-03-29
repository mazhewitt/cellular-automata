// Shared Metal device, command queue, layer setup, and uniform buffer allocation.

use metal::{
    Buffer, CommandQueue, Device, MetalLayerRef, MTLPixelFormat, MTLResourceOptions,
};
use metal::foreign_types::ForeignType;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

/// Must match the Uniforms struct in game_of_life.metal and physarum.metal.
#[repr(C)]
pub struct Uniforms {
    pub grid_width: u32,
    pub grid_height: u32,
    pub cell_width: f32,
    pub cell_height: f32,
}

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
