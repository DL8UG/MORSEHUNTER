//! The operating position: time and rate, the log line, the message keys, the log.

use std::time::Instant;

use egui::{Align2, Color32, FontId, Rect, RichText, Sense, Stroke, Ui, UiBuilder, pos2, vec2};
use mr_sim::contest::RunMode;
use mr_sim::entry::{Field, Key};
use mr_sim::log::HISTO_BINS;
use mr_sim::settings::Lang;
use mr_sim::spectrum::SPAN;

use super::fields::{self, HEIGHT, Window};
use super::theme::*;
use super::{App, Board, Tab, View, tr};
use crate::store::ScoreLine;

const HEADER: f32 = 74.0;
const KEY_H: f32 = 34.0;
/// The strip with run and score over the log, and the least width for its two columns.
const DASH_H: f32 = 112.0;
const DASH_MIN_W: f32 = 520.0;
/// The score table shows this many places.
const PLACES: usize = 100;

pub(super) fn clock(secs: f64) -> String {
    let s = secs as u64;
    format!("{:02}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
}

fn mode_name(mode: RunMode, lang: Lang) -> &'static str {
    match mode {
        RunMode::Stop => tr(lang, "Stopped", "Gestoppt"),
        RunMode::Pileup => "Pile-Up",
        RunMode::Single => tr(lang, "Single calls", "Einzelanrufe"),
        RunMode::Wpx => tr(lang, "WPX competition", "WPX-Wettbewerb"),
        RunMode::Hst => tr(lang, "HST competition", "HST-Wettbewerb"),
    }
}

impl App {
    pub(super) fn main_view(&mut self, ui: &mut Ui, area: Rect, v: &View) {
        let lang = v.settings.lang;
        let header = Rect::from_min_size(area.min, vec2(area.width(), HEADER));
        self.header(ui, header, v);

        let fields_top = header.bottom() + 8.0;
        let row = Rect::from_min_size(pos2(area.left(), fields_top), vec2(area.width(), HEIGHT));
        self.log_line(ui, row, v);

        let keys_top = row.bottom() + 10.0;
        self.message_keys(ui, Rect::from_min_size(pos2(area.left(), keys_top), vec2(area.width(), KEY_H)), v);

        let mut top = keys_top + KEY_H + 6.0;
        if v.settings.paddle {
            let line = if v.keying.is_empty() { tr(lang, "Keyed text shows here", "Hier steht der gekeyte Text") } else { &v.keying };
            let color = if v.keying.is_empty() { pal().muted } else { pal().glow };
            ui.painter().text(pos2(area.left(), top + 9.0), Align2::LEFT_CENTER, line, FontId::monospace(15.0), color);
            top += 22.0;
        }
        top += 8.0;

        let lower = Rect::from_min_max(pos2(area.left(), top), area.max);
        if lower.width() >= DASH_MIN_W && lower.height() >= DASH_H + 160.0 {
            // run and score in two columns over the full width log
            let dash = Rect::from_min_size(lower.min, vec2(lower.width(), DASH_H));
            self.dashboard(ui, dash, v);
            let line = dash.bottom() + 6.0;
            ui.painter().line_segment([pos2(lower.left(), line), pos2(lower.right(), line)], Stroke::new(1.0, pal().line));
            self.log_area(ui, Rect::from_min_max(pos2(lower.left(), line + 8.0), lower.max), v);
        } else {
            // no room: the buttons under the log, the score is in the header
            let y = lower.bottom() - 40.0;
            self.log_area(ui, Rect::from_min_max(lower.min, pos2(lower.right(), y - 6.0)), v);
            self.run_controls(ui, Rect::from_min_max(pos2(lower.left(), y), lower.max), v, false);
        }
    }

