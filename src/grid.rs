pub const GRID_WIDTH: usize = 256;
pub const GRID_HEIGHT: usize = 256;
pub const GRID_SIZE: usize = GRID_WIDTH * GRID_HEIGHT;
pub const ALIVE: u8 = 255;

#[derive(Clone, Copy, Debug)]
pub struct GridConfig {
    pub width: usize,
    pub height: usize,
}

impl GridConfig {
    pub fn size(&self) -> usize {
        self.width * self.height
    }

    /// Compute grid dimensions for a screen so cells are square.
    /// Fixes height at 256 and derives width from the aspect ratio.
    pub fn for_screen(screen_width: f64, screen_height: f64) -> Self {
        let height = GRID_HEIGHT;
        let width = (height as f64 * screen_width / screen_height).round() as usize;
        GridConfig { width: width.max(1), height }
    }
}

impl Default for GridConfig {
    fn default() -> Self {
        GridConfig { width: GRID_WIDTH, height: GRID_HEIGHT }
    }
}

#[inline]
pub fn index(x: usize, y: usize) -> usize {
    y * GRID_WIDTH + x
}

#[inline]
pub fn index_wh(x: usize, y: usize, width: usize) -> usize {
    y * width + x
}

pub fn count_alive_neighbors(grid: &[u8], x: usize, y: usize) -> u8 {
    count_alive_neighbors_wh(grid, x, y, GRID_WIDTH, GRID_HEIGHT)
}

pub fn count_alive_neighbors_wh(grid: &[u8], x: usize, y: usize, width: usize, height: usize) -> u8 {
    let mut count: u8 = 0;
    for dy in [height - 1, 0, 1] {
        for dx in [width - 1, 0, 1] {
            if dx == 0 && dy == 0 {
                continue;
            }
            let nx = (x + dx) % width;
            let ny = (y + dy) % height;
            if grid[index_wh(nx, ny, width)] == ALIVE {
                count += 1;
            }
        }
    }
    count
}

pub fn step(src: &[u8], dst: &mut [u8]) {
    step_wh(src, dst, GRID_WIDTH, GRID_HEIGHT);
}

pub fn step_wh(src: &[u8], dst: &mut [u8], width: usize, height: usize) {
    for y in 0..height {
        for x in 0..width {
            let idx = index_wh(x, y, width);
            let cell = src[idx];
            let neighbors = count_alive_neighbors_wh(src, x, y, width, height);

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
    place_wh(grid, cx, cy, offsets, GRID_WIDTH, GRID_HEIGHT);
}

fn place_wh(grid: &mut [u8], cx: usize, cy: usize, offsets: &[(isize, isize)], width: usize, height: usize) {
    for &(dx, dy) in offsets {
        let x = (cx as isize + dx).rem_euclid(width as isize) as usize;
        let y = (cy as isize + dy).rem_euclid(height as isize) as usize;
        grid[index_wh(x, y, width)] = ALIVE;
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

/// Spawn a glider at (cx, cy) with custom grid dimensions.
pub fn spawn_glider_wh(grid: &mut [u8], cx: usize, cy: usize, rotation: usize, width: usize, height: usize) {
    place_wh(grid, cx, cy, &GLIDER_ROTATIONS[rotation % 4], width, height);
}
