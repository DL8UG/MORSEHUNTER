//! The colours of the panel, one palette per theme. The first is the look of a station
//! receiver at night: a slate front panel, and the three log fields are the lit dial
//! windows on it. Nothing else on the panel is that bright. The others keep those roles:
//! the lamp is whatever marks the field with the cursor, the ink is what is written on it.

use std::sync::atomic::{AtomicU8, Ordering::Relaxed};

use egui::{Color32, Stroke};
use mr_sim::settings::Theme;

const fn c(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

pub struct Palette {
    /// The window.
    pub panel: Color32,
    /// Strips, cards, a raised key.
    pub panel_hi: Color32,
    /// The deepest well: text boxes, the floor of the waterfall.
    pub deep: Color32,
    pub line: Color32,
    pub text: Color32,
    pub muted: Color32,
    /// The field with the cursor.
    pub lamp: Color32,
    /// The other fields, and what is marked.
    pub lamp_dim: Color32,
    /// Writing on lamp and lamp_dim.
    pub ink: Color32,
    /// The lamp as writing and marks on the panel: titles, the best figures, a trace.
    pub glow: Color32,
    /// Less of it: numbers of steps, keys in the help, the bars of the rate.
    pub glow_dim: Color32,
    pub signal: Color32,
    pub good: Color32,
    pub dark: bool,
}

/// The slate receiver at night, as it always was.
const NIGHT: Palette = Palette {
    panel: c(0x22, 0x30, 0x3A),
    panel_hi: c(0x2C, 0x3D, 0x49),
    deep: c(0x1A, 0x25, 0x2C),
    line: c(0x46, 0x5A, 0x66),
    text: c(0xDD, 0xE2, 0xDE),
    muted: c(0x9C, 0xAC, 0xB0),
    lamp: c(0xF3, 0xE4, 0xAE),
    lamp_dim: c(0xA8, 0x9C, 0x6F),
    ink: c(0x2A, 0x24, 0x15),
    glow: c(0xF3, 0xE4, 0xAE),
    glow_dim: c(0xA8, 0x9C, 0x6F),
    signal: c(0xF0, 0x70, 0x48),
    good: c(0x8C, 0xCF, 0xB0),
    dark: true,
};

/// The same receiver by day: a pale slate panel, the dial lamps a warmer amber.
const DAY: Palette = Palette {
    panel: c(0xE4, 0xE9, 0xEA),
    panel_hi: c(0xF2, 0xF5, 0xF5),
    deep: c(0xD3, 0xDB, 0xDE),
    line: c(0xA3, 0xB2, 0xB8),
    text: c(0x1B, 0x28, 0x30),
    muted: c(0x50, 0x60, 0x68),
    lamp: c(0xF2, 0xCF, 0x6B),
    lamp_dim: c(0xD8, 0xCC, 0xA2),
    ink: c(0x2A, 0x24, 0x15),
    glow: c(0x86, 0x5E, 0x0C),
    glow_dim: c(0x72, 0x63, 0x3F),
    signal: c(0xC2, 0x41, 0x24),
    good: c(0x23, 0x73, 0x58),
    dark: false,
};

/// dl8ug.de and its PDFs: paper, the green of radio enamel, olive, sage, the TX lamp red.
/// The lit field is the enamel with paper writing, as the CW bars of the band plans.
const DL8UG_LIGHT: Palette = Palette {
    panel: c(0xF6, 0xF7, 0xF3),
    panel_hi: c(0xE9, 0xEC, 0xE4),
    deep: c(0xDF, 0xE3, 0xD8),
    line: c(0xBF, 0xC6, 0xB8),
    text: c(0x1F, 0x2A, 0x24),
    muted: c(0x5F, 0x67, 0x5C),
    lamp: c(0x1F, 0x2A, 0x24),
    lamp_dim: c(0x6B, 0x74, 0x67),
    ink: c(0xF6, 0xF7, 0xF3),
    glow: c(0x1F, 0x2A, 0x24),
    glow_dim: c(0x3E, 0x45, 0x36),
    signal: c(0xC8, 0x36, 0x1F),
    good: c(0x2F, 0x7D, 0x4F),
    dark: false,
};

/// The dark mode of dl8ug.de: the lit field is paper, the others olive.
const DL8UG_DARK: Palette = Palette {
    panel: c(0x14, 0x19, 0x15),
    panel_hi: c(0x1E, 0x25, 0x20),
    deep: c(0x0E, 0x12, 0x0F),
    line: c(0x3A, 0x45, 0x3C),
    text: c(0xE6, 0xEA, 0xE1),
    muted: c(0x9A, 0xA3, 0x94),
    lamp: c(0xE6, 0xEA, 0xE1),
    lamp_dim: c(0xB9, 0xC2, 0xAD),
    ink: c(0x14, 0x19, 0x15),
    glow: c(0xE6, 0xEA, 0xE1),
    glow_dim: c(0xB9, 0xC2, 0xAD),
    signal: c(0xEC, 0x6A, 0x4F),
    good: c(0x6C, 0xC0, 0x8B),
    dark: true,
};

/// For weak eyes: black, white and yellow, nothing in between that has to be told apart.
const HIGH_CONTRAST: Palette = Palette {
    panel: c(0x00, 0x00, 0x00),
    panel_hi: c(0x1A, 0x1A, 0x1A),
    deep: c(0x00, 0x00, 0x00),
    line: c(0xC8, 0xC8, 0xC8),
    text: c(0xFF, 0xFF, 0xFF),
    muted: c(0xE0, 0xE0, 0xE0),
    lamp: c(0xFF, 0xF0, 0x00),
    lamp_dim: c(0xFF, 0xFF, 0xFF),
    ink: c(0x00, 0x00, 0x00),
    glow: c(0xFF, 0xF0, 0x00),
    glow_dim: c(0xD0, 0xD0, 0xD0),
    signal: c(0xFF, 0x60, 0x40),
    good: c(0x40, 0xFF, 0x90),
    dark: true,
};

fn of(t: Theme) -> &'static Palette {
    match t {
        Theme::Night => &NIGHT,
        Theme::Day => &DAY,
        Theme::Dl8ugLight => &DL8UG_LIGHT,
        Theme::Dl8ugDark => &DL8UG_DARK,
        Theme::HighContrast => &HIGH_CONTRAST,
    }
}