    fn header(&self, ui: &mut Ui, r: Rect, v: &View) {
        let p = ui.painter();
        let lang = v.settings.lang;
        let on = v.mode != RunMode::Stop;
        let (x, y) = (r.left(), r.center().y);
        let big = FontId::monospace(44.0);
        let color = if on { pal().text } else { pal().muted };
        let w = p.text(pos2(x, y), Align2::LEFT_CENTER, clock(v.secs), big.clone(), color).width();

        let mut x = x + w + 28.0;
        if r.width() >= 660.0 {
            p.text(pos2(x, y - 13.0), Align2::LEFT_CENTER, format!("{}", v.rate), FontId::monospace(24.0), color);
            p.text(pos2(x, y + 13.0), Align2::LEFT_CENTER, tr(lang, "QSOs per hour", "QSOs pro Stunde"), FontId::proportional(13.0), pal().muted);
            x += 128.0;
        }
        let name = mode_name(v.mode, lang);
        let mode_color = if v.mode.competition() { pal().signal } else if on { pal().good } else { pal().muted };
        p.text(pos2(x, y - 13.0), Align2::LEFT_CENTER, name, FontId::proportional(19.0), mode_color);
        let sub = if on {
            let what = match v.mode {
                RunMode::Pileup => format!("{}: {}", tr(lang, "Calling", "Anrufer"), v.dx),
                _ => format!("{} min", v.settings.duration),
            };
            what + if v.fixed { tr(lang, ", fixed by code", ", fest per Code") } else { "" }
        } else {
            tr(lang, "F9 or Alt-R starts a pile-up", "F9 oder Alt-R startet ein Pile-Up").to_string()
        };
        p.text(pos2(x, y + 13.0), Align2::LEFT_CENTER, sub, FontId::proportional(13.0), pal().muted);

        let right = r.right();
        if v.unscored {
            // what is sent is on show: no points, and it says why
            p.text(pos2(right, y - 13.0), Align2::RIGHT_CENTER, tr(lang, "No score", "Keine Wertung"), FontId::proportional(22.0), pal().signal);
            let why = if v.settings.debug { tr(lang, "debug is on", "Debug ist an") } else { tr(lang, "debug was on", "Debug war an") };
            p.text(pos2(right, y + 15.0), Align2::RIGHT_CENTER, why, FontId::proportional(13.0), pal().muted);
            return;
        }
        let score = if v.mode == RunMode::Hst { v.hst.1 } else { v.verified.score };
        p.text(pos2(right, y - 13.0), Align2::RIGHT_CENTER, format!("{score}"), FontId::monospace(30.0), if on { pal().text } else { pal().muted });
        p.text(pos2(right, y + 15.0), Align2::RIGHT_CENTER, tr(lang, "verified score", "geprüfte Punkte"), FontId::proportional(13.0), pal().muted);
    }

    fn log_line(&mut self, ui: &mut Ui, r: Rect, v: &View) {
        let lang = v.settings.lang;
        let lit = v.mode != RunMode::Stop;
        let alarm = self.flash_until.is_some_and(|t| Instant::now() < t);
        let gap = 10.0;
        let info = 170.0;
        let free = (r.width() - info - 3.0 * gap).max(220.0);
        let widths = [free * 0.56, free * 0.2, free * 0.24];
        let list = [
            (Field::Call, "call", tr(lang, "Call", "Rufzeichen"), &v.call),
            (Field::Rst, "rst", "RST", &v.rst),
            (Field::Nr, "nr", "Nr", &v.nr),
        ];
        let mut x = r.left();
        for (i, (f, id, label, edit)) in list.into_iter().enumerate() {
            let rect = Rect::from_min_size(pos2(x, r.top()), vec2(widths[i], HEIGHT));
            let w = Window { id, label, edit, focused: v.focus == f, lit, alarm };
            if fields::field(ui, rect, &w) {
                self.contest.lock().unwrap().entry.set_focus(f);
            }
            x += widths[i] + gap;
        }

        let info_rect = Rect::from_min_max(pos2(x, r.top()), r.right_bottom());
        let p = ui.painter().clone();
        let cx = info_rect.left();
        let y = info_rect.top() + 9.0;
        p.text(pos2(cx, y), Align2::LEFT_CENTER, tr(lang, "Your number", "Deine Nummer"), FontId::proportional(13.0), pal().muted);
        p.text(pos2(info_rect.right(), y), Align2::RIGHT_CENTER, format!("{:03}", v.next_nr), FontId::monospace(20.0), if lit { pal().text } else { pal().muted });
        let strip = Rect::from_min_max(pos2(cx, info_rect.bottom() - 18.0), info_rect.right_bottom());
        let fall = Rect::from_min_max(pos2(cx, info_rect.top() + 21.0), pos2(info_rect.right(), strip.top() - 2.0));
        self.waterfall(ui, fall, v);
        self.rit_strip(ui, strip, v);
    }

