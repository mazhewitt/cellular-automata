pub const GRID_WIDTH: usize = 256;
pub const GRID_HEIGHT: usize = 256;
pub const GRID_SIZE: usize = GRID_WIDTH * GRID_HEIGHT;
pub const ALIVE: u8 = 255;

#[inline]
pub fn index(x: usize, y: usize) -> usize {
    y * GRID_WIDTH + x
}

pub fn count_alive_neighbors(grid: &[u8], x: usize, y: usize) -> u8 {
    let mut count: u8 = 0;
    for dy in [GRID_HEIGHT - 1, 0, 1] {
        for dx in [GRID_WIDTH - 1, 0, 1] {
            if dx == 0 && dy == 0 {
                continue;
            }
            let nx = (x + dx) % GRID_WIDTH;
            let ny = (y + dy) % GRID_HEIGHT;
            if grid[index(nx, ny)] == ALIVE {
                count += 1;
            }
        }
    }
    count
}

pub fn step(src: &[u8], dst: &mut [u8]) {
    for y in 0..GRID_HEIGHT {
        for x in 0..GRID_WIDTH {
            let idx = index(x, y);
            let cell = src[idx];
            let neighbors = count_alive_neighbors(src, x, y);

            dst[idx] = if cell == ALIVE {
                if neighbors == 2 || neighbors == 3 { ALIVE } else { 192 }
            } else if cell == 0 {
                if neighbors == 3 { ALIVE } else { 0 }
            } else {
                if neighbors == 3 { ALIVE } else { cell.saturating_sub(64) }
            };
        }
    }
}

fn place(grid: &mut [u8], cx: usize, cy: usize, offsets: &[(isize, isize)]) {
    for &(dx, dy) in offsets {
        let x = (cx as isize + dx).rem_euclid(GRID_WIDTH as isize) as usize;
        let y = (cy as isize + dy).rem_euclid(GRID_HEIGHT as isize) as usize;
        grid[index(x, y)] = ALIVE;
    }
}

pub fn seed_blinker(grid: &mut [u8], cx: usize, cy: usize) {
    place(grid, cx, cy, &[(-1, 0), (0, 0), (1, 0)]);
}

pub fn seed_glider(grid: &mut [u8], cx: usize, cy: usize) {
    // Standard glider:
    //  .X.
    //  ..X
    //  XXX
    place(grid, cx, cy, &[(0, -1), (1, 0), (-1, 1), (0, 1), (1, 1)]);
}

pub fn seed_r_pentomino(grid: &mut [u8], cx: usize, cy: usize) {
    // R-pentomino:
    //  .XX
    //  XX.
    //  .X.
    place(grid, cx, cy, &[(0, -1), (1, -1), (-1, 0), (0, 0), (0, 1)]);
}

/// Four glider rotation offset tables (0°=SE, 90°=SW, 180°=NW, 270°=NE).
const GLIDER_ROTATIONS: [[(isize, isize); 5]; 4] = [
    // 0° (SE):  .X. / ..X / XXX
    [(0, -1), (1, 0), (-1, 1), (0, 1), (1, 1)],
    // 90° (SW): X.. / XXX / .X.  — rotate (x,y)→(-y,x)
    [(1, 0), (0, 1), (-1, -1), (-1, 0), (-1, 1)],
    // 180° (NW): XXX / X.. / .X. — rotate (x,y)→(-x,-y)
    [(0, 1), (-1, 0), (1, -1), (0, -1), (-1, -1)],
    // 270° (NE): .X. / XXX / ..X — rotate (x,y)→(y,-x)
    [(-1, 0), (0, -1), (1, 1), (1, 0), (1, -1)],
];

/// Spawn a glider at (cx, cy) with the given rotation (0–3).
pub fn spawn_glider(grid: &mut [u8], cx: usize, cy: usize, rotation: usize) {
    place(grid, cx, cy, &GLIDER_ROTATIONS[rotation % 4]);
}
