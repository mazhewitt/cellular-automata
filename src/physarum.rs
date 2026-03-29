// Physarum slime mould simulation — CPU reference implementation.
// This module provides pure-Rust equivalents of the GPU compute kernels
// for use in testing and validation.

use std::f32::consts::PI;

/// Fixed parameters for the Physarum simulation.
/// Values match the compile-time constants in `physarum.metal`.
#[derive(Debug, Clone)]
pub struct PhysarumConfig {
    pub sensor_angle: f32,
    pub sensor_dist: f32,
    pub turn_speed: f32,
    pub move_speed: f32,
    pub deposit_amount: f32,
    pub decay_factor: f32,
    pub width: u32,
    pub height: u32,
    pub num_species: u32,
}

impl Default for PhysarumConfig {
    fn default() -> Self {
        Self {
            sensor_angle: PI / 4.0,     // 45 degrees
            sensor_dist: 9.0,           // pixels
            turn_speed: PI / 4.0,       // 45 degrees per step
            move_speed: 1.0,            // 1 pixel per step
            deposit_amount: 5.0,
            decay_factor: 0.95,
            width: 0,
            height: 0,
            num_species: 3,
        }
    }
}

impl PhysarumConfig {
    /// Total cells per species plane.
    pub fn plane_size(&self) -> usize {
        self.width as usize * self.height as usize
    }

    /// Total floats in one trail buffer (all species planes).
    pub fn trail_len(&self) -> usize {
        self.plane_size() * self.num_species as usize
    }
}

/// Toroidal wrap for float coordinates.
fn wrap(val: f32, max: f32) -> f32 {
    ((val % max) + max) % max
}

/// Sample the trail map at `(fx, fy)` for species `s` with toroidal wrap.
fn sample_trail(trail: &[f32], fx: f32, fy: f32, config: &PhysarumConfig) -> f32 {
    let w = config.width as f32;
    let h = config.height as f32;
    let x = wrap(fx, w) as u32;
    let y = wrap(fy, h) as u32;
    // Caller is responsible for offsetting into the correct species plane.
    trail[(y as usize) * (config.width as usize) + (x as usize)]
}

/// CPU reference implementation of the agent step kernel.
///
/// For each agent: sense 3 probes on its species trail plane, rotate, move
/// (toroidal wrap), and deposit trail in-place into `trail` (matching GPU
/// behaviour where agents deposit into the current trail buffer).
///
/// Agents are updated in-place.
pub fn cpu_agent_step(
    agents: &mut [[f32; 4]],
    trail: &mut [f32],
    config: &PhysarumConfig,
) {
    let w = config.width as f32;
    let h = config.height as f32;
    let plane = config.plane_size();

    // Two-pass to match GPU parallel execution: all agents sense from the
    // original trail before any deposits land.

    // Pass 1: sense + move (read-only access to trail).
    for agent in agents.iter_mut() {
        let x = agent[0];
        let y = agent[1];
        let heading = agent[2];
        let species = agent[3] as usize;

        // Species plane offset.
        let plane_offset = species * plane;
        let species_trail = &trail[plane_offset..plane_offset + plane];

        // --- Sense ---
        let left_angle = heading - config.sensor_angle;
        let right_angle = heading + config.sensor_angle;
        let d = config.sensor_dist;

        let probe_l = sample_trail(
            species_trail,
            x + d * left_angle.cos(),
            y + d * left_angle.sin(),
            config,
        );
        let probe_c = sample_trail(
            species_trail,
            x + d * heading.cos(),
            y + d * heading.sin(),
            config,
        );
        let probe_r = sample_trail(
            species_trail,
            x + d * right_angle.cos(),
            y + d * right_angle.sin(),
            config,
        );

        // --- Rotate ---
        let new_heading = if probe_c >= probe_l && probe_c >= probe_r {
            heading // centre is highest (or tied) — no turn
        } else if probe_l > probe_r {
            heading - config.turn_speed
        } else {
            heading + config.turn_speed
        };

        // --- Move ---
        let nx = wrap(x + config.move_speed * new_heading.cos(), w);
        let ny = wrap(y + config.move_speed * new_heading.sin(), h);

        agent[0] = nx;
        agent[1] = ny;
        agent[2] = new_heading;
    }

    // Pass 2: deposit (write to trail after all agents have moved).
    for agent in agents.iter() {
        let nx = agent[0];
        let ny = agent[1];
        let species = agent[3] as usize;

        let cx = nx as u32;
        let cy = ny as u32;
        let deposit_idx = species * plane + (cy as usize) * (config.width as usize) + (cx as usize);
        trail[deposit_idx] += config.deposit_amount;
    }
}