    /// The band before the filter over the strip, a click tunes onto a trace.
    fn waterfall(&mut self, ui: &mut Ui, r: Rect, v: &View) {
        self.waterfall.update(&self.contest.lock().unwrap());
        if let Some(hz) = self.waterfall.show(ui, r, v.rit, v.settings.bandwidth) {
            self.contest.lock().unwrap().tune_to(hz);
        }
    }

    /// The passband as a window on a scale of ±800 Hz around the pitch, click to tune.
    fn rit_strip(&mut self, ui: &mut Ui, r: Rect, v: &View) {
        let resp = ui.interact(r, ui.scope_id().with("rit"), Sense::click());
        let p = ui.painter();
        p.rect_filled(r, 2.0, pal().panel_hi);
        let span = SPAN as f32;
        let px = r.width() / span;
        let c = r.center().x + v.rit as f32 * px;
        let half = v.settings.bandwidth as f32 / 2.0 * px;
        let band = Rect::from_min_max(pos2(c - half, r.top() + 2.0), pos2(c + half, r.bottom() - 2.0));
        p.rect_filled(band, 1.0, if v.rit == 0 { pal().line } else { pal().glow_dim });
        p.line_segment([pos2(r.center().x, r.top()), pos2(r.center().x, r.bottom())], Stroke::new(1.0, pal().muted));
        if v.rit != 0 {
            p.text(r.right_center() - vec2(4.0, 0.0), Align2::RIGHT_CENTER, format!("{:+}", v.rit), FontId::monospace(11.0), pal().text);
        }
        if resp.clicked()
            && let Some(pos) = resp.interact_pointer_pos()
        {
            if pos.x < band.left() {
                self.key(Key::Rit(-1));
            } else if pos.x > band.right() {
                self.key(Key::Rit(1));
            } else {
                self.contest.lock().unwrap().rit_zero();
            }
        }
    }

    fn message_keys(&mut self, ui: &mut Ui, r: Rect, v: &View) {
        let lang = v.settings.lang;
        let on = v.mode != RunMode::Stop;
        let keys = [
            "CQ",
            tr(lang, "Exchange", "Report"),
            "TU",
            tr(lang, "My call", "Mein Call"),
            tr(lang, "His call", "Sein Call"),
            "QSO B4",
            "?",
            "NIL",
        ];
        let gap = 6.0;
        let w = (r.width() - 7.0 * gap) / 8.0;
        for (i, text) in keys.into_iter().enumerate() {
            let rect = Rect::from_min_size(r.min + vec2(i as f32 * (w + gap), 0.0), vec2(w, KEY_H));
            if fields::keycap(ui, rect, i as u8, &format!("F{}", i + 1), text, on) {
                self.key(Key::F(i as u8 + 1));
            }
        }
    }