static CURRENT: AtomicU8 = AtomicU8::new(0);

/// The theme in use.
pub fn current() -> Theme {
    Theme::ALL[CURRENT.load(Relaxed) as usize]
}

/// The palette in use.
pub fn pal() -> &'static Palette {
    of(current())
}

/// Switches to a theme, true if it was another one.
pub fn select(ctx: &egui::Context, t: Theme) -> bool {
    if t == current() {
        return false;
    }
    let i = Theme::ALL.iter().position(|&x| x == t).unwrap_or(0);
    CURRENT.store(i as u8, Relaxed);
    apply(ctx);
    true
}

pub fn apply(ctx: &egui::Context) {
    let p = pal();
    let mut v = if p.dark { egui::Visuals::dark() } else { egui::Visuals::light() };
    v.panel_fill = p.panel;
    v.window_fill = p.panel_hi;
    v.extreme_bg_color = p.deep;
    v.faint_bg_color = p.panel_hi;
    // not override_text_color: that would write on a selected tab in the panel colour too
    for w in [&mut v.widgets.noninteractive, &mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.open] {
        w.fg_stroke.color = p.text;
    }
    v.selection.bg_fill = p.lamp_dim;
    v.selection.stroke.color = p.ink;
    v.widgets.inactive.weak_bg_fill = p.panel_hi;
    v.widgets.inactive.bg_fill = if p.dark { p.panel_hi } else { p.deep };
    v.widgets.hovered.weak_bg_fill = p.line;
    v.widgets.active.weak_bg_fill = p.line;
    v.widgets.noninteractive.bg_stroke.color = p.line;
    if current() == Theme::HighContrast {
        // every control outlined, the hovered one in the lamp colour
        let w = &mut v.widgets;
        w.noninteractive.bg_stroke = Stroke::new(1.0, p.line);
        w.inactive.bg_stroke = Stroke::new(1.0, p.text);
        w.inactive.fg_stroke = Stroke::new(1.5, p.text);
        w.hovered.bg_stroke = Stroke::new(2.0, p.lamp);
        w.hovered.fg_stroke = Stroke::new(2.0, p.lamp);
        w.active.bg_stroke = Stroke::new(2.0, p.lamp);
        w.active.fg_stroke = Stroke::new(2.0, p.lamp);
        v.selection.bg_fill = p.lamp;
        v.selection.stroke = Stroke::new(2.0, p.ink);
    }
    ctx.set_visuals(v);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// WCAG contrast of two colours.
    fn contrast(a: Color32, b: Color32) -> f32 {
        let lum = |c: Color32| {
            let ch = |v: u8| {
                let s = v as f32 / 255.0;
                if s <= 0.03928 { s / 12.92 } else { ((s + 0.055) / 1.055).powf(2.4) }
            };
            0.2126 * ch(c.r()) + 0.7152 * ch(c.g()) + 0.0722 * ch(c.b())
        };
        let (x, y) = (lum(a), lum(b));
        (x.max(y) + 0.05) / (x.min(y) + 0.05)
    }

    #[test]
    fn every_theme_can_be_read() {
        for t in Theme::ALL {
            let p = of(t);
            let need = if t == Theme::HighContrast { 7.0 } else { 4.5 };
            for (what, fg, bg) in [
                ("text", p.text, p.panel),
                ("text on a strip", p.text, p.panel_hi),
                ("ink on the lamp", p.ink, p.lamp),
                ("muted", p.muted, p.panel),
                ("glow", p.glow, p.panel),
                ("glow on a card", p.glow, p.panel_hi),
                ("glow_dim", p.glow_dim, p.panel),
            ] {
                let k = contrast(fg, bg);
                assert!(k >= need, "{t:?}: {what} has contrast {k:.1}, needs {need}");
            }
            // the big digits in the other fields
            let k = contrast(p.ink, p.lamp_dim);
            assert!(k >= 3.0, "{t:?}: ink on lamp_dim has {k:.1}");
            // the lamp stands out from the fields beside it
            assert!(p.lamp != p.lamp_dim, "{t:?}");
        }
    }

    #[test]
    fn select_switches_and_says_so() {
        let ctx = egui::Context::default();
        assert!(select(&ctx, Theme::HighContrast));
        assert!(!select(&ctx, Theme::HighContrast));
        assert_eq!(pal().panel, HIGH_CONTRAST.panel);
        assert!(select(&ctx, Theme::Night));
        assert_eq!(pal().panel, NIGHT.panel);
    }
}
