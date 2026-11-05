//! Sample-rate conversion from the simulation to the sound card: windowed sinc on a fixed
//! grid of fractional positions. Works in the audio callback, it allocates only in `new`.

use std::f64::consts::PI;

const TAPS: usize = 32;
const PHASES: usize = 256;

pub struct Resampler {
    table: Vec<[f32; TAPS]>,
    hist: [f32; TAPS],
    /// Input samples per output sample.
    step: f64,
    frac: f64,
}

impl Resampler {
    pub fn new(from: u32, to: u32) -> Self {
        let step = from as f64 / to as f64;
        // the cutoff stays below the input Nyquist, and below the output one when going down
        let cutoff = 0.92 * step.recip().min(1.0);
        let i0 = |x: f64| {
            let (mut sum, mut term) = (1.0, 1.0);
            for k in 1..30 {
                term *= (x / (2.0 * k as f64)).powi(2);
                sum += term;
            }
            sum
        };
        let beta = 9.0;
        let mut table = vec![[0f32; TAPS]; PHASES + 1];
        for (p, row) in table.iter_mut().enumerate() {
            let frac = p as f64 / PHASES as f64;
            let mut sum = 0.0;
            let mut v = [0f64; TAPS];
            for (t, c) in v.iter_mut().enumerate() {
                // the newest sample is tap TAPS - 1, the output lies between the two middle ones
                let x = t as f64 - (TAPS / 2 - 1) as f64 - frac;
                let s = if x.abs() < 1e-9 { 1.0 } else { (PI * cutoff * x).sin() / (PI * cutoff * x) };
                let w = 1.0 - (x / (TAPS as f64 / 2.0)).powi(2);
                *c = s * i0(beta * w.max(0.0).sqrt()) / i0(beta);
                sum += *c;
            }
            for (o, c) in row.iter_mut().zip(v) {
                *o = (c / sum) as f32;
            }
        }
        Resampler { table, hist: [0.0; TAPS], step, frac: 0.0 }
    }

    /// One output sample. `input` is asked for as many new samples as this one needs.
    #[inline]
    pub fn next(&mut self, mut input: impl FnMut() -> f32) -> f32 {
        while self.frac >= 1.0 {
            self.frac -= 1.0;
            self.hist.copy_within(1.., 0);
            self.hist[TAPS - 1] = input();
        }
        let pos = self.frac * PHASES as f64;
        let i = pos as usize;
        let t = (pos - i as f64) as f32;
        let (a, b) = (&self.table[i], &self.table[i + 1]);
        let mut y = 0.0;
        for k in 0..TAPS {
            y += self.hist[k] * (a[k] + t * (b[k] - a[k]));
        }
        self.frac += self.step;
        y
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Level of a tone in a signal, by correlation.
    fn level(sig: &[f32], hz: f64, rate: f64) -> f64 {
        let (mut s, mut c) = (0.0, 0.0);
        for (i, v) in sig.iter().enumerate() {
            let p = 2.0 * PI * hz * i as f64 / rate;
            s += *v as f64 * p.sin();
            c += *v as f64 * p.cos();
        }
        2.0 * (s * s + c * c).sqrt() / sig.len() as f64
    }

    fn convert(hz: f64, to: u32) -> Vec<f32> {
        let mut r = Resampler::new(11025, to);
        let mut n = 0u64;
        let out: Vec<f32> = (0..to)
            .map(|_| {
                r.next(|| {
                    n += 1;
                    (2.0 * PI * hz * n as f64 / 11025.0).sin() as f32
                })
            })
            .collect();
        out[2000..].to_vec()
    }

    #[test]
    fn tones_pass_at_full_level() {
        for to in [44100, 48000, 96000] {
            for hz in [300.0, 600.0, 900.0, 3000.0] {
                let l = level(&convert(hz, to), hz, to as f64);
                assert!((l - 1.0).abs() < 0.01, "{to} {hz}: {l}");
            }
        }
    }

    #[test]
    fn images_are_60_db_down() {
        for to in [44100, 48000] {
            for hz in [600.0, 1000.0] {
                let sig = convert(hz, to);
                for image in [11025.0 - hz, 11025.0 + hz, 22050.0 - hz] {
                    let l = level(&sig, image, to as f64);
                    assert!(l < 1e-3, "{to} {hz} image {image}: {l}");
                }
            }
        }
    }

    #[test]
    fn takes_the_right_amount_of_input() {
        let mut r = Resampler::new(11025, 48000);
        let mut n = 0i32;
        for _ in 0..48000 {
            r.next(|| {
                n += 1;
                0.0
            });
        }
        assert!((n - 11025).abs() <= 1, "{n}");
    }
}