    fn log_area(&mut self, ui: &mut Ui, r: Rect, v: &View) {
        let lang = v.settings.lang;
        ui.scope_builder(UiBuilder::new().max_rect(r), |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.tab, Tab::Log, tr(lang, "Log", "Log"));
                ui.selectable_value(&mut self.tab, Tab::Scores, tr(lang, "High scores", "Bestenliste"));
                ui.selectable_value(&mut self.tab, Tab::Analysis, tr(lang, "Analysis", "Auswertung"));
                if v.settings.debug {
                    ui.selectable_value(&mut self.tab, Tab::Sent, tr(lang, "Sent", "Gesendet"));
                }
                if v.total > v.qsos.len() {
                    ui.label(RichText::new(format!("{} QSOs", v.total)).color(pal().muted));
                }
            });
            self.result_banner(ui, v);
            match self.tab {
                Tab::Log => log_table(ui, v),
                Tab::Scores => self.score_table(ui, v),
                Tab::Sent => self.sent_table(ui, lang),
                Tab::Analysis => super::analysis::analysis_tab(ui, v),
            }
        });
    }

    /// The best runs of one kind, best first.
    fn score_table(&mut self, ui: &mut Ui, v: &View) {
        let lang = v.settings.lang;
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            let boards = [
                (Board::Skill, tr(lang, "Skill", "Leistung")),
                (Board::Wpx, "WPX"),
                (Board::Hst, "HST"),
                (Board::Coded, tr(lang, "From a code", "Per Code")),
                (Board::Progress, tr(lang, "Progress", "Verlauf")),
            ];
            for (b, label) in boards {
                let n = self.scores.iter().filter(|s| b.holds(s)).count();
                let text = if n > 0 && b != Board::Progress { format!("{label} ({n})") } else { label.to_string() };
                ui.selectable_value(&mut self.board, b, RichText::new(text).size(13.0));
            }
        });
        if v.settings.debug {
            ui.label(RichText::new(tr(lang, "Debug is on: runs score nothing and stay out of this list.", "Debug ist an: Läufe bekommen keine Punkte und kommen nicht in diese Liste.")).color(pal().signal).size(13.0));
        }
        let board = self.board;
        if board == Board::Progress {
            super::analysis::progress(ui, &self.scores, lang);
            return;
        }
        let mut list: Vec<(usize, &ScoreLine)> = self.scores.iter().enumerate().filter(|(_, s)| board.holds(s)).collect();
        if list.is_empty() {
            ui.add_space(12.0);
            let hint = match board {
                Board::Skill => tr(lang, "Every run that ends by itself lands here, ranked by skill points per hour.", "Jeder Lauf, der von selbst endet, steht dann hier, sortiert nach Leistungspunkten pro Stunde."),
                Board::Coded => tr(lang, "A pile-up or single calls run started from a code and played to the end lands here.", "Ein per Code gestarteter und zu Ende gespielter Pile-Up- oder Einzelanruf-Lauf steht dann hier."),
                _ => tr(lang, "A finished competition lands here. Stopping it early gives no score.", "Ein zu Ende gespielter Wettbewerb steht dann hier. Wer vorher stoppt, bekommt keine Wertung."),
            };
            ui.label(RichText::new(hint).color(pal().muted));
            return;
        }
        if board == Board::Skill {
            ui.label(RichText::new(tr(lang, "Pts/h are skill points per hour. Per good QSO 10 points at 25 wpm, in proportion to the speed. Each question -20 %, a busted QSO takes off half its worth, each band effect +10 %, a head start +10 % from 1 h, +20 % from 6 h.", "Pkt/h sind Leistungspunkte pro Stunde. Je gutes QSO 10 Punkte bei 25 wpm, im Verhältnis zum Tempo. Jede Rückfrage -20 %, ein fehlerhaftes QSO zieht die Hälfte seines Werts ab, jeder Bandeffekt +10 %, ein Vorlauf ab 1 h +10 %, ab 6 h +20 %.")).color(pal().muted).size(12.0));
        }
        let rank = |s: &ScoreLine| if board == Board::Skill { s.skill.as_ref().map_or(0, |k| k.per_hour) } else { s.score };
        // best first, and of two equal scores the earlier one
        list.sort_by(|a, b| rank(b.1).cmp(&rank(a.1)).then(a.0.cmp(&b.0)));
        let head = if board == Board::Skill {
            skill_row(tr(lang, "Pl.", "Pl."), tr(lang, "Pts/h", "Pkt/h"), tr(lang, "Sum", "Summe"), "QSOs", tr(lang, "Ask", "Rf"), "wpm", tr(lang, "Mode", "Modus"), tr(lang, "Date", "Datum"))
        } else {
            let pfx = if board == Board::Hst { "" } else { tr(lang, "Pref", "Präf") };
            score_row(tr(lang, "Pl.", "Pl."), tr(lang, "Score", "Punkte"), "QSOs", pfx, "Call", tr(lang, "Date", "Datum"))
        };
        let size = fitting_size(ui, &head);
        let mono = |s: String, c: Color32| RichText::new(s).monospace().size(size).color(c);
        ui.add(egui::Label::new(mono(head, pal().muted)).extend());
        egui::ScrollArea::vertical().id_salt("scores").auto_shrink([false, false]).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            for (place, (i, s)) in list.iter().take(PLACES).enumerate() {
                let new = self.latest == Some(*i);
                let color = if new { pal().good } else if place == 0 { pal().glow } else { pal().text };
                let pl = format!("{}.", place + 1);
                let line = match (&s.skill, board) {
                    (Some(k), Board::Skill) => skill_row(&pl, &k.per_hour.to_string(), &k.points.to_string(), &k.good.to_string(), &k.asks.to_string(), &k.wpm.to_string(), mode_label(&s.mode, lang), &short_date(&s.date, lang)),
                    _ => {
                        let mults = if board == Board::Hst { String::new() } else { s.mults.to_string() };
                        score_row(&pl, &s.score.to_string(), &s.qsos.to_string(), &mults, &s.call, &short_date(&s.date, lang))
                    }
                };
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    let hit = ui.add(egui::Label::new(mono(line, color)).extend().sense(Sense::click()));
                    if new {
                        ui.label(RichText::new(tr(lang, "new", "neu")).size(12.0).color(pal().good));
                    }
                    if !s.code.is_empty() && hit.on_hover_text(format!("{}\n{}", s.code, tr(lang, "Click to copy the code", "Klick kopiert den Code"))).clicked() {
                        ui.ctx().copy_text(s.code.clone());
                    }
                });
            }
        });
    }

    /// Run and score side by side: buttons, run code and the QSOs per five minutes on the
    /// left, raw against verified score and skill points on the right.
    fn dashboard(&mut self, ui: &mut Ui, r: Rect, v: &View) {
        let gap = 24.0;
        let half = (r.width() - gap) / 2.0;
        let left = Rect::from_min_size(r.min, vec2(half, r.height()));
        let right = Rect::from_min_max(pos2(r.left() + half + gap, r.top()), r.max);
        let y = self.run_controls(ui, left, v, true);
        let hist = Rect::from_min_max(pos2(left.left(), y + 6.0), pos2(left.right(), left.bottom() - 16.0));
        histogram(ui.painter(), hist, v);
        score_block(ui.painter(), right, v);
    }

    /// The buttons that start a run, or stop it, and the code of the run. Gives the y
    /// below them.
    fn run_controls(&mut self, ui: &mut Ui, r: Rect, v: &View, with_code: bool) -> f32 {
        let lang = v.settings.lang;
        let on = v.mode != RunMode::Stop;
        let used = ui.scope_builder(UiBuilder::new().max_rect(r), |ui| {
            ui.spacing_mut().button_padding = vec2(10.0, 5.0);
            ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
            ui.horizontal_wrapped(|ui| {
                if on {
                    if ui.button(tr(lang, "Stop  F10", "Stopp  F10")).clicked() {
                        self.stop();
                    }
                } else {
                    let list = [
                        (RunMode::Pileup, "Pile-Up"),
                        (RunMode::Single, tr(lang, "Single", "Einzeln")),
                        (RunMode::Wpx, "WPX"),
                        (RunMode::Hst, "HST"),
                    ];
                    for (m, label) in list {
                        if ui.button(label).clicked() {
                            self.start(m);
                        }
                    }
                }
            });
            if let Some(e) = &self.audio_error {
                ui.colored_label(pal().signal, format!("{}: {e}", tr(lang, "No sound", "Kein Ton")));
            }
        });
        let mut y = used.response.rect.bottom() + 6.0;
        if with_code && let Some(code) = &v.code {
            let line = Rect::from_min_size(pos2(r.left(), y), vec2(r.width(), 20.0));
            let hit = ui.interact(line, ui.scope_id().with("run code"), Sense::click());
            let color = if hit.hovered() { pal().glow } else { pal().muted };
            ui.painter().text(line.left_center(), Align2::LEFT_CENTER, code, FontId::monospace(13.0), color);
            let hint = if v.fixed {
                tr(lang, "Code of this run, conditions fixed. Click to copy.", "Code dieses Laufs, Bedingungen fest. Klick kopiert.")
            } else {
                tr(lang, "Code of this run. Click to copy.", "Code dieses Laufs. Klick kopiert.")
            };
            if hit.on_hover_text(hint).clicked() {
                ui.ctx().copy_text(code.clone());
            }
            y += 22.0;
        }
        y
    }

    /// The result of the run that just ended, as a strip above the log until closed.
    fn result_banner(&mut self, ui: &mut Ui, v: &View) {
        let Some(f) = self.result.clone() else { return };
        let lang = v.settings.lang;
        let mut close = false;
        ui.add_space(4.0);
        egui::Frame::new().fill(pal().panel_hi).stroke(Stroke::new(1.0, pal().glow_dim)).corner_radius(3.0).inner_margin(10.0).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let score = if f.mode == RunMode::Hst { f.hst_score } else { f.verified.score };
                if f.unscored {
                    ui.label(RichText::new("—").monospace().size(32.0).color(pal().signal));
                } else {
                    ui.label(RichText::new(score.to_string()).monospace().size(32.0).color(pal().glow));
                }
                ui.add_space(10.0);
                ui.vertical(|ui| {
                    let over = if f.mode.competition() { tr(lang, "The contest is over", "Der Contest ist zu Ende") } else { tr(lang, "The run is over", "Der Lauf ist zu Ende") };
                    ui.label(RichText::new(over).size(15.0));
                    let mut line = format!("{} QSOs, {} {}", f.verified.points, f.verified.mults, tr(lang, "prefixes", "Präfixe"));
                    if f.unscored {
                        ui.label(RichText::new(line).color(pal().muted));
                        let why = tr(lang, "No score: debug was on during the run. Not in the high scores.", "Keine Wertung: Debug war während des Laufs an. Kein Eintrag in der Bestenliste.");
                        ui.label(RichText::new(why).color(pal().signal));
                        return;
                    }
                    if f.new_record {
                        line += tr(lang, ", new record", ", neuer Rekord");
                    }
                    ui.label(RichText::new(line).color(if f.new_record { pal().good } else { pal().muted }));
                    if let Some(k) = f.skill {
                        let text = format!(
                            "{}: {} {}, {} {}  ({} {}, {} {}, {} wpm)",
                            tr(lang, "Skill", "Leistung"),
                            k.per_hour,
                            tr(lang, "per hour", "pro Stunde"),
                            k.points,
                            tr(lang, "in total", "gesamt"),
                            k.asks,
                            tr(lang, "questions", "Rückfragen"),
                            k.busted,
                            tr(lang, "busted", "fehlerhaft"),
                            k.wpm,
                        );
                        ui.label(RichText::new(text).color(pal().glow));
                    }
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                    if ui.button(tr(lang, "Close", "Schließen")).clicked() {
                        close = true;
                    }
                });
            });
            if let Some(code) = &f.code {
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(tr(lang, "Same run for someone else:", "Derselbe Lauf für andere:")).color(pal().muted));
                    ui.label(RichText::new(code.as_str()).monospace().size(13.0));
                    if ui.small_button(tr(lang, "Copy", "Kopieren")).clicked() {
                        ui.ctx().copy_text(code.clone());
                    }
                });
            }
        });
        ui.add_space(4.0);
        if close {
            self.result = None;
        }
    }
}

