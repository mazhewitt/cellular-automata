use um_game_of_life::game_of_life::*;

const W: usize = 256;
const H: usize = 256;
const SZ: usize = W * H;

fn empty_grid() -> Vec<u8> {
    vec![0u8; SZ]
}

fn empty_grid_wh(width: usize, height: usize) -> Vec<u8> {
    vec![0u8; width * height]
}

fn set_alive(grid: &mut [u8], cells: &[(usize, usize)]) {
    for &(x, y) in cells {
        grid[index(x, y, W)] = ALIVE;
    }
}

fn alive_cells(grid: &[u8]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    for y in 0..H {
        for x in 0..W {
            if grid[index(x, y, W)] == ALIVE {
                out.push((x, y));
            }
        }
    }
    out
}

// --- Still life: block ---

#[test]
fn test_block_still_life() {
    let mut src = empty_grid();
    // 2x2 block at (10,10)
    set_alive(&mut src, &[(10, 10), (11, 10), (10, 11), (11, 11)]);
    let mut dst = empty_grid();
    step(&src, &mut dst, W, H);
    assert_eq!(alive_cells(&dst), alive_cells(&src), "Block should be unchanged after one step");
}

// --- Oscillator: blinker period 2 ---

#[test]
fn test_blinker_oscillator() {
    let mut src = empty_grid();
    // Horizontal blinker at (10,10)
    set_alive(&mut src, &[(9, 10), (10, 10), (11, 10)]);

    let mut gen1 = empty_grid();
    step(&src, &mut gen1, W, H);
    // Should become vertical
    let expected_v: Vec<(usize, usize)> = vec![(10, 9), (10, 10), (10, 11)];
    assert_eq!(alive_cells(&gen1), expected_v, "Blinker gen1 should be vertical");

    let mut gen2 = empty_grid();
    step(&gen1, &mut gen2, W, H);
    // Should return to horizontal
    assert_eq!(alive_cells(&gen2), alive_cells(&src), "Blinker gen2 should match initial");
}

// --- Spaceship: glider 4-step movement ---

#[test]
fn test_glider_4_steps() {
    let mut grid = empty_grid();
    // Standard glider at (10,10):
    //  .X.
    //  ..X
    //  XXX
    set_alive(&mut grid, &[(11, 10), (12, 11), (10, 12), (11, 12), (12, 12)]);

    let initial = alive_cells(&grid);

    let mut buf_a = grid;
    let mut buf_b = empty_grid();
    for i in 0..4 {
        if i % 2 == 0 {
            step(&buf_a, &mut buf_b, W, H);
        } else {
            step(&buf_b, &mut buf_a, W, H);
        }
    }
    // After 4 steps the glider: same shape shifted (1,1) down-right
    let final_cells = alive_cells(&buf_a);
    assert_eq!(final_cells.len(), initial.len(), "Glider should still have 5 cells after 4 steps");
    // Each cell should be shifted by (1,1)
    let shifted: Vec<(usize, usize)> = initial.iter().map(|&(x, y)| (x + 1, y + 1)).collect();
    assert_eq!(final_cells, shifted, "Glider should shift (1,1) after 4 steps");
}

// --- Edge wrapping at (0,0) ---

#[test]
fn test_edge_wrapping_origin() {
    let mut grid = empty_grid();
    // Place 3 alive neighbors around (0,0) using wrapping
    set_alive(&mut grid, &[
        (W - 1, H - 1), // top-left wrapping
        (0, H - 1),     // directly above wrapping
        (1, 0),          // right neighbor
    ]);
    let neighbors = count_alive_neighbors(&grid, 0, 0, W, H);
    assert_eq!(neighbors, 3, "Cell (0,0) should count 3 wrapped neighbors");
}

// --- Birth: dead cell with exactly 3 neighbors ---

#[test]
fn test_birth() {
    let mut src = empty_grid();
    // 3 alive neighbors around (5,5)
    set_alive(&mut src, &[(4, 4), (5, 4), (6, 4)]);
    let mut dst = empty_grid();
    step(&src, &mut dst, W, H);
    assert_eq!(dst[index(5, 5, W)], ALIVE, "Dead cell with 3 alive neighbors should become alive");
}

// --- Survival: alive cell with 2 or 3 neighbors ---

#[test]
fn test_survival() {
    let mut src = empty_grid();
    // Cell (5,5) alive with 2 alive neighbors
    set_alive(&mut src, &[(5, 5), (4, 5), (6, 5)]);
    let mut dst = empty_grid();
    step(&src, &mut dst, W, H);
    assert_eq!(dst[index(5, 5, W)], ALIVE, "Alive cell with 2 neighbors should survive");
}

// --- Death: alive cell with <2 or >3 neighbors ---

#[test]
fn test_death_underpopulation() {
    let mut src = empty_grid();
    // Cell (5,5) alive with only 1 neighbor
    set_alive(&mut src, &[(5, 5), (4, 5)]);
    let mut dst = empty_grid();
    step(&src, &mut dst, W, H);
    assert_eq!(dst[index(5, 5, W)], 192, "Alive cell with 1 neighbor should begin dying (192)");
}

#[test]
fn test_death_overpopulation() {
    let mut src = empty_grid();
    // Cell (5,5) alive with 4 neighbors
    set_alive(&mut src, &[(5, 5), (4, 5), (6, 5), (5, 4), (5, 6)]);
    let mut dst = empty_grid();
    step(&src, &mut dst, W, H);
    assert_eq!(dst[index(5, 5, W)], 192, "Alive cell with 4 neighbors should begin dying (192)");
}

