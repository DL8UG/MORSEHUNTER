//! The tide of a contest: callers come in waves, with lulls and short openings, instead of
//! the same crowd all run long. Not part of the original, off unless asked for.

use std::f64::consts::TAU;

use crate::rng::Rng;

/// The slow swell: three waves of their own length, the first the strongest.
const WEIGHTS: [f64; 3] = [0.6, 0.3, 0.15];
const PERIOD_MIN: f64 = 6.0;
const PERIOD_MAX: f64 = 30.0;
/// How far the swell moves the crowd up and down.
const SWELL: f64 = 0.55;
/// An opening on average every this many minutes, at least one.
const OPENING_EVERY: f64 = 20.0;
/// An opening doubles the crowd at its height.
const OPENING_GAIN: f64 = 1.0;
const FLOOR: f64 = 0.2;
const CEILING: f64 = 2.5;

#[derive(Clone, Debug, PartialEq)]
pub struct Waves {
    /// Period in minutes and phase of each wave.
    parts: [(f64, f64); 3],
    /// Start and length of each opening, in minutes.
    openings: Vec<(f64, f64)>,
}

impl Waves {
    /// The tide of a run of `minutes`.
    pub fn new(rng: &mut Rng, minutes: f64) -> Self {
        let parts = std::array::from_fn(|_| (rng.range(PERIOD_MIN, PERIOD_MAX), rng.range(0.0, TAU)));
        let n = (minutes / OPENING_EVERY).round().max(1.0) as usize;
        let openings = (0..n).map(|_| (rng.range(1.0, (minutes - 2.0).max(1.0)), rng.range(2.0, 5.0))).collect();
        Waves { parts, openings }
    }

    /// The crowd at `minute`, as a factor on the callers per CQ: 1 on average, from 0.2 in
    /// a lull to 2.5 in an opening.
    pub fn factor(&self, minute: f64) -> f64 {
        let swell: f64 = self.parts.iter().zip(WEIGHTS).map(|((p, ph), w)| w * (TAU * minute / p + ph).sin()).sum();
        let mut f = 1.0 + SWELL * swell;
        for (start, len) in &self.openings {
            let x = (minute - start) / len;
            if (0.0..1.0).contains(&x) {
                // up and down again, smoothly
                f *= 1.0 + OPENING_GAIN * (std::f64::consts::PI * x).sin().powi(2);
            }
        }
        f.clamp(FLOOR, CEILING)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_crowd_comes_and_goes() {
        for seed in 0..20 {
            let w = Waves::new(&mut Rng::new(seed), 60.0);
            let f: Vec<f64> = (0..600).map(|i| w.factor(i as f64 / 10.0)).collect();
            let (lo, hi) = f.iter().fold((f64::MAX, f64::MIN), |(a, b), v| (a.min(*v), b.max(*v)));
            assert!(lo >= FLOOR && hi <= CEILING, "seed {seed}: {lo} {hi}");
            assert!(hi - lo > 0.6, "seed {seed}: a tide that barely moves, {lo} to {hi}");
            let mean = f.iter().sum::<f64>() / f.len() as f64;
            assert!((0.7..1.4).contains(&mean), "seed {seed}: mean {mean}");
            // slow: from one tenth of a minute to the next it hardly moves
            assert!(f.windows(2).all(|p| (p[1] - p[0]).abs() < 0.35), "seed {seed}");
        }
    }

    #[test]
    fn every_run_has_an_opening_inside_it() {
        for seed in 0..20 {
            let w = Waves::new(&mut Rng::new(seed), 10.0);
            assert!(!w.openings.is_empty());
            assert!(w.openings.iter().all(|(s, _)| (1.0..=8.0).contains(s)), "{:?}", w.openings);
        }
    }

    #[test]
    fn the_same_seed_gives_the_same_tide() {
        assert_eq!(Waves::new(&mut Rng::new(3), 30.0), Waves::new(&mut Rng::new(3), 30.0));
    }
}