/// Raw against verified score, then the skill points so far.
fn score_block(p: &egui::Painter, r: Rect, v: &View) {
    let lang = v.settings.lang;
    let hst = v.mode == RunMode::Hst;
    let mono = FontId::monospace(14.0);
    let w = r.width().min(330.0);
    let col = [r.left() + w * 0.62, r.left() + w];
    let mut y = r.top();
    p.text(pos2(col[0], y + 8.0), Align2::RIGHT_CENTER, tr(lang, "Raw", "Roh"), FontId::proportional(13.0), pal().muted);
    p.text(pos2(col[1], y + 8.0), Align2::RIGHT_CENTER, tr(lang, "Verified", "Geprüft"), FontId::proportional(13.0), pal().muted);
    y += 20.0;
    let rows: Vec<(&str, u64, u64)> = if hst {
        vec![(tr(lang, "Points", "Punkte"), v.hst.0, v.hst.1)]
    } else {
        vec![
            ("QSOs", v.raw.points, v.verified.points),
            (tr(lang, "Prefixes", "Präfixe"), v.raw.mults, v.verified.mults),
            (tr(lang, "Score", "Punkte"), v.raw.score, v.verified.score),
        ]
    };
    for (label, a, b) in rows {
        p.text(pos2(r.left(), y + 8.0), Align2::LEFT_CENTER, label, FontId::proportional(14.0), pal().muted);
        p.text(pos2(col[0], y + 8.0), Align2::RIGHT_CENTER, a.to_string(), mono.clone(), pal().text);
        p.text(pos2(col[1], y + 8.0), Align2::RIGHT_CENTER, b.to_string(), mono.clone(), pal().text);
        y += 20.0;
    }

    // skill points so far, scaled to an hour once a minute is worked
    let k = v.skill;
    let (sum, hour) = if v.unscored {
        ("-".to_string(), String::new())
    } else if v.secs >= 60.0 {
        (k.points.to_string(), format!("{} /h", k.per_hour))
    } else {
        (k.points.to_string(), String::new())
    };
    p.text(pos2(r.left(), y + 8.0), Align2::LEFT_CENTER, tr(lang, "Skill", "Leistung"), FontId::proportional(14.0), pal().muted);
    p.text(pos2(col[0], y + 8.0), Align2::RIGHT_CENTER, sum, mono.clone(), if v.unscored { pal().signal } else { pal().glow });
    p.text(pos2(col[1], y + 8.0), Align2::RIGHT_CENTER, hour, mono, pal().glow);
}

