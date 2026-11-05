//! Audio blocks: receiver filter, modulator, AGC and fading.

use std::collections::VecDeque;
use std::f64::consts::TAU;

use crate::rng::Rng;
use crate::{BLOCK, RATE};

/// Moving average run three times over, for one of the I and Q paths. The sums are `f64`:
/// the rounding error stays far below the signal for any contest length.
struct Average {
    ring: [Vec<f32>; 3],
    sum: [f64; 3],
    pos: usize,
}

impl Average {
    fn new(points: usize) -> Self {
        Average { ring: [vec![0.0; points], vec![0.0; points], vec![0.0; points]], sum: [0.0; 3], pos: 0 }
    }

    #[inline]
    fn run(&mut self, mut x: f32) -> f32 {
        let n = self.ring[0].len() as f64;
        for p in 0..3 {
            self.sum[p] += x as f64 - self.ring[p][self.pos] as f64;
            self.ring[p][self.pos] = x;
            x = (self.sum[p] / n) as f32;
        }
        self.pos = (self.pos + 1) % self.ring[0].len();
        x
    }
}

/// The receiver filter at baseband.
pub struct Filter {
    re: Average,
    im: Average,
    gain: f32,
}

impl Filter {
    pub fn new(bandwidth: u32) -> Self {
        let points = (0.7 * RATE as f64 / bandwidth as f64).round() as usize;
        let db = 10.0 * (500.0 / bandwidth as f32).log10();
        Filter { re: Average::new(points), im: Average::new(points), gain: 10f32.powf(0.05 * db) }
    }

    pub fn run(&mut self, re: &mut [f32; BLOCK], im: &mut [f32; BLOCK]) {
        for v in re.iter_mut() {
            *v = self.re.run(*v) * self.gain;
        }
        for v in im.iter_mut() {
            *v = self.im.run(*v) * self.gain;
        }
    }
}

/// Mixes the baseband up to the pitch.
pub struct Modulator {
    phase: f64,
    step: f64,
}

impl Modulator {
    pub fn new(pitch: u32) -> Self {
        let mut m = Modulator { phase: 0.0, step: 0.0 };
        m.set_pitch(pitch);
        m
    }

    pub fn set_pitch(&mut self, pitch: u32) {
        self.step = TAU * pitch as f64 / RATE as f64;
    }

    pub fn run(&mut self, re: &[f32; BLOCK], im: &[f32; BLOCK], out: &mut [f32; BLOCK]) {
        for i in 0..BLOCK {
            let (s, c) = self.phase.sin_cos();
            out[i] = re[i] * s as f32 - im[i] * c as f32;
            self.phase = (self.phase + self.step) % TAU;
        }
    }
}

const AGC_ATTACK: usize = 155;
const AGC_HOLD: usize = 155;
const AGC_LEN: usize = 2 * (AGC_ATTACK + AGC_HOLD) + 1;

/// Soft limiter with look-ahead: the gain follows the highest sample around the current
/// one, weighted by a raised cosine on both flanks.
pub struct Agc {
    shape: Vec<f32>,
    samples: Vec<f32>,
    mag: Vec<f32>,
    idx: usize,
    max_out: f32,
    beta: f32,
}

impl Agc {
    pub fn new() -> Self {
        let max_out = 20000f32;
        let noise_in = 10f32.powf(0.05 * 76.0);
        let noise_out = (0.25 * max_out).min(10f32.powf(0.05 * 76.0));
        let mut shape = vec![0.0; AGC_LEN];
        for i in 0..AGC_ATTACK {
            let v = (0.5 - 0.5 * ((i + 1) as f32 * std::f32::consts::PI / (AGC_ATTACK + 1) as f32).cos()).ln();
            shape[i] = v;
            shape[AGC_LEN - 1 - i] = v;
        }
        Agc {
            shape,
            samples: vec![0.0; AGC_LEN],
            mag: vec![0.0; AGC_LEN],
            idx: 0,
            max_out,
            beta: noise_in / (max_out / (max_out - noise_out)).ln(),
        }
    }

    fn gain(&self) -> f32 {
        let mut env = 1e-10f32;
        let (older, newer) = (&self.mag[self.idx..], &self.mag[..self.idx]);
        for (m, w) in older.iter().chain(newer).zip(&self.shape) {
            env = env.max(m + w);
        }
        let env = env.exp();
        self.max_out * (1.0 - (-env / self.beta).exp()) / env
    }

    pub fn run(&mut self, block: &mut [f32; BLOCK]) {
        for v in block.iter_mut() {
            self.samples[self.idx] = *v;
            self.mag[self.idx] = (v.abs() + 1e-10).ln();
            self.idx = (self.idx + 1) % AGC_LEN;
            *v = self.samples[(self.idx + AGC_LEN / 2) % AGC_LEN] * self.gain();
        }
    }
}

impl Default for Agc {
    fn default() -> Self {
        Self::new()
    }
}

/// Moving average of a complex signal, for the fading.
struct QuickAvg {
    points: usize,
    re: [VecDeque<f64>; 3],
    im: [VecDeque<f64>; 3],
    sum_re: [f64; 3],
    sum_im: [f64; 3],
}

impl QuickAvg {
    fn new(points: usize) -> Self {
        let q = || std::array::from_fn(|_| VecDeque::from(vec![0.0; points]));
        QuickAvg { points, re: q(), im: q(), sum_re: [0.0; 3], sum_im: [0.0; 3] }
    }

