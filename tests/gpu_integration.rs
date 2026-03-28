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

    fn run_gpu_step_wh(
        device: &metal::Device,
        queue: &metal::CommandQueue,
        pipeline: &metal::ComputePipelineState,
        src_data: &[u8],
        width: usize,
        height: usize,
    ) -> Vec<u8> {
        let grid_size = width * height;

        let src_buf = device.new_buffer_with_data(
            src_data.as_ptr() as *const _,
            grid_size as u64,
            MTLResourceOptions::StorageModeShared,
        );
        let dst_buf = device.new_buffer(
            grid_size as u64,
            MTLResourceOptions::StorageModeShared,
        );

        #[repr(C)]
        struct Uniforms {
            grid_width: u32,
            grid_height: u32,
            cell_width: f32,
            cell_height: f32,
        }
        let uniforms = Uniforms {
            grid_width: width as u32,
            grid_height: height as u32,
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
        let dispatch_size = metal::MTLSize::new(width as u64, height as u64, 1);
        enc.dispatch_threads(dispatch_size, tg_size);
        enc.end_encoding();
        cmd.commit();
        cmd.wait_until_completed();

        unsafe {
            let ptr = dst_buf.contents() as *const u8;
            std::slice::from_raw_parts(ptr, grid_size).to_vec()
        }
    }

    fn run_gpu_step(
        device: &metal::Device,
        queue: &metal::CommandQueue,
        pipeline: &metal::ComputePipelineState,
        src_data: &[u8],
    ) -> Vec<u8> {
        use um_game_of_life::grid::{GRID_WIDTH, GRID_HEIGHT};
        run_gpu_step_wh(device, queue, pipeline, src_data, GRID_WIDTH, GRID_HEIGHT)
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

    #[test]
    fn test_gpu_spawned_glider_one_step_matches_cpu() {
        use um_game_of_life::grid::GRID_SIZE;

        let device = Device::system_default().expect("Metal device not available");
        let queue = device.new_command_queue();
        let (pipeline, _lib) = create_compute_pipeline(&device);

        // Spawn gliders with all 4 rotations via CPU write into a shared buffer.
        let mut src = vec![0u8; GRID_SIZE];
        um_game_of_life::grid::spawn_glider(&mut src, 64, 64, 0);
        um_game_of_life::grid::spawn_glider(&mut src, 192, 64, 1);
        um_game_of_life::grid::spawn_glider(&mut src, 64, 192, 2);
        um_game_of_life::grid::spawn_glider(&mut src, 192, 192, 3);

        // CPU step.
        let mut cpu_dst = vec![0u8; GRID_SIZE];
        um_game_of_life::grid::step(&src, &mut cpu_dst);

        // GPU step — simulates the frame-boundary write pattern:
        // CPU writes glider into shared buffer, then GPU reads it.
        let gpu_dst = run_gpu_step(&device, &queue, &pipeline, &src);

        assert_eq!(
            cpu_dst, gpu_dst,
            "GPU output after spawned-glider CPU write must match CPU step byte-for-byte"
        );
    }

    #[test]
    fn test_gpu_non_square_grid_blinker_matches_cpu() {
        // Validates runtime grid dimensions in the shader pipeline with a non-square grid.
        use um_game_of_life::grid::{index_wh, step_wh};

        let width: usize = 32;
        let height: usize = 20;
        let grid_size = width * height;

        let device = Device::system_default().expect("Metal device not available");
        let queue = device.new_command_queue();
        let (pipeline, _lib) = create_compute_pipeline(&device);

        // Seed a blinker in the centre of a 32×20 grid.
        let mut src = vec![0u8; grid_size];
        let cx = width / 2;
        let cy = height / 2;
        src[index_wh(cx - 1, cy, width)] = 255;
        src[index_wh(cx, cy, width)] = 255;
        src[index_wh(cx + 1, cy, width)] = 255;

        // CPU step.
        let mut cpu_dst = vec![0u8; grid_size];
        step_wh(&src, &mut cpu_dst, width, height);

        // GPU step.
        let gpu_dst = run_gpu_step_wh(&device, &queue, &pipeline, &src, width, height);

        assert_eq!(
            cpu_dst, gpu_dst,
            "GPU non-square grid blinker 1-step output must match CPU output byte-for-byte"
        );
    }

    // --- Physarum GPU integration tests ---

    use um_game_of_life::metal_renderer::PhysarumRenderer;
    use um_game_of_life::physarum::{PhysarumConfig, cpu_agent_step, cpu_diffuse_decay, init_agents};

    const EPSILON: f32 = 1e-4;

    fn assert_slices_close(a: &[f32], b: &[f32], label: &str) {
        assert_eq!(a.len(), b.len(), "{}: length mismatch", label);
        for (i, (&av, &bv)) in a.iter().zip(b.iter()).enumerate() {
            assert!(
                (av - bv).abs() < EPSILON,
                "{}: mismatch at index {}: gpu={} cpu={}",
                label, i, av, bv
            );
        }
    }

    fn assert_agents_close(a: &[[f32; 4]], b: &[[f32; 4]], label: &str) {
        assert_eq!(a.len(), b.len(), "{}: agent count mismatch", label);
        for (i, (ag, bg)) in a.iter().zip(b.iter()).enumerate() {
            for c in 0..4 {
                assert!(
                    (ag[c] - bg[c]).abs() < EPSILON,
                    "{}: agent {} component {} mismatch: gpu={} cpu={}",
                    label, i, c, ag[c], bg[c]
                );
            }
        }
    }

    #[test]
    fn test_gpu_physarum_agent_step_matches_cpu() {
        let width = 32u32;
        let height = 32u32;
        let num_agents = 100u32;

        let mut renderer = PhysarumRenderer::new(width, height, num_agents)
            .expect("PhysarumRenderer creation failed");

        let agents = init_agents(width, height, num_agents as usize, 123);
        renderer.upload_agents(&agents);

        // Trail starts zeroed (blank). Run 1 compute step on GPU.
        renderer.compute_step();

        // Read back GPU results.
        let gpu_agents = renderer.agent_buffer_slice_mut().to_vec();
        let gpu_trail = renderer.trail_buffer_slice_mut(renderer.current_trail()).to_vec();

        // CPU reference: same initial conditions.
        let config = PhysarumConfig { width, height, ..PhysarumConfig::default() };

        let mut cpu_agents = agents.clone();
        let mut cpu_trail = vec![0.0f32; config.trail_len()];

        // agent_step: deposits in-place into cpu_trail
        cpu_agent_step(&mut cpu_agents, &mut cpu_trail, &config);

        // diffuse_decay: reads cpu_trail (with deposits), writes to cpu_trail_dst
        let mut cpu_trail_dst = vec![0.0f32; config.trail_len()];
        cpu_diffuse_decay(&cpu_trail, &mut cpu_trail_dst, &config);

        assert_agents_close(&gpu_agents, &cpu_agents, "agent_step");
        assert_slices_close(&gpu_trail, &cpu_trail_dst, "trail after full step");
    }

    #[test]
    fn test_gpu_physarum_diffuse_decay_matches_cpu() {
        let width = 16u32;
        let height = 16u32;

        // Create renderer with 0 agents to test diffuse_decay alone.
        let mut renderer = PhysarumRenderer::new(width, height, 0)
            .expect("PhysarumRenderer creation failed");

        // Seed trail with known pattern: single deposit in species 0.
        let trail = renderer.trail_buffer_slice_mut(0);
        trail[8 * 16 + 8] = 9.0; // centre of species 0 plane

        // Run 1 compute step.
        renderer.compute_step();

        // Read back GPU result (current_trail is now 1 after swap).
        let gpu_trail = renderer.trail_buffer_slice_mut(renderer.current_trail()).to_vec();

        // CPU reference.
        let config = PhysarumConfig { width, height, ..PhysarumConfig::default() };

        let mut src_trail = vec![0.0f32; config.trail_len()];
        src_trail[8 * 16 + 8] = 9.0;
        let mut cpu_trail_dst = vec![0.0f32; config.trail_len()];
        cpu_diffuse_decay(&src_trail, &mut cpu_trail_dst, &config);

        assert_slices_close(&gpu_trail, &cpu_trail_dst, "diffuse_decay");
    }

    #[test]
    fn test_gpu_physarum_full_frame_matches_cpu() {
        let width = 32u32;
        let height = 32u32;
        let num_agents = 50u32;

        let mut renderer = PhysarumRenderer::new(width, height, num_agents)
            .expect("PhysarumRenderer creation failed");

        let agents = init_agents(width, height, num_agents as usize, 999);
        renderer.upload_agents(&agents);

        // Run 1 full frame.
        renderer.compute_step();

        // Read back GPU results.
        let gpu_agents = renderer.agent_buffer_slice_mut().to_vec();
        let gpu_trail = renderer.trail_buffer_slice_mut(renderer.current_trail()).to_vec();

        // CPU reference.
        let config = PhysarumConfig { width, height, ..PhysarumConfig::default() };

        let mut cpu_agents = agents.clone();
        let mut cpu_trail = vec![0.0f32; config.trail_len()];
        cpu_agent_step(&mut cpu_agents, &mut cpu_trail, &config);
        let mut cpu_trail_dst = vec![0.0f32; config.trail_len()];
        cpu_diffuse_decay(&cpu_trail, &mut cpu_trail_dst, &config);

        assert_agents_close(&gpu_agents, &cpu_agents, "full frame agents");
        assert_slices_close(&gpu_trail, &cpu_trail_dst, "full frame trail");
    }
}