/// QSOs per five minutes, the shape of the session, with its caption below.
fn histogram(p: &egui::Painter, hist: Rect, v: &View) {
    let lang = v.settings.lang;
    let on = v.mode != RunMode::Stop;
    p.rect_filled(hist, 2.0, pal().panel_hi);
    let top = v.histo.iter().copied().max().unwrap_or(0).max(10) as f32;
    let w = hist.width() / HISTO_BINS as f32;
    let now = (v.secs / 300.0) as usize;
    for (i, n) in v.histo.iter().enumerate() {
        if *n == 0 {
            continue;
        }
        let bar = (*n as f32 / top) * (hist.height() - 4.0);
        let x = hist.left() + i as f32 * w;
        let c = if i == now && on { pal().glow } else { pal().glow_dim };
        p.rect_filled(Rect::from_min_max(pos2(x + 0.5, hist.bottom() - bar), pos2(x + w - 0.5, hist.bottom())), 0.0, c);
    }
    p.text(pos2(hist.left(), hist.bottom() + 8.0), Align2::LEFT_CENTER, tr(lang, "QSOs per 5 minutes", "QSOs je 5 Minuten"), FontId::proportional(12.0), pal().muted);
}

/// The monospace size at which `line` fits the width of `ui`, 14 at most.
fn fitting_size(ui: &Ui, line: &str) -> f32 {
    let w = ui.painter().layout_no_wrap(line.to_string(), FontId::monospace(14.0), pal().text).size().x;
    let room = ui.available_width() - 30.0;
    if w <= room { 14.0 } else { (14.0 * room / w).max(10.0) }
}

