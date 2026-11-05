//! Everything of Morse Runner that needs neither sound card nor window. It runs faster
//! than real time, the tests play whole contests with a scripted operator.

pub mod analysis;
pub mod bot;
pub mod callgen;
pub mod code;
pub mod contest;
pub mod country;
pub mod dsp;
pub mod dx;
pub mod entry;
pub mod fist;
pub mod keyer;
pub mod log;
pub mod morse;
pub mod msg;
pub mod paddle;
pub mod parser;
pub mod prefixes;
pub mod resample;
pub mod rng;
pub mod rxtext;
pub mod sent;
pub mod settings;
pub mod skill;
pub mod spectrum;
pub mod station;
pub mod waves;

/// Sample rate of the simulation.
pub const RATE: u32 = 11025;
/// Samples per block. All timers count blocks.
pub const BLOCK: usize = 256;

pub fn seconds_to_blocks(s: f64) -> i32 {
    (RATE as f64 / BLOCK as f64 * s).round() as i32
}

pub fn blocks_to_seconds(b: u64) -> f64 {
    b as f64 * BLOCK as f64 / RATE as f64
}
