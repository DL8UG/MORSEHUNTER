//! The tab beside the log with everything that goes out on the air, newest line at the bottom.

use egui::{RichText, Ui};
use mr_sim::sent::{Sent, Sender};
use mr_sim::settings::Lang;

use super::theme::*;
use super::{App, tr};

const TEXT_SIZE: f32 = 15.0;

fn line(ui: &mut Ui, l: &Sent, lang: Lang) {
    let secs = l.t as u64;
    let call_color = match l.who {
        Sender::Me => pal().good,
        Sender::Caller => pal().text,
        Sender::Qrm => pal().signal,
    };
    let mono = |s: String, c| RichText::new(s).monospace().size(TEXT_SIZE).color(c);
    let mut notes = Vec::new();
    if l.who == Sender::Qrm {
        notes.push("QRM");
    }
    if l.hand {
        notes.push(tr(lang, "by hand", "von Hand"));
    }
    if l.cut {
        notes.push(tr(lang, "stopped", "abgebrochen"));
    }
    ui.horizontal_top(|ui| {
        ui.label(mono(format!("{:02}:{:02}", secs / 60, secs % 60), pal().muted));
        ui.label(mono(format!("{:<12}", l.call), call_color));
        ui.spacing_mut().item_spacing.x = 10.0;
        ui.horizontal_wrapped(|ui| {
            ui.label(mono(l.text.clone(), if l.on_air { pal().glow } else { pal().muted }));
            if !notes.is_empty() {
                ui.label(RichText::new(notes.join(", ")).size(12.0).color(pal().muted));
            }
        });
    });
}

impl App {
    pub(super) fn sent_table(&self, ui: &mut Ui, lang: Lang) {
        let lines = self.contest.lock().unwrap().sent.lines().cloned().collect::<Vec<Sent>>();
        if lines.is_empty() {
            ui.add_space(12.0);
            ui.label(RichText::new(tr(lang, "Start a run. Everything that is sent appears here.", "Starte einen Lauf. Alles Gesendete erscheint hier.")).color(pal().muted));
            return;
        }
        ui.add_space(2.0);
        egui::ScrollArea::vertical().id_salt("sent").auto_shrink([false, false]).stick_to_bottom(true).show(ui, |ui| {
            for l in &lines {
                line(ui, l, lang);
            }
        });
    }
}
