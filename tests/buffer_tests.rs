#[cfg(test)]
mod tests {
    use um_game_of_life::game_of_life::*;

    const W: usize = 256;
    const H: usize = 256;
    const SZ: usize = W * H;

    #[test]
    fn test_index_correctness() {
        assert_eq!(index(0, 0, W), 0);
        assert_eq!(index(1, 0, W), 1);
        assert_eq!(index(0, 1, W), W);
        assert_eq!(index(W - 1, H - 1, W), SZ - 1);
        assert_eq!(index(10, 20, W), 20 * W + 10);
    }

    #[test]
    fn test_double_buffer_swap_alternates() {
        let mut current: usize = 0;
        assert_eq!(current, 0);

        current ^= 1;
        assert_eq!(current, 1);

        current ^= 1;
        assert_eq!(current, 0);

        // Verify read/write roles.
        for _ in 0..10 {
            let read = current;
            let write = 1 - current;
            assert_ne!(read, write);
            current ^= 1;
        }
    }

    #[test]
    fn test_seed_blinker_places_cells() {
        let mut grid = vec![0u8; SZ];
        let cx = W / 2;
        let cy = H / 2;
        seed_blinker(&mut grid, cx, cy, W, H);

        // Blinker: three horizontal cells at (cx-1,cy), (cx,cy), (cx+1,cy).
        assert_eq!(grid[index(cx - 1, cy, W)], ALIVE);
        assert_eq!(grid[index(cx, cy, W)], ALIVE);
        assert_eq!(grid[index(cx + 1, cy, W)], ALIVE);

        // Exactly 3 alive cells.
        let alive_count = grid.iter().filter(|&&v| v == ALIVE).count();
        assert_eq!(alive_count, 3);
    }

    #[test]
    fn test_seed_glider_places_cells() {
        let mut grid = vec![0u8; SZ];
        let cx = W / 2;
        let cy = H / 2;
        seed_glider(&mut grid, cx, cy, W, H);

        // Glider: 5 cells.
        let alive_count = grid.iter().filter(|&&v| v == ALIVE).count();
        assert_eq!(alive_count, 5);

        // Check expected positions.
        assert_eq!(grid[index(cx, cy - 1, W)], ALIVE);
        assert_eq!(grid[index(cx + 1, cy, W)], ALIVE);
        assert_eq!(grid[index(cx - 1, cy + 1, W)], ALIVE);
        assert_eq!(grid[index(cx, cy + 1, W)], ALIVE);
        assert_eq!(grid[index(cx + 1, cy + 1, W)], ALIVE);
    }

    #[test]
    fn test_seed_r_pentomino_places_cells() {
        let mut grid = vec![0u8; SZ];
        let cx = W / 2;
        let cy = H / 2;
        seed_r_pentomino(&mut grid, cx, cy, W, H);

        // R-pentomino: 5 cells.
        let alive_count = grid.iter().filter(|&&v| v == ALIVE).count();
        assert_eq!(alive_count, 5);

        // Check expected positions.
        assert_eq!(grid[index(cx, cy - 1, W)], ALIVE);
        assert_eq!(grid[index(cx + 1, cy - 1, W)], ALIVE);
        assert_eq!(grid[index(cx - 1, cy, W)], ALIVE);
        assert_eq!(grid[index(cx, cy, W)], ALIVE);
        assert_eq!(grid[index(cx, cy + 1, W)], ALIVE);
    }
}
