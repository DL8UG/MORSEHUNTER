//! Small deterministic generator: a seed gives the same session on every platform.

use std::f64::consts::PI;

#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        // a zero state would stay zero
        Rng((seed ^ 0x5DEE_CE66_D000_0001).wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// Uniform in 0..1.
    pub fn f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / 9007199254740992.0)
    }

    pub fn range(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.f64()
    }

    /// Uniform in 0..n, 0 for an empty range.
    pub fn below(&mut self, n: usize) -> usize {
        (self.f64() * n as f64) as usize
    }

    pub fn chance(&mut self, p: f64) -> bool {
        self.f64() < p
    }

    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }

    /// Standard normal.
    pub fn gauss(&mut self) -> f64 {
        let u = self.f64().max(1e-12);
        let v = self.f64();
        (-2.0 * u.ln()).sqrt() * (2.0 * PI * v).cos()
    }

    /// Poisson, small means only (Knuth), at most 30.
    pub fn poisson(&mut self, mean: f64) -> usize {
        let limit = (-mean).exp();
        let mut p = 1.0;
        for n in 0..30 {
            p *= self.f64();
            if p <= limit {
                return n;
            }
        }
        30
    }

    /// Normal around `mean`, 2 sigma at `lim`, clipped there.
    pub fn gauss_lim(&mut self, mean: f64, lim: f64) -> f64 {
        (mean + self.gauss() * 0.5 * lim).clamp(mean - lim.abs(), mean + lim.abs())
    }

    pub fn rayleigh(&mut self, mean: f64) -> f64 {
        let (a, b) = (self.f64().max(1e-12), self.f64().max(1e-12));
        mean * (-a.ln() - b.ln()).sqrt()
    }

    /// Uniform in -1..1.
    pub fn uniform(&mut self) -> f64 {
        2.0 * self.f64() - 1.0
    }

    /// In -1..1, mostly near the ends.
    pub fn u_shaped(&mut self) -> f64 {
        (PI * (self.f64() - 0.5)).sin()
    }
}
