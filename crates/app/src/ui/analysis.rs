//! What the operator can learn from a run: the copying errors and how the paddle was keyed,
//! and the skill over the days.

use egui::{Align2, FontId, RichText, Sense, Stroke, Ui, pos2, vec2};
use mr_sim::analysis::{Analysis, MissKind};
use mr_sim::fist::{Advice, Fist, Stat};
use mr_sim::settings::Lang;

use super::main_view::{clock, short_date};
use super::theme::*;
use super::{View, tr};
use crate::store::ScoreLine;

/// One decimal, with the decimal comma where the language has one.
fn num(v: f64, lang: Lang) -> String {
    let s = format!("{v:.1}");
    if lang == Lang::En { s } else { s.replace('.', ",") }
}

fn heading(ui: &mut Ui, text: &str) {
    ui.add_space(6.0);
    ui.label(RichText::new(text).color(pal().glow).size(15.0));
}

pub(super) fn analysis_tab(ui: &mut Ui, v: &View) {
    let lang = v.settings.lang;
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        copying(ui, &v.analysis, lang);
        keying(ui, &v.fist, lang);
    });
}

fn copying(ui: &mut Ui, a: &Analysis, lang: Lang) {
    heading(ui, tr(lang, "Copying", "Abschreiben"));
    if a.qsos == 0 {
        ui.label(RichText::new(tr(lang, "Log a few contacts and your copying errors show here.", "Logge ein paar QSOs, dann stehen hier deine Abschreibfehler.")).color(pal().muted));
        return;
    }
    let mut line = format!("{} QSOs, {} {}, {} {}", a.qsos, a.good, tr(lang, "clean", "sauber"), a.misses.len(), tr(lang, "with an error", "mit Fehler"));
    if a.dupes > 0 {
        line += &format!(", {} {}", a.dupes, tr(lang, "dupes", "Dupes"));
    }
    ui.label(RichText::new(line).color(pal().text));
    if a.misses.is_empty() {
        ui.label(RichText::new(tr(lang, "No copying error so far.", "Bisher kein Abschreibfehler.")).color(pal().good));
        return;
    }
    ui.add_space(4.0);
    for m in &a.misses {
        let kind = match m.kind {
            MissKind::Call | MissKind::Unknown => "Call",
            MissKind::Rst => "RST",
            MissKind::Nr => "NR",
        };
        let truth = if m.truth.is_empty() { tr(lang, "no caller with a call like it", "kein Anrufer mit ähnlichem Rufzeichen").to_string() } else { format!("{} {}", tr(lang, "instead of", "statt"), m.truth) };
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.label(RichText::new(format!("{:<9} {:<5} {:<12} ", clock(m.t), kind, m.logged)).monospace().size(14.0).color(pal().text));
            ui.label(RichText::new(truth).size(14.0).color(pal().muted));
        });
    }
    if a.confusions.is_empty() {
        return;
    }
    ui.add_space(6.0);
    ui.label(RichText::new(tr(lang, "Mixed up", "Verwechselt")).color(pal().muted));
    ui.horizontal_wrapped(|ui| {
        for c in a.confusions.iter().take(12) {
            let text = match (c.sent, c.written) {
                (Some(s), Some(w)) => format!("{s} → {w}"),
                (Some(s), None) => format!("{s} {}", tr(lang, "missing", "fehlt")),
                (None, Some(w)) => format!("{w} {}", tr(lang, "too many", "zu viel")),
                (None, None) => continue,
            };
            let chip = egui::Frame::new().fill(pal().panel_hi).corner_radius(4.0).inner_margin(vec2(8.0, 3.0));
            chip.show(ui, |ui| {
                ui.label(RichText::new(format!("{text}  {}×", c.n)).monospace().size(14.0).color(pal().text));
            });
        }
    });
}

