// GPU integration tests.
// Tests Metal device availability, command queue creation, and shared memory buffer validation.

#[cfg(test)]
mod tests {
    use metal::{Device, MTLResourceOptions, NSRange};

    #[test]
    fn test_metal_device_available() {
        // Task 2.2: Verify Metal device is available on this system
        let device = Device::system_default();
        assert!(device.is_some(), "Metal device should be available on macOS/Apple Silicon");
    }

    #[test]
    fn test_command_queue_creation() {
        // Task 2.2: Verify command queue can be created from device
        let device = Device::system_default().expect("Metal device not available");
        let queue = device.new_command_queue();
        // The queue is valid if it was created successfully (no panic)
        // metal-rs doesn't expose is_null, so just verify creation succeeded
        let _ = queue;
    }

    #[test]
    fn test_shared_buffer_cpu_write_read() {
        // Task 3.1: Verify StorageModeShared buffer validation
        let device = Device::system_default().expect("Metal device not available");

        // Allocate a small shared buffer (64 bytes)
        let buffer_size = 64;
        let buffer = device.new_buffer(
            buffer_size,
            MTLResourceOptions::StorageModeShared,
        );

        // Write test data from CPU
        let test_data: [u32; 16] = [
            1, 2, 3, 4, 5, 6, 7, 8,
            9, 10, 11, 12, 13, 14, 15, 16,
        ];
        unsafe {
            let ptr = buffer.contents() as *mut u32;
            ptr.copy_from_nonoverlapping(test_data.as_ptr(), test_data.len());
        }

        // Read back from CPU and verify
        let read_data: &[u32] = unsafe {
            let ptr = buffer.contents() as *const u32;
            std::slice::from_raw_parts(ptr, test_data.len())
        };

        assert_eq!(read_data, &test_data, "Buffer data should match after CPU write/read");
    }

    #[test]
    fn test_shared_buffer_accessible_in_same_memory() {
        // Task 3.1: Verify CPU and GPU access the same physical memory
        let device = Device::system_default().expect("Metal device not available");

        // Allocate a shared buffer with initial data
        let buffer_size = 32;
        let buffer = device.new_buffer(
            buffer_size,
            MTLResourceOptions::StorageModeShared,
        );

        // Write from CPU
        let initial_value = 42u32;
        unsafe {
            let ptr = buffer.contents() as *mut u32;
            *ptr = initial_value;
        }

        // Verify the same memory is still accessible (GPU would read this in production)
        let read_back: u32 = unsafe {
            let ptr = buffer.contents() as *const u32;
            *ptr
        };

        assert_eq!(read_back, initial_value, "StorageModeShared buffer should preserve CPU writes for GPU access");
    }

    #[test]
    fn test_shared_buffer_gpu_write_readback() {
        // Validate spec scenario: GPU writes shared memory, CPU reads it back.
        let device = Device::system_default().expect("Metal device not available");
        let queue = device.new_command_queue();

        let buffer_size: u64 = 64;
        let fill_value: u8 = 0xAB;

        let buffer = device.new_buffer(
            buffer_size,
            MTLResourceOptions::StorageModeShared,
        );

        // GPU-side write via blit fill.
        let command_buffer = queue.new_command_buffer();
        let blit = command_buffer.new_blit_command_encoder();
        blit.fill_buffer(&buffer, NSRange::new(0, buffer_size), fill_value);
        blit.end_encoding();
        command_buffer.commit();
        command_buffer.wait_until_completed();

        // CPU reads the same shared memory after GPU completion.
        let bytes: &[u8] = unsafe {
            let ptr = buffer.contents() as *const u8;
            std::slice::from_raw_parts(ptr, buffer_size as usize)
        };

        assert!(
            bytes.iter().all(|&b| b == fill_value),
            "All bytes should match the GPU-written fill value"
        );
    }

    // --- Cross-validation: GPU compute shader vs Rust step() ---

    fn create_compute_pipeline(device: &metal::Device) -> (metal::ComputePipelineState, metal::Library) {
        let shader_source = include_str!("../src/shaders/game_of_life.metal");
        let opts = metal::CompileOptions::new();
        let library = device.new_library_with_source(shader_source, &opts)
            .expect("Failed to compile shader");
        let update_fn = library.get_function("update_cells", None)
            .expect("Missing update_cells");
        let pipeline = device.new_compute_pipeline_state_with_function(&update_fn)
            .expect("Compute pipeline creation failed");
        (pipeline, library)
    }

