//! The band around the pitch before the receive filter, for a small waterfall over the
//! passband. A row covers the same ±800 Hz as the RIT scale, in the frequency a station
//! sits at, so a trace stays put when the RIT moves and only the window goes over it.

use std::collections::VecDeque;
use std::f64::consts::TAU;

use crate::settings::RIT_MAX;
use crate::{BLOCK, RATE};

/// Bins of a row across the span.
pub const BINS: usize = 128;
/// Rows kept, the newest last.
pub const ROWS: usize = 48;
/// Hertz across a row, the RIT range with room for half the widest filter.
pub const SPAN: f64 = 2.0 * (RIT_MAX as f64 + 300.0);
/// Samples in a row: six blocks, 139 ms.
const N: usize = 6 * BLOCK;

pub type Row = [f32; BINS];

pub struct Spectrum {
    re: Vec<f32>,
    im: Vec<f32>,
    win: Vec<f64>,
    /// Power in dB, the oldest row first.
    pub rows: VecDeque<Row>,
    /// Rows made since the last reset.
    pub made: u64,
    /// Goes up with every reset, so a reader knows its rows are stale.
    pub epoch: u32,
}

impl Default for Spectrum {
    fn default() -> Self {
        let win = (0..N).map(|n| 0.5 - 0.5 * (TAU * n as f64 / N as f64).cos()).collect();
        Spectrum { re: Vec::with_capacity(N), im: Vec::with_capacity(N), win, rows: VecDeque::new(), made: 0, epoch: 0 }
    }
}

/// The frequency of a station that shows in bin k, relative to the pitch.
pub fn bin_hz(k: usize) -> f64 {
    -SPAN / 2.0 + (k as f64 + 0.5) * SPAN / BINS as f64
}

impl Spectrum {
    pub fn reset(&mut self) {
        self.re.clear();
        self.im.clear();
        self.rows.clear();
        self.made = 0;
        self.epoch = self.epoch.wrapping_add(1);
    }

    /// Takes a block of the baseband as it goes into the filter, with the RIT it was mixed with.
    pub fn push(&mut self, re: &[f32; BLOCK], im: &[f32; BLOCK], rit: i32) {
        self.re.extend_from_slice(re);
        self.im.extend_from_slice(im);
        if self.re.len() < N {
            return;
        }
        let row = self.row(rit);
        self.re.clear();
        self.im.clear();
        if self.rows.len() == ROWS {
            self.rows.pop_front();
        }
        self.rows.push_back(row);
        self.made += 1;
    }

    /// A station `d` Hz off the pitch is at `rit - d` in the baseband: a DFT at just those
    /// frequencies, by turning a phasor, costs less than a whole FFT.
    fn row(&self, rit: i32) -> Row {
        let mut row = [0f32; BINS];
        for (k, out) in row.iter_mut().enumerate() {
            let f = rit as f64 - bin_hz(k);
            let (s, c) = (-TAU * f / RATE as f64).sin_cos();
            let (mut wr, mut wi) = (1.0f64, 0.0f64);
            let (mut xr, mut xi) = (0.0f64, 0.0f64);
            for n in 0..N {
                let (a, b) = (self.re[n] as f64 * self.win[n], self.im[n] as f64 * self.win[n]);
                xr += a * wr - b * wi;
                xi += a * wi + b * wr;
                (wr, wi) = (wr * c - wi * s, wr * s + wi * c);
            }
            *out = (10.0 * (xr * xr + xi * xi + 1e-9).log10()) as f32;
        }
        row
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tone mixed like a station `d` Hz off the pitch, with the RIT at `rit`.
    fn tone(d: f64, rit: i32) -> Spectrum {
        let mut s = Spectrum::default();
        let step = TAU * (d - rit as f64) / RATE as f64;
        let mut ph = 0.0f64;
        for _ in 0..N / BLOCK {
            let (mut re, mut im) = ([0f32; BLOCK], [0f32; BLOCK]);
            for i in 0..BLOCK {
                re[i] = 1000.0 * ph.cos() as f32;
                im[i] = -1000.0 * ph.sin() as f32;
                ph += step;
            }
            s.push(&re, &im, rit);
        }
        s
    }

    fn peak(s: &Spectrum) -> f64 {
        let r = s.rows.back().expect("a row");
        let k = (0..BINS).max_by(|&a, &b| r[a].total_cmp(&r[b])).unwrap();
        bin_hz(k)
    }

    #[test]
    fn a_station_shows_where_it_is_whatever_the_rit() {
        let bin = SPAN / BINS as f64;
        for (d, rit) in [(0.0, 0), (230.0, 0), (-410.0, 0), (230.0, 200), (-410.0, -350), (600.0, 500)] {
            let p = peak(&tone(d, rit));
            assert!((p - d).abs() <= bin / 2.0 + 0.1, "station at {d}, RIT {rit}: peak at {p}");
        }
    }

    #[test]
    fn rows_roll_and_a_reset_starts_over() {
        let mut s = Spectrum::default();
        let z = [0f32; BLOCK];
        for _ in 0..(ROWS + 5) * N / BLOCK {
            s.push(&z, &z, 0);
        }
        assert_eq!(s.rows.len(), ROWS);
        assert_eq!(s.made, (ROWS + 5) as u64);
        let e = s.epoch;
        s.reset();
        assert!(s.rows.is_empty());
        assert_eq!(s.made, 0);
        assert_ne!(s.epoch, e);
    }
}