fn keying(ui: &mut Ui, f: &Fist, lang: Lang) {
    heading(ui, tr(lang, "Keying", "Gebeweise"));
    if f.chars == 0 {
        ui.label(RichText::new(tr(lang, "Key a run with the paddle and this shows how clean your rhythm is.", "Gib einen Lauf mit dem Paddle, dann steht hier, wie sauber dein Rhythmus ist.")).color(pal().muted));
        return;
    }
    if !f.by_hand {
        ui.label(RichText::new(tr(lang, "With a keyer only the pauses between letters and words are yours.", "Mit Keyer sind nur die Pausen zwischen Buchstaben und Wörtern deine eigenen.")).color(pal().muted).size(13.0));
    }
    let dits = tr(lang, "dits", "Punkte");
    let book = tr(lang, "book", "Soll");
    let row = |ui: &mut Ui, label: &str, value: String| {
        ui.horizontal(|ui| {
            ui.allocate_ui_with_layout(vec2(190.0, 18.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.set_min_width(190.0);
                ui.label(RichText::new(label).color(pal().muted));
            });
            ui.label(RichText::new(value).color(pal().text));
        });
    };
    let measured = |s: &Stat, want: f64| s.mean().map(|m| format!("{} {dits} ({book} {})", num(m, lang), num(want, lang).trim_end_matches(",0").trim_end_matches(".0")));
    if f.by_hand {
        if let Some(t) = measured(&f.dits, 1.0) {
            row(ui, tr(lang, "Dit", "Punkt"), t);
        }
        if let Some(t) = measured(&f.dahs, 3.0) {
            row(ui, tr(lang, "Dah", "Strich"), t);
        }
        if let Some(w) = f.weight() {
            row(ui, tr(lang, "Dah to dit", "Strich zu Punkt"), format!("{} ({book} 3)", num(w, lang)));
        }
        if let Some(t) = measured(&f.element_gaps, 1.0) {
            row(ui, tr(lang, "Gap inside a letter", "Abstand im Zeichen"), t);
        }
        if let Some(s) = f.dits.spread().into_iter().chain(f.dahs.spread()).reduce(f64::max) {
            row(ui, tr(lang, "Steadiness", "Gleichmäßigkeit"), format!("± {:.0} %", s * 100.0));
        }
    }
    if let Some(t) = measured(&f.letter_gaps, 3.0) {
        row(ui, tr(lang, "Letter gap", "Buchstabenabstand"), t);
    }
    if let Some(t) = measured(&f.word_gaps, 7.0) {
        row(ui, tr(lang, "Word gap", "Wortabstand"), t);
    }
    row(ui, tr(lang, "Unreadable", "Unlesbar"), format!("{} {} {} {}", f.unknown, tr(lang, "of", "von"), f.chars, tr(lang, "characters", "Zeichen")));
    if f.errors > 0 {
        row(ui, tr(lang, "Error signs", "Irrungen"), f.errors.to_string());
    }
    ui.add_space(4.0);
    let advice = f.advice();
    if advice.is_empty() {
        ui.label(RichText::new(tr(lang, "Clean keying, nothing stands out.", "Saubere Gebeweise, nichts fällt auf.")).color(pal().good));
    }
    for a in advice {
        let text = match a {
            Advice::LettersTight(_) => tr(lang, "Your letters run together. Leave more room between them.", "Deine Buchstaben laufen ineinander. Lass mehr Platz zwischen ihnen."),
            Advice::LettersWide(_) => tr(lang, "Your letters stand far apart. That makes the over slow.", "Deine Buchstaben stehen weit auseinander. Das macht den Durchgang langsam."),
            Advice::WordsLikeLetters(..) => tr(lang, "Your words hardly stand apart from the letters. Pause longer after a word.", "Deine Wörter heben sich kaum von den Buchstaben ab. Mach nach einem Wort eine längere Pause."),
            Advice::Weight(r) if r < 3.0 => tr(lang, "Your dahs are short against the dits.", "Deine Striche sind im Verhältnis zu den Punkten zu kurz."),
            Advice::Weight(_) => tr(lang, "Your dahs are long against the dits.", "Deine Striche sind im Verhältnis zu den Punkten zu lang."),
            Advice::Unsteady(_) => tr(lang, "Your dits and dahs vary a lot in length. Aim for an even hand.", "Deine Punkte und Striche schwanken stark in der Länge. Versuch eine gleichmäßige Hand."),
            Advice::Unreadable(..) => tr(lang, "Many characters were no Morse character. Watch the gaps inside the letters.", "Viele Zeichen waren kein Morsezeichen. Achte auf die Abstände innerhalb der Buchstaben."),
        };
        ui.label(RichText::new(format!("• {text}")).color(pal().signal));
    }
}