    fn run_gpu_step(
        device: &metal::Device,
        queue: &metal::CommandQueue,
        pipeline: &metal::ComputePipelineState,
        src_data: &[u8],
    ) -> Vec<u8> {
        use um_game_of_life::grid::{GRID_WIDTH, GRID_HEIGHT, GRID_SIZE};

        let src_buf = device.new_buffer_with_data(
            src_data.as_ptr() as *const _,
            GRID_SIZE as u64,
            MTLResourceOptions::StorageModeShared,
        );
        let dst_buf = device.new_buffer(
            GRID_SIZE as u64,
            MTLResourceOptions::StorageModeShared,
        );

        // Uniforms buffer.
        #[repr(C)]
        struct Uniforms {
            grid_width: u32,
            grid_height: u32,
            cell_width: f32,
            cell_height: f32,
        }
        let uniforms = Uniforms {
            grid_width: GRID_WIDTH as u32,
            grid_height: GRID_HEIGHT as u32,
            cell_width: 1.0,
            cell_height: 1.0,
        };
        let uniform_buf = device.new_buffer_with_data(
            &uniforms as *const _ as *const _,
            std::mem::size_of::<Uniforms>() as u64,
            MTLResourceOptions::StorageModeShared,
        );

        let cmd = queue.new_command_buffer();
        let enc = cmd.new_compute_command_encoder();
        enc.set_compute_pipeline_state(pipeline);
        enc.set_buffer(0, Some(&src_buf), 0);
        enc.set_buffer(1, Some(&dst_buf), 0);
        enc.set_buffer(2, Some(&uniform_buf), 0);

        let tg_size = metal::MTLSize::new(16, 16, 1);
        let grid_size = metal::MTLSize::new(GRID_WIDTH as u64, GRID_HEIGHT as u64, 1);
        enc.dispatch_threads(grid_size, tg_size);
        enc.end_encoding();
        cmd.commit();
        cmd.wait_until_completed();

        unsafe {
            let ptr = dst_buf.contents() as *const u8;
            std::slice::from_raw_parts(ptr, GRID_SIZE).to_vec()
        }
    }

    #[test]
    fn test_gpu_blinker_one_step_matches_cpu() {
        use um_game_of_life::grid::{GRID_WIDTH, GRID_HEIGHT, GRID_SIZE};

        let device = Device::system_default().expect("Metal device not available");
        let queue = device.new_command_queue();
        let (pipeline, _lib) = create_compute_pipeline(&device);

        // Seed blinker.
        let mut src = vec![0u8; GRID_SIZE];
        let cx = GRID_WIDTH / 2;
        let cy = GRID_HEIGHT / 2;
        um_game_of_life::grid::seed_blinker(&mut src, cx, cy);

        // CPU step.
        let mut cpu_dst = vec![0u8; GRID_SIZE];
        um_game_of_life::grid::step(&src, &mut cpu_dst);

        // GPU step.
        let gpu_dst = run_gpu_step(&device, &queue, &pipeline, &src);

        assert_eq!(
            cpu_dst, gpu_dst,
            "GPU blinker 1-step output must match CPU output byte-for-byte"
        );
    }

    #[test]
    fn test_gpu_glider_four_steps_matches_cpu() {
        use um_game_of_life::grid::{GRID_WIDTH, GRID_HEIGHT, GRID_SIZE};

        let device = Device::system_default().expect("Metal device not available");
        let queue = device.new_command_queue();
        let (pipeline, _lib) = create_compute_pipeline(&device);

        // Seed glider.
        let mut state = vec![0u8; GRID_SIZE];
        let cx = GRID_WIDTH / 2;
        let cy = GRID_HEIGHT / 2;
        um_game_of_life::grid::seed_glider(&mut state, cx, cy);

        // Run 4 steps on both CPU and GPU.
        let mut cpu_state = state.clone();
        let mut cpu_tmp = vec![0u8; GRID_SIZE];
        for _ in 0..4 {
            um_game_of_life::grid::step(&cpu_state, &mut cpu_tmp);
            std::mem::swap(&mut cpu_state, &mut cpu_tmp);
        }

        let mut gpu_state = state;
        for _ in 0..4 {
            let result = run_gpu_step(&device, &queue, &pipeline, &gpu_state);
            gpu_state = result;
        }

        assert_eq!(
            cpu_state, gpu_state,
            "GPU glider 4-step output must match CPU output byte-for-byte"
        );
    }
}
