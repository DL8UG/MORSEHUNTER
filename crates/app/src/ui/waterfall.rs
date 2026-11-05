//! A small waterfall over the passband strip. It shows the band before the filter, on the
//! scale of the strip, so a caller off the pitch shows up before it can be heard. The
//! newest row is at the bottom, next to the strip. A click on a trace tunes the RIT to it.

use std::collections::VecDeque;

use egui::{Color32, ColorImage, Rect, Sense, Stroke, TextureHandle, TextureOptions, Ui, pos2};
use mr_sim::contest::Contest;
use mr_sim::spectrum::{BINS, ROWS, Row, SPAN};

use super::theme::{self, Palette, pal};
use mr_sim::settings::Theme;

/// Decibels over the noise from dark to the brightest lamp.
const RANGE_DB: f32 = 30.0;
/// The bottom of that range, a little over the noise so that it stays dark.
const LIFT_DB: f32 = 4.0;

#[derive(Default)]
pub(super) struct Waterfall {
    tex: Option<TextureHandle>,
    rows: VecDeque<Row>,
    epoch: u32,
    made: u64,
    /// The noise floor in dB, followed slowly.
    floor: Option<f32>,
    dirty: bool,
    /// The theme the texture was drawn in.
    theme: Option<Theme>,
}

impl Waterfall {
    /// Takes the rows made since the last frame.
    pub(super) fn update(&mut self, c: &Contest) {
        let s = &c.spectrum;
        if s.epoch != self.epoch {
            self.epoch = s.epoch;
            self.made = 0;
            self.rows.clear();
            self.floor = None;
            self.dirty = true;
        }
        let new = (s.made.saturating_sub(self.made) as usize).min(s.rows.len());
        self.made = s.made;
        for row in s.rows.iter().skip(s.rows.len() - new) {
            self.follow_floor(row);
            if self.rows.len() == ROWS {
                self.rows.pop_front();
            }
            self.rows.push_back(*row);
            self.dirty = true;
        }
    }

    /// Most bins hold noise only, so the median of a row is the noise.
    fn follow_floor(&mut self, row: &Row) {
        let mut v = *row;
        v.sort_by(f32::total_cmp);
        let m = v[BINS / 2];
        self.floor = Some(match self.floor {
            Some(f) => f + 0.1 * (m - f),
            None => m,
        });
    }

    fn image(&self) -> ColorImage {
        let floor = self.floor.unwrap_or(0.0) + LIFT_DB;
        let p = pal();
        let mut px = vec![p.deep; BINS * ROWS];
        // the newest row at the bottom
        let top = ROWS - self.rows.len();
        for (y, row) in self.rows.iter().enumerate() {
            for (x, db) in row.iter().enumerate() {
                px[(top + y) * BINS + x] = shade(p, (db - floor) / RANGE_DB);
            }
        }
        ColorImage::new([BINS, ROWS], px)
    }

    /// Draws into `r`; a click gives the offset from the pitch in Hz.
    pub(super) fn show(&mut self, ui: &mut Ui, r: Rect, rit: i32, bandwidth: u32) -> Option<f64> {
        let resp = ui.interact(r, ui.scope_id().with("waterfall"), Sense::click());
        if self.dirty || self.tex.is_none() || self.theme != Some(theme::current()) {
            self.theme = Some(theme::current());
            let img = self.image();
            match &mut self.tex {
                Some(t) => t.set(img, TextureOptions::LINEAR),
                None => self.tex = Some(ui.ctx().load_texture("waterfall", img, TextureOptions::LINEAR)),
            }
            self.dirty = false;
        }
        let p = ui.painter();
        if let Some(t) = &self.tex {
            let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
            p.image(t.id(), r, uv, Color32::WHITE);
        }
        // the edges of the passband, as in the strip below
        let px = r.width() / SPAN as f32;
        let c = r.center().x + rit as f32 * px;
        let half = bandwidth as f32 / 2.0 * px;
        let edge = Stroke::new(1.0, pal().glow.gamma_multiply(0.6));
        for x in [c - half, c + half] {
            if x > r.left() && x < r.right() {
                p.line_segment([pos2(x, r.top()), pos2(x, r.bottom())], edge);
            }
        }
        let pos = resp.clicked().then(|| resp.interact_pointer_pos()).flatten()?;
        Some(((pos.x - r.center().x) / px) as f64)
    }
}

/// From the floor to the glow of the lamp, over the colour of the lines.
fn shade(p: &Palette, v: f32) -> Color32 {
    let stops = [(0.0, p.deep), (0.35, p.line), (0.7, mix(p.line, p.glow, 0.5)), (1.0, p.glow)];
    let v = v.clamp(0.0, 1.0);
    for w in stops.windows(2) {
        let ((a, ca), (b, cb)) = (w[0], w[1]);
        if v <= b {
            return mix(ca, cb, (v - a) / (b - a));
        }
    }
    p.glow
}

fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let m = |x: u8, y: u8| (x as f32 + t * (y as f32 - x as f32)).round() as u8;
    Color32::from_rgb(m(a.r(), b.r()), m(a.g(), b.g()), m(a.b(), b.b()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shade_runs_from_the_floor_to_the_lamp() {
        let p = pal();
        assert_eq!(shade(p, -1.0), p.deep);
        assert_eq!(shade(p, 0.0), p.deep);
        assert_eq!(shade(p, 0.35), p.line);
        assert_eq!(shade(p, 1.0), p.glow);
        assert_eq!(shade(p, 2.0), p.glow);
    }
}
