//! The log line: three lit windows. The one with the cursor burns at full brightness.

use egui::{Align2, FontFamily, FontId, Pos2, Rect, Sense, Stroke, Ui, Vec2, pos2};

use super::theme::*;
use mr_sim::entry::Edit;

pub const HEIGHT: f32 = 76.0;
const SIZE: f32 = 46.0;

pub struct Window<'a> {
    pub id: &'static str,
    pub label: &'a str,
    pub edit: &'a Edit,
    pub focused: bool,
    pub lit: bool,
    pub alarm: bool,
}

/// Draws one field, true if it was clicked.
pub fn field(ui: &mut Ui, rect: Rect, w: &Window) -> bool {
    let p = ui.painter().clone();
    let font = FontId::new(SIZE, FontFamily::Monospace);
    let fill = match (w.lit, w.focused, w.alarm) {
        (_, _, true) => pal().signal,
        (true, true, _) => pal().lamp,
        (true, false, _) => pal().lamp_dim,
        (false, ..) => pal().panel_hi,
    };
    let ink = if w.lit { pal().ink } else { pal().muted };
    p.rect_filled(rect, 3.0, fill);
    let x = rect.left() + 14.0;
    let y = rect.center().y;
    let galley = |t: &str| p.layout_no_wrap(t.to_string(), font.clone(), ink);
    let width = |t: &str| galley(t).size().x;
    let cursor_x = x + width(&w.edit.text[..w.edit.cursor]);
    if w.focused && w.edit.sel > 0 {
        let end = x + width(&w.edit.text[..w.edit.cursor + w.edit.sel]);
        let r = Rect::from_min_max(pos2(cursor_x, rect.top() + 10.0), pos2(end, rect.bottom() - 10.0));
        p.rect_filled(r, 0.0, pal().lamp_dim);
    }
    p.text(Pos2::new(x, y), Align2::LEFT_CENTER, &w.edit.text, font.clone(), ink);
    if w.edit.text.is_empty() {
        p.text(Pos2::new(x, y), Align2::LEFT_CENTER, w.label, FontId::proportional(18.0), if w.lit { pal().ink.gamma_multiply(0.6) } else { pal().muted });
    }
    if w.focused && w.lit && w.edit.sel == 0 {
        p.line_segment([pos2(cursor_x, rect.top() + 12.0), pos2(cursor_x, rect.bottom() - 12.0)], Stroke::new(2.5, pal().ink));
    }
    ui.interact(rect, ui.scope_id().with(w.id), Sense::click()).clicked()
}

/// A key with its function, the F number at the side.
pub fn keycap(ui: &mut Ui, rect: Rect, id: u8, key: &str, text: &str, enabled: bool) -> bool {
    let r = ui.interact(rect, ui.scope_id().with(("keycap", id)), if enabled { Sense::click() } else { Sense::hover() });
    let p = ui.painter();
    let fill = if r.is_pointer_button_down_on() {
        pal().line
    } else if r.hovered() && enabled {
        pal().panel_hi
    } else {
        pal().panel
    };
    p.rect_filled(rect, 2.0, fill);
    p.rect_stroke(rect, 2.0, Stroke::new(1.0, pal().line), egui::StrokeKind::Inside);
    let color = if enabled { pal().text } else { pal().muted };
    // too narrow for both: the key is what the hand looks for
    if rect.width() < 84.0 {
        p.text(rect.center(), Align2::CENTER_CENTER, key, FontId::monospace(12.0), color);
    } else {
        p.text(rect.left_center() + Vec2::new(8.0, 0.0), Align2::LEFT_CENTER, key, FontId::monospace(11.0), pal().muted);
        // longer words of some languages get a smaller type, not over the key
        let room = rect.width() - 30.0;
        let wide = p.layout_no_wrap(text.to_string(), FontId::proportional(14.0), color).size().x;
        let size = if wide > room { (14.0 * room / wide).max(10.0) } else { 14.0 };
        p.text(rect.center() + Vec2::new(11.0, 0.0), Align2::CENTER_CENTER, text, FontId::proportional(size), color);
    }
    r.clicked()
}
