// Shared application constants used by renderer event loops.

use std::sync::atomic::AtomicBool;

pub const TICK_RATES: &[u64] = &[1, 2, 5, 10, 20, 30, 60, 120];

pub static SIGTERM_RECEIVED: AtomicBool = AtomicBool::new(false);