// --- Fade decrement ---

#[test]
fn test_fade_decrement() {
    let mut src = empty_grid();
    src[index(5, 5, W)] = 100; // Dying cell, no alive neighbors nearby
    let mut dst = empty_grid();
    step(&src, &mut dst, W, H);
    assert_eq!(dst[index(5, 5, W)], 36, "Dying cell should decrement by 64");
}

// --- Dying rebirth ---

#[test]
fn test_dying_rebirth() {
    let mut src = empty_grid();
    src[index(5, 5, W)] = 50; // Dying cell
    // Give it exactly 3 alive neighbors
    set_alive(&mut src, &[(4, 4), (5, 4), (6, 4)]);
    let mut dst = empty_grid();
    step(&src, &mut dst, W, H);
    assert_eq!(dst[index(5, 5, W)], ALIVE, "Dying cell with 3 alive neighbors should be reborn");
}

// --- Empty grid stays empty ---

#[test]
fn test_empty_grid_stays_empty() {
    let src = empty_grid();
    let mut dst = empty_grid();
    step(&src, &mut dst, W, H);
    assert!(dst.iter().all(|&v| v == 0), "Empty grid should remain empty");
}

// --- Glider rotation variants ---

#[test]
fn test_spawn_glider_all_rotations_produce_5_cells() {
    for rotation in 0..4 {
        let mut grid = empty_grid();
        spawn_glider(&mut grid, 128, 128, rotation, W, H);
        let cells = alive_cells(&grid);
        assert_eq!(cells.len(), 5, "Rotation {} should place exactly 5 cells", rotation);
    }
}

#[test]
fn test_spawn_glider_rotations_are_distinct() {
    let mut grids = Vec::new();
    for rotation in 0..4 {
        let mut grid = empty_grid();
        spawn_glider(&mut grid, 128, 128, rotation, W, H);
        grids.push(alive_cells(&grid));
    }
    for i in 0..4 {
        for j in (i + 1)..4 {
            assert_ne!(grids[i], grids[j], "Rotation {} and {} should produce different patterns", i, j);
        }
    }
}

#[test]
fn test_spawn_glider_edge_wrapping() {
    let mut grid = empty_grid();
    spawn_glider(&mut grid, 255, 255, 0, W, H);
    let cells = alive_cells(&grid);
    assert_eq!(cells.len(), 5, "Glider at (255,255) should wrap and place exactly 5 cells");
}

#[test]
fn test_spawn_glider_each_rotation_evolves_correctly() {
    // Each rotated glider should still be a valid glider — after 4 steps,
    // it should have 5 alive cells (same shape, shifted).
    for rotation in 0..4 {
        let mut grid = empty_grid();
        spawn_glider(&mut grid, 128, 128, rotation, W, H);
        let mut buf_a = grid;
        let mut buf_b = empty_grid();
        for i in 0..4 {
            if i % 2 == 0 {
                step(&buf_a, &mut buf_b, W, H);
            } else {
                step(&buf_b, &mut buf_a, W, H);
            }
        }
        let final_cells = alive_cells(&buf_a);
        assert_eq!(final_cells.len(), 5, "Rotation {} should still have 5 alive cells after 4 steps", rotation);
    }
}

// --- Aspect-ratio grid sizing ---

#[test]
fn test_grid_config_for_screen_16_10() {
    let gc = GridConfig::for_screen(2560.0, 1600.0);
    assert_eq!(gc.height, 256);
    assert_eq!(gc.width, 410); // round(256 * 2560/1600) = round(409.6) = 410
}

#[test]
fn test_grid_config_for_screen_16_9() {
    let gc = GridConfig::for_screen(2560.0, 1440.0);
    assert_eq!(gc.height, 256);
    assert_eq!(gc.width, 455); // round(256 * 2560/1440) = round(455.11) = 455
}

#[test]
fn test_grid_config_for_screen_square() {
    let gc = GridConfig::for_screen(1024.0, 1024.0);
    assert_eq!(gc.height, 256);
    assert_eq!(gc.width, 256);
}

#[test]
fn test_grid_config_default_is_256x256() {
    let gc = GridConfig::default();
    assert_eq!(gc.width, 256);
    assert_eq!(gc.height, 256);
    assert_eq!(gc.size(), 256 * 256);
}

// --- Non-square grid step ---

#[test]
fn test_step_wh_blinker_on_wide_grid() {
    let w = 32;
    let h = 20;
    let mut src = empty_grid_wh(w, h);
    // Horizontal blinker at (16, 10)
    src[index(15, 10, w)] = ALIVE;
    src[index(16, 10, w)] = ALIVE;
    src[index(17, 10, w)] = ALIVE;

    let mut dst = empty_grid_wh(w, h);
    step(&src, &mut dst, w, h);

    // Should become vertical
    assert_eq!(dst[index(16, 9, w)], ALIVE);
    assert_eq!(dst[index(16, 10, w)], ALIVE);
    assert_eq!(dst[index(16, 11, w)], ALIVE);
    // Horizontal cells should have started dying
    assert_eq!(dst[index(15, 10, w)], 192);
    assert_eq!(dst[index(17, 10, w)], 192);
}