/// CPU reference implementation of the diffuse + decay kernel.
///
/// For each cell in each species plane: compute 3×3 box-blur mean (toroidal
/// wrap), multiply by `decay_factor`, write to `trail_dst`.
pub fn cpu_diffuse_decay(
    trail_src: &[f32],
    trail_dst: &mut [f32],
    config: &PhysarumConfig,
) {
    let w = config.width as usize;
    let h = config.height as usize;
    let plane = config.plane_size();

    for s in 0..config.num_species as usize {
        let offset = s * plane;
        for y in 0..h {
            for x in 0..w {
                let mut sum = 0.0f32;
                for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        let nx = ((x as i32 + dx).rem_euclid(w as i32)) as usize;
                        let ny = ((y as i32 + dy).rem_euclid(h as i32)) as usize;
                        sum += trail_src[offset + ny * w + nx];
                    }
                }
                trail_dst[offset + y * w + x] = (sum / 9.0) * config.decay_factor;
            }
        }
    }
}

/// Initialise agents with random positions, random headings, cycling species.
///
/// Uses a deterministic seed for reproducibility in tests.
pub fn init_agents(width: u32, height: u32, num_agents: usize, seed: u64) -> Vec<[f32; 4]> {
    use rand::rngs::StdRng;
    use rand::SeedableRng;
    use rand::Rng;

    let mut rng = StdRng::seed_from_u64(seed);
    let w = width as f32;
    let h = height as f32;
    let num_species = 3u32;

    (0..num_agents)
        .map(|i| {
            let x: f32 = rng.r#gen();
            let y: f32 = rng.r#gen();
            let heading: f32 = rng.r#gen();
            let species = (i % num_species as usize) as f32;
            [x * w, y * h, heading * 2.0 * PI, species]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = PhysarumConfig::default();
        assert_eq!(cfg.num_species, 3);
        assert!((cfg.sensor_angle - PI / 4.0).abs() < 1e-6);
        assert!((cfg.decay_factor - 0.95).abs() < 1e-6);
    }

    #[test]
    fn test_cpu_agent_step_single_agent() {
        let config = PhysarumConfig { width: 64, height: 64, ..PhysarumConfig::default() };

        let trail_len = config.trail_len();
        let mut trail = vec![0.0f32; trail_len]; // blank trail

        // Agent at centre, heading right (0 radians), species 0
        let mut agents = vec![[32.0_f32, 32.0, 0.0, 0.0]];

        cpu_agent_step(&mut agents, &mut trail, &config);

        let a = &agents[0];
        // With blank trail, centre == left == right (all 0), so heading unchanged (0).
        // Move: x += 1*cos(0)=1, y += 1*sin(0)=0  =>  (33, 32)
        assert!((a[0] - 33.0).abs() < 1e-4, "x={}", a[0]);
        assert!((a[1] - 32.0).abs() < 1e-4, "y={}", a[1]);
        assert!((a[2] - 0.0).abs() < 1e-4, "heading={}", a[2]);

        // Deposit: species 0 plane, cell (33,32)
        let deposit_idx = 32 * 64 + 33;
        assert!((trail[deposit_idx] - config.deposit_amount).abs() < 1e-4);
    }

    #[test]
    fn test_cpu_agent_step_turns_toward_trail() {
        let config = PhysarumConfig { width: 64, height: 64, ..PhysarumConfig::default() };

        let trail_len = config.trail_len();
        let mut trail = vec![0.0f32; trail_len];

        // Place trail to the left of agent's heading (agent heading = 0, left probe at -45°)
        // Probe position: (32 + 9*cos(-π/4), 32 + 9*sin(-π/4)) ≈ (38.36, 25.64)
        // Nearest cell: (38, 25) in species 0 plane
        let lx = (32.0 + 9.0 * (-PI / 4.0).cos()) as u32; // ~38
        let ly = (32.0 + 9.0 * (-PI / 4.0).sin()) as u32; // ~25
        trail[(ly as usize) * 64 + (lx as usize)] = 10.0;

        let mut agents = vec![[32.0_f32, 32.0, 0.0, 0.0]];

        cpu_agent_step(&mut agents, &mut trail, &config);

        // Should turn left (heading decreases by turn_speed)
        let expected_heading = 0.0 - config.turn_speed;
        assert!(
            (agents[0][2] - expected_heading).abs() < 1e-4,
            "heading={} expected={}",
            agents[0][2],
            expected_heading
        );
    }

    #[test]
    fn test_cpu_diffuse_decay_single_cell() {
        let config = PhysarumConfig { width: 8, height: 8, ..PhysarumConfig::default() };

        let trail_len = config.trail_len();
        let mut trail_src = vec![0.0f32; trail_len];
        let mut trail_dst = vec![0.0f32; trail_len];

        // Set a single cell to 9.0 in species 0 plane at (4, 4)
        trail_src[4 * 8 + 4] = 9.0;

        cpu_diffuse_decay(&trail_src, &mut trail_dst, &config);

        // 3×3 box blur: the source cell contributes 9.0 to each of its 9 neighbours'
        // blur sums. Each of those 9 cells gets mean = 9.0/9 = 1.0, then * 0.95 = 0.95.
        // The source cell itself also gets 0.95.
        for dy in -1i32..=1 {
            for dx in -1i32..=1 {
                let nx = (4 + dx) as usize;
                let ny = (4 + dy) as usize;
                let val = trail_dst[ny * 8 + nx];
                assert!(
                    (val - 0.95).abs() < 1e-4,
                    "cell ({},{}) = {} expected 0.95",
                    nx, ny, val
                );
            }
        }

        // A cell outside the 3×3 neighbourhood should be 0
        assert!((trail_dst[0]).abs() < 1e-6);

        // Species 1 and 2 planes should be untouched
        let plane = config.plane_size();
        assert!((trail_dst[plane]).abs() < 1e-6);
        assert!((trail_dst[2 * plane]).abs() < 1e-6);
    }

    #[test]
    fn test_init_agents() {
        let agents = init_agents(128, 128, 300, 42);
        assert_eq!(agents.len(), 300);

        // Species distribution: cycling 0,1,2,0,1,2,...
        let mut counts = [0u32; 3];
        for a in &agents {
            let s = a[3] as u32;
            assert!(s < 3, "invalid species {}", s);
            counts[s as usize] += 1;
        }
        assert_eq!(counts[0], 100);
        assert_eq!(counts[1], 100);
        assert_eq!(counts[2], 100);

        // All positions within bounds
        for a in &agents {
            assert!(a[0] >= 0.0 && a[0] < 128.0, "x={}", a[0]);
            assert!(a[1] >= 0.0 && a[1] < 128.0, "y={}", a[1]);
            assert!(a[2] >= 0.0 && a[2] < 2.0 * PI, "heading={}", a[2]);
        }

        // Deterministic: same seed gives same result
        let agents2 = init_agents(128, 128, 300, 42);
        assert_eq!(agents, agents2);
    }
}