fn score_row(place: &str, score: &str, qsos: &str, pfx: &str, call: &str, date: &str) -> String {
    format!("{place:>3} {score:>7} {qsos:>5} {pfx:>5}  {call:<10} {date}")
}

/// `YYYY-MM-DD` without the year, the way it is written in the language.
pub(super) fn short_date(date: &str, lang: Lang) -> String {
    match (date.get(5..7), date.get(8..10), lang) {
        (Some(m), Some(d), Lang::De) => format!("{d}.{m}."),
        (Some(m), Some(d), Lang::En) => format!("{m}-{d}"),
        (Some(m), Some(d), Lang::Pl | Lang::Ru) => format!("{d}.{m}"),
        (Some(m), Some(d), _) => format!("{d}/{m}"),
        _ => date.to_string(),
    }
}

#[allow(clippy::too_many_arguments)]
fn skill_row(place: &str, per_hour: &str, sum: &str, qsos: &str, asks: &str, wpm: &str, mode: &str, date: &str) -> String {
    format!("{place:>3} {per_hour:>6} {sum:>6} {qsos:>4} {asks:>3} {wpm:>3}  {mode:<7} {date}")
}

fn mode_label(mode: &str, lang: Lang) -> &'static str {
    match mode {
        "WPX" => "WPX",
        "HST" => "HST",
        "One" => tr(lang, "Single", "Einzeln"),
        _ => "Pile-Up",
    }
}