/// Days since 1970 of a date like 2026-11-03.
fn day_no(date: &str) -> Option<i64> {
    let y: i64 = date.get(0..4)?.parse().ok()?;
    let m: i64 = date.get(5..7)?.parse().ok()?;
    let d: i64 = date.get(8..10)?.parse().ok()?;
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146097 + doe - 719468)
}

/// The best skill per hour of each day with runs, the oldest first: date, best, runs.
fn by_day(scores: &[ScoreLine]) -> Vec<(String, u64, u32)> {
    let mut days: Vec<(String, u64, u32)> = Vec::new();
    for s in scores {
        let Some(k) = &s.skill else { continue };
        match days.iter_mut().find(|d| d.0 == s.date) {
            Some(d) => {
                d.1 = d.1.max(k.per_hour);
                d.2 += 1;
            }
            None => days.push((s.date.clone(), k.per_hour, 1)),
        }
    }
    days.sort_by(|a, b| a.0.cmp(&b.0));
    days
}

/// A step for about four grid lines up to `top`: 1, 2 or 5 times a power of ten.
fn nice_step(top: f64) -> f64 {
    let raw = (top / 4.0).max(1.0);
    let p = 10f64.powf(raw.log10().floor());
    [1.0, 2.0, 5.0, 10.0].into_iter().map(|m| m * p).find(|s| *s >= raw).unwrap_or(10.0 * p)
}

