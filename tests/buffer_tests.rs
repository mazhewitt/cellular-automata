#[cfg(test)]
mod tests {
    use um_game_of_life::grid::*;

    #[test]
    fn test_index_correctness() {
        assert_eq!(index(0, 0), 0);
        assert_eq!(index(1, 0), 1);
        assert_eq!(index(0, 1), GRID_WIDTH);
        assert_eq!(index(GRID_WIDTH - 1, GRID_HEIGHT - 1), GRID_SIZE - 1);
        assert_eq!(index(10, 20), 20 * GRID_WIDTH + 10);
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
        let mut grid = vec![0u8; GRID_SIZE];
        let cx = GRID_WIDTH / 2;
        let cy = GRID_HEIGHT / 2;
        seed_blinker(&mut grid, cx, cy);

        // Blinker: three horizontal cells at (cx-1,cy), (cx,cy), (cx+1,cy).
        assert_eq!(grid[index(cx - 1, cy)], ALIVE);
        assert_eq!(grid[index(cx, cy)], ALIVE);
        assert_eq!(grid[index(cx + 1, cy)], ALIVE);

        // Exactly 3 alive cells.
        let alive_count = grid.iter().filter(|&&v| v == ALIVE).count();
        assert_eq!(alive_count, 3);
    }

    #[test]
    fn test_seed_glider_places_cells() {
        let mut grid = vec![0u8; GRID_SIZE];
        let cx = GRID_WIDTH / 2;
        let cy = GRID_HEIGHT / 2;
        seed_glider(&mut grid, cx, cy);

        // Glider: 5 cells.
        let alive_count = grid.iter().filter(|&&v| v == ALIVE).count();
        assert_eq!(alive_count, 5);

        // Check expected positions.
        assert_eq!(grid[index(cx, cy - 1)], ALIVE);
        assert_eq!(grid[index(cx + 1, cy)], ALIVE);
        assert_eq!(grid[index(cx - 1, cy + 1)], ALIVE);
        assert_eq!(grid[index(cx, cy + 1)], ALIVE);
        assert_eq!(grid[index(cx + 1, cy + 1)], ALIVE);
    }

    #[test]
    fn test_seed_r_pentomino_places_cells() {
        let mut grid = vec![0u8; GRID_SIZE];
        let cx = GRID_WIDTH / 2;
        let cy = GRID_HEIGHT / 2;
        seed_r_pentomino(&mut grid, cx, cy);

        // R-pentomino: 5 cells.
        let alive_count = grid.iter().filter(|&&v| v == ALIVE).count();
        assert_eq!(alive_count, 5);

        // Check expected positions.
        assert_eq!(grid[index(cx, cy - 1)], ALIVE);
        assert_eq!(grid[index(cx + 1, cy - 1)], ALIVE);
        assert_eq!(grid[index(cx - 1, cy)], ALIVE);
        assert_eq!(grid[index(cx, cy)], ALIVE);
        assert_eq!(grid[index(cx, cy + 1)], ALIVE);
    }
}