    fn step(q: &mut [VecDeque<f64>; 3], sum: &mut [f64; 3], mut x: f64, scale: f64) -> f64 {
        for p in 0..3 {
            let old = q[p].pop_front().unwrap_or(0.0);
            q[p].push_back(x);
            sum[p] += x - old;
            x = sum[p];
        }
        x * scale
    }

    fn run(&mut self, re: f64, im: f64) -> (f64, f64) {
        let scale = (self.points as f64).powi(-3);
        (Self::step(&mut self.re, &mut self.sum_re, re, scale), Self::step(&mut self.im, &mut self.sum_im, im, scale))
    }
}

const QSB_STEP: usize = 128;

/// Rayleigh fading of one station.
pub struct Qsb {
    filter: QuickAvg,
    gain: f32,
}

impl Qsb {
    pub fn new(bandwidth: f64, rng: &mut Rng) -> Self {
        let points = (0.37 * RATE as f64 / (QSB_STEP as f64 * bandwidth)).ceil() as usize;
        let mut q = Qsb { filter: QuickAvg::new(points.max(1)), gain: 1.0 };
        for _ in 0..=q.filter.points * 3 {
            q.gain = q.next_gain(rng);
        }
        q
    }

    fn next_gain(&mut self, rng: &mut Rng) -> f32 {
        let (re, im) = self.filter.run(rng.uniform(), rng.uniform());
        ((re * re + im * im) * 3.0 * self.filter.points as f64).sqrt() as f32
    }

    pub fn apply(&mut self, data: &mut [f32], rng: &mut Rng) {
        for chunk in data.chunks_mut(QSB_STEP) {
            let d = (self.next_gain(rng) - self.gain) / QSB_STEP as f32;
            for v in chunk {
                *v *= self.gain;
                self.gain += d;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::TAU as TAU32;

    /// Gain of the filter at an offset from the centre.
    fn response(bandwidth: u32, hz: f32) -> f32 {
        let mut f = Filter::new(bandwidth);
        let mut last = 0.0;
        let mut n = 0;
        for _ in 0..40 {
            let (mut re, mut im) = ([0.0; BLOCK], [0.0; BLOCK]);
            for i in 0..BLOCK {
                let p = TAU32 * hz * n as f32 / RATE as f32;
                re[i] = p.cos();
                im[i] = p.sin();
                n += 1;
            }
            f.run(&mut re, &mut im);
            last = re.iter().zip(&im).map(|(a, b)| (a * a + b * b).sqrt()).sum::<f32>() / BLOCK as f32;
        }
        last
    }

    #[test]
    fn filter_width_follows_the_setting() {
        for bw in [100, 250, 500] {
            let centre = response(bw, 0.0);
            let edge = response(bw, bw as f32 / 2.0) / centre;
            // half power near the band edge
            assert!((0.5..0.9).contains(&edge), "{bw}: {edge}");
            assert!(response(bw, bw as f32 * 1.5) / centre < 0.2, "{bw}");
        }
    }

    #[test]
    fn narrower_filter_keeps_the_signal_up() {
        let (wide, narrow) = (response(500, 0.0), response(100, 0.0));
        assert!((narrow / wide - 5f32.sqrt()).abs() < 0.05);
    }

    #[test]
    fn modulator_hits_the_pitch() {
        let mut m = Modulator::new(600);
        let (re, im) = ([1.0; BLOCK], [0.0; BLOCK]);
        let mut out = [0.0; BLOCK];
        let mut crossings = 0i32;
        let mut prev = 0.0;
        for _ in 0..43 {
            m.run(&re, &im, &mut out);
            for v in out {
                if prev <= 0.0 && v > 0.0 {
                    crossings += 1;
                }
                prev = v;
            }
        }
        // 43 blocks are 0.998 s
        assert!((crossings - 599).abs() <= 2, "{crossings}");
    }

    #[test]
    fn agc_limits_and_passes_quiet_signals() {
        let mut a = Agc::new();
        let mut peak = 0f32;
        for b in 0..40 {
            let mut blk = [0.0; BLOCK];
            for (i, v) in blk.iter_mut().enumerate() {
                *v = 300000.0 * (0.3 * (b * BLOCK + i) as f32).sin();
            }
            a.run(&mut blk);
            if b > 4 {
                peak = blk.iter().fold(peak, |p, v| p.max(v.abs()));
            }
        }
        assert!(peak <= 20000.0 && peak > 15000.0, "{peak}");
        let mut a = Agc::new();
        let mut last = [0.0; BLOCK];
        for _ in 0..10 {
            last = [100.0; BLOCK];
            a.run(&mut last);
        }
        let g = last[100] / 100.0;
        assert!((0.85..0.95).contains(&g), "{g}");
    }

    #[test]
    fn fading_averages_near_one_and_varies() {
        let mut rng = Rng::new(5);
        let mut q = Qsb::new(0.3, &mut rng);
        let (mut sum, mut low, mut high) = (0.0, f32::MAX, 0f32);
        let n = 60000;
        for _ in 0..n {
            let mut d = [1.0; QSB_STEP];
            q.apply(&mut d, &mut rng);
            sum += d[0] * d[0];
            low = low.min(d[0]);
            high = high.max(d[0]);
        }
        let mean = sum / n as f32;
        assert!((0.5..1.6).contains(&mean), "{mean}");
        assert!(low < 0.3 && high > 1.5, "{low} {high}");
    }
}