fn log_table(ui: &mut Ui, v: &View) {
    let lang = v.settings.lang;
    if v.qsos.is_empty() {
        ui.add_space(12.0);
        let hint = if v.mode == RunMode::Stop {
            tr(lang, "Press F9 or Alt-R for a pile-up. Enter calls CQ, then the Enter key works the whole contact.", "F9 oder Alt-R startet ein Pile-Up. Enter ruft CQ, danach führt die Enter-Taste durch das ganze QSO.")
        } else {
            tr(lang, "Type the call you heard and press Enter.", "Tippe das gehörte Rufzeichen und drücke Enter.")
        };
        ui.label(RichText::new(hint).color(pal().muted));
        return;
    }
    let hst = v.mode == RunMode::Hst;
    let row = |t: &str, call: &str, rx: &str, tx: &str, pfx: &str, asks: &str, pts: &str| format!("{t:<8}  {call:<12} {rx:<8}  {tx:<8}  {pfx:<5} {asks:>2} {pts:>4}  ");
    ui.add_space(2.0);
    let head = row(tr(lang, "Time", "Zeit"), "Call", tr(lang, "Received", "Empfang"), tr(lang, "Sent", "Gesendet"), if hst { "Pts" } else { "Pref" }, tr(lang, " ?", "Rf"), tr(lang, "Skl", "Lst")) + "Check " + tr(lang, "Country", "Land");
    let size = fitting_size(ui, &head);
    ui.add(egui::Label::new(RichText::new(head).monospace().size(size).color(pal().muted)).extend());
    egui::ScrollArea::vertical().stick_to_bottom(true).auto_shrink([false, false]).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 2.0;
        for q in &v.qsos {
            let pts = mr_sim::log::call_to_score(&q.call).to_string();
            let line = row(
                &clock(q.t),
                &q.call,
                &format!("{} {:04}", q.rst, q.nr),
                &format!("{} {:04}", q.sent_rst, q.sent_nr),
                if hst { &pts } else { &q.pfx },
                &if q.asks > 0 { q.asks.to_string() } else { String::new() },
                &if v.unscored { "-".to_string() } else { format!("{:.0}", mr_sim::skill::value(q)) },
            );
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.add(egui::Label::new(RichText::new(line).monospace().size(size)).extend());
                let mark = format!("{:<6}", q.err.label());
                ui.add(egui::Label::new(RichText::new(mark).monospace().size(size).color(pal().signal)).extend());
                // the country of the call as logged, cut short where the log ends
                if let Some(c) = mr_sim::country::of(&q.call) {
                    let full = if lang == Lang::En { c.name } else { super::countries::name(c, lang) };
                    ui.add(egui::Label::new(RichText::new(super::countries::name(c, lang)).size(size).color(pal().muted)).truncate()).on_hover_text(format!("{full} ({})", c.main));
                }
            });
        }
    });
}

