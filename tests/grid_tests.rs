use um_game_of_life::grid::*;

fn empty_grid() -> Vec<u8> {
    vec![0u8; GRID_SIZE]
}

fn set_alive(grid: &mut [u8], cells: &[(usize, usize)]) {
    for &(x, y) in cells {
        grid[index(x, y)] = ALIVE;
    }
}

fn alive_cells(grid: &[u8]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    for y in 0..GRID_HEIGHT {
        for x in 0..GRID_WIDTH {
            if grid[index(x, y)] == ALIVE {
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
    step(&src, &mut dst);
    assert_eq!(alive_cells(&dst), alive_cells(&src), "Block should be unchanged after one step");
}

// --- Oscillator: blinker period 2 ---

#[test]
fn test_blinker_oscillator() {
    let mut src = empty_grid();
    // Horizontal blinker at (10,10)
    set_alive(&mut src, &[(9, 10), (10, 10), (11, 10)]);

    let mut gen1 = empty_grid();
    step(&src, &mut gen1);
    // Should become vertical
    let expected_v: Vec<(usize, usize)> = vec![(10, 9), (10, 10), (10, 11)];
    assert_eq!(alive_cells(&gen1), expected_v, "Blinker gen1 should be vertical");

    let mut gen2 = empty_grid();
    step(&gen1, &mut gen2);
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
            step(&buf_a, &mut buf_b);
        } else {
            step(&buf_b, &mut buf_a);
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
        (GRID_WIDTH - 1, GRID_HEIGHT - 1), // top-left wrapping
        (0, GRID_HEIGHT - 1),               // directly above wrapping
        (1, 0),                              // right neighbor
    ]);
    let neighbors = count_alive_neighbors(&grid, 0, 0);
    assert_eq!(neighbors, 3, "Cell (0,0) should count 3 wrapped neighbors");
}

// --- Birth: dead cell with exactly 3 neighbors ---

#[test]
fn test_birth() {
    let mut src = empty_grid();
    // 3 alive neighbors around (5,5)
    set_alive(&mut src, &[(4, 4), (5, 4), (6, 4)]);
    let mut dst = empty_grid();
    step(&src, &mut dst);
    assert_eq!(dst[index(5, 5)], ALIVE, "Dead cell with 3 alive neighbors should become alive");
}

// --- Survival: alive cell with 2 or 3 neighbors ---

#[test]
fn test_survival() {
    let mut src = empty_grid();
    // Cell (5,5) alive with 2 alive neighbors
    set_alive(&mut src, &[(5, 5), (4, 5), (6, 5)]);
    let mut dst = empty_grid();
    step(&src, &mut dst);
    assert_eq!(dst[index(5, 5)], ALIVE, "Alive cell with 2 neighbors should survive");
}

// --- Death: alive cell with <2 or >3 neighbors ---

#[test]
fn test_death_underpopulation() {
    let mut src = empty_grid();
    // Cell (5,5) alive with only 1 neighbor
    set_alive(&mut src, &[(5, 5), (4, 5)]);
    let mut dst = empty_grid();
    step(&src, &mut dst);
    assert_eq!(dst[index(5, 5)], 192, "Alive cell with 1 neighbor should begin dying (192)");
}

#[test]
fn test_death_overpopulation() {
    let mut src = empty_grid();
    // Cell (5,5) alive with 4 neighbors
    set_alive(&mut src, &[(5, 5), (4, 5), (6, 5), (5, 4), (5, 6)]);
    let mut dst = empty_grid();
    step(&src, &mut dst);
    assert_eq!(dst[index(5, 5)], 192, "Alive cell with 4 neighbors should begin dying (192)");
}

// --- Fade decrement ---

#[test]
fn test_fade_decrement() {
    let mut src = empty_grid();
    src[index(5, 5)] = 100; // Dying cell, no alive neighbors nearby
    let mut dst = empty_grid();
    step(&src, &mut dst);
    assert_eq!(dst[index(5, 5)], 36, "Dying cell should decrement by 64");
}

// --- Dying rebirth ---

#[test]
fn test_dying_rebirth() {
    let mut src = empty_grid();
    src[index(5, 5)] = 50; // Dying cell
    // Give it exactly 3 alive neighbors
    set_alive(&mut src, &[(4, 4), (5, 4), (6, 4)]);
    let mut dst = empty_grid();
    step(&src, &mut dst);
    assert_eq!(dst[index(5, 5)], ALIVE, "Dying cell with 3 alive neighbors should be reborn");
}

// --- Empty grid stays empty ---

#[test]
fn test_empty_grid_stays_empty() {
    let src = empty_grid();
    let mut dst = empty_grid();
    step(&src, &mut dst);
    assert!(dst.iter().all(|&v| v == 0), "Empty grid should remain empty");
}