pub(super) fn progress(ui: &mut Ui, scores: &[ScoreLine], lang: Lang) {
    let days = by_day(scores);
    let per_hour = tr(lang, "Pts/h", "Pkt/h");
    ui.add_space(6.0);
    ui.label(RichText::new(tr(lang, "Best skill points per hour of each day", "Beste Leistungspunkte pro Stunde je Tag")).color(pal().text));
    if days.len() < 2 {
        // one day is a number, not a curve yet
        if let Some((date, best, runs)) = days.first() {
            ui.add_space(8.0);
            ui.label(RichText::new(format!("{best} {per_hour}")).color(pal().glow).size(28.0));
            ui.label(RichText::new(format!("{}, {runs} {}", short_date(date, lang), tr(lang, "runs", "Läufe"))).color(pal().muted));
        }
        ui.add_space(8.0);
        ui.label(RichText::new(tr(lang, "A few days of runs and your progress shows here.", "Nach ein paar Tagen mit Läufen siehst du hier deinen Fortschritt.")).color(pal().muted));
        return;
    }
    let pts: Vec<(i64, u64)> = days.iter().filter_map(|(d, b, _)| Some((day_no(d)?, *b))).collect();
    let (first, last) = (pts[0].0, pts[pts.len() - 1].0);
    let top = pts.iter().map(|p| p.1).max().unwrap_or(1) as f64 * 1.1;
    let step = nice_step(top);
    let top = (top / step).ceil() * step;
    let width = ui.available_width().min(760.0);
    let (resp, painter) = ui.allocate_painter(vec2(width, 250.0), Sense::hover());
    let r = resp.rect;
    let plot = egui::Rect::from_min_max(pos2(r.left() + 46.0, r.top() + 26.0), pos2(r.right() - 12.0, r.bottom() - 24.0));
    let x = |day: i64| if last == first { plot.center().x } else { plot.left() + (day - first) as f32 / (last - first) as f32 * plot.width() };
    let y = |v: f64| plot.bottom() - (v / top) as f32 * plot.height();
    let small = FontId::proportional(12.0);
    // a recessive grid with its values on the left
    let mut v = 0.0;
    while v <= top + 0.5 {
        let yy = y(v);
        painter.line_segment([pos2(plot.left(), yy), pos2(plot.right(), yy)], Stroke::new(1.0, pal().line.gamma_multiply(if v == 0.0 { 1.0 } else { 0.45 })));
        painter.text(pos2(plot.left() - 8.0, yy), Align2::RIGHT_CENTER, format!("{v:.0}"), small.clone(), pal().muted);
        v += step;
    }
    painter.text(pos2(plot.left() - 8.0, r.top() + 2.0), Align2::RIGHT_TOP, per_hour, small.clone(), pal().muted);
    // dates under the first, the last and, with room, the middle day
    let mut ticks = vec![0, days.len() - 1];
    if days.len() > 4 {
        ticks.insert(1, days.len() / 2);
    }
    for i in ticks {
        if let Some(d) = day_no(&days[i].0) {
            // the outer ones flush with the edges of the plot, so they stay inside
            let align = if i == 0 { Align2::LEFT_TOP } else if i == days.len() - 1 { Align2::RIGHT_TOP } else { Align2::CENTER_TOP };
            painter.text(pos2(x(d), plot.bottom() + 6.0), align, short_date(&days[i].0, lang), small.clone(), pal().muted);
        }
    }
    let line: Vec<egui::Pos2> = pts.iter().map(|(d, b)| pos2(x(*d), y(*b as f64))).collect();
    painter.add(egui::Shape::line(line.clone(), Stroke::new(2.0, pal().glow)));
    for p in &line {
        // a ring in the panel colour keeps the dot apart from the line
        painter.circle(*p, 4.0, pal().glow, Stroke::new(2.0, pal().panel));
    }
    // the day under the pointer: a crosshair and its numbers
    if let Some(at) = resp.hover_pos() {
        let near = line.iter().enumerate().min_by(|a, b| (a.1.x - at.x).abs().total_cmp(&(b.1.x - at.x).abs()));
        if let Some((i, p)) = near
            && (p.x - at.x).abs() < 24.0
        {
            painter.line_segment([pos2(p.x, plot.top()), pos2(p.x, plot.bottom())], Stroke::new(1.0, pal().muted.gamma_multiply(0.6)));
            painter.circle(*p, 6.0, pal().glow, Stroke::new(2.0, pal().panel));
            let (date, best, runs) = &days[i];
            resp.clone().on_hover_ui_at_pointer(|ui| {
                ui.label(RichText::new(short_date(date, lang)).color(pal().muted));
                ui.label(RichText::new(format!("{best} {per_hour}")).color(pal().text).strong());
                ui.label(RichText::new(format!("{runs} {}", tr(lang, "runs", "Läufe"))).color(pal().muted));
            });
        }
    }
    // the same numbers as text, for those who rather read
    let best = days.iter().max_by_key(|d| d.1).map(|d| (d.1, short_date(&d.0, lang)));
    let runs: u32 = days.iter().map(|d| d.2).sum();
    if let Some((b, on)) = best {
        ui.label(RichText::new(format!("{} {b} {per_hour}, {on}  ·  {runs} {} / {} {}", tr(lang, "Best day", "Bester Tag"), tr(lang, "runs", "Läufe"), days.len(), tr(lang, "days", "Tage"))).color(pal().muted).size(13.0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn day_numbers() {
        assert_eq!(day_no("1970-01-01"), Some(0));
        assert_eq!(day_no("2026-11-03").zip(day_no("2026-11-01")).map(|(a, b)| a - b), Some(2));
        assert_eq!(day_no("2024-03-01").zip(day_no("2024-02-28")).map(|(a, b)| a - b), Some(2), "leap day");
        assert_eq!(day_no("junk"), None);
    }

    #[test]
    fn steps_are_round() {
        assert_eq!(nice_step(400.0), 100.0);
        assert_eq!(nice_step(130.0), 50.0);
        assert_eq!(nice_step(7.0), 2.0);
    }
}
