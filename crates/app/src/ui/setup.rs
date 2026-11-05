//! The options page: everything that can be set, in cards side by side.

use egui::{Rect, RichText, Stroke, Ui, UiBuilder};
use mr_sim::callgen::Region;
use mr_sim::code::{CodeError, SIM_VERSION};
use mr_sim::contest::RunMode;
use mr_sim::keyer::Mode;
use mr_sim::settings::{ACTIVITY_MAX, BW_MAX, BW_MIN, CALLER_FAST_MAX, CALLER_SLOW_MIN, DURATION_MAX, HEAD_START_MAX, Lang, PITCH_MAX, PITCH_MIN, STEP_HZ, Settings, Theme, WPM_MAX, WPM_MIN};

use super::theme::*;
use super::{App, Paddle, View, tr};
use crate::{store, vail};

/// A card is not narrower than this, the number of columns follows from it.
const CARD_MIN: f32 = 330.0;
const GAP: f32 = 14.0;
const LABEL_W: f32 = 112.0;
const MARGIN: f32 = 12.0;

#[derive(Clone, Copy)]
enum Card {
    Station,
    Signal,
    Band,
    Length,
    Regions,
    Paddle,
    Code,
    Recording,
    Display,
}

/// The cards per column, for one, two and three columns. Each column about the same height.
fn layout(columns: usize) -> Vec<Vec<Card>> {
    use Card::*;
    match columns {
        1 => vec![vec![Station, Signal, Band, Length, Regions, Paddle, Code, Recording, Display]],
        2 => vec![vec![Station, Signal, Band, Length, Display], vec![Paddle, Regions, Code, Recording]],
        _ => vec![vec![Station, Signal, Length], vec![Band, Regions, Display], vec![Paddle, Code, Recording]],
    }
}

fn region_name(r: Region, lang: Lang) -> &'static str {
    match r {
        Region::NorthAmerica => tr(lang, "North America", "Nordamerika"),
        Region::SouthAmerica => tr(lang, "South America", "Südamerika"),
        Region::Europe => tr(lang, "Europe", "Europa"),
        Region::Asia => tr(lang, "Asia", "Asien"),
        Region::Oceania => tr(lang, "Oceania", "Ozeanien"),
        Region::Africa => tr(lang, "Africa", "Afrika"),
    }
}

/// What opens a file with the program the desktop has for it.
const OPEN: &str = if cfg!(target_os = "macos") {
    "open"
} else if cfg!(windows) {
    "explorer"
} else {
    "xdg-open"
};

fn mode_name(m: Mode, lang: Lang) -> &'static str {
    match m {
        Mode::Straight => tr(lang, "Straight key", "Handtaste"),
        Mode::IambicA => "Iambic A",
        Mode::IambicB => "Iambic B",
        Mode::Bug => "Bug",
        Mode::Ultimatic => "Ultimatic",
    }
}

fn row(ui: &mut Ui, label: &str, add: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(egui::vec2(LABEL_W, 20.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
            ui.set_min_width(LABEL_W);
            ui.label(RichText::new(label).color(pal().muted));
        });
        add(ui);
    });
}

fn note(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).color(pal().muted).size(12.5));
}

/// A framed card with a title and, if the card is locked now, why. The panel color stays
/// inside, the controls are drawn in `pal().panel_hi` and would vanish on it.
fn card(ui: &mut Ui, title: &str, locked: Option<&str>, body: impl FnOnce(&mut Ui)) {
    let inner = ui.max_rect().width() - 2.0 * MARGIN - 2.0;
    egui::Frame::new().stroke(Stroke::new(1.0, pal().line)).corner_radius(4.0).inner_margin(MARGIN).show(ui, |ui| {
        ui.set_width(inner);
        ui.horizontal(|ui| {
            ui.label(RichText::new(title).size(16.0).color(pal().glow));
            if let Some(why) = locked {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(why).size(12.0).color(pal().muted));
                });
            }
        });
        ui.add_space(6.0);
        ui.spacing_mut().slider_width = (inner - LABEL_W - 90.0).clamp(100.0, 280.0);
        body(ui);
    });
    ui.add_space(GAP);
}

/// What the card needs besides the settings.
struct Ctx<'a> {
    v: &'a View,
    lang: Lang,
    stopped: bool,
    competition: bool,
    hst: bool,
    start: bool,
}

impl App {
    pub(super) fn setup(&mut self, ui: &mut Ui, r: Rect, v: &View) {
        let mut s = v.settings.clone();
        let mut cx = Ctx {
            v,
            lang: v.settings.lang,
            stopped: v.mode == RunMode::Stop,
            competition: v.mode.competition(),
            hst: v.mode == RunMode::Hst,
            start: false,
        };
        ui.scope_builder(UiBuilder::new().max_rect(r), |ui| {
            egui::ScrollArea::vertical().id_salt("options").auto_shrink([false, false]).show(ui, |ui| {
                let width = ui.available_width() - 12.0;
                let n = ((width + GAP) / (CARD_MIN + GAP)).floor().clamp(1.0, 3.0) as usize;
                let lists = layout(n);
                let col_w = (width - GAP * (n - 1) as f32) / n as f32;
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = GAP;
                    for list in lists {
                        ui.allocate_ui_with_layout(egui::vec2(col_w, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                            ui.set_width(col_w);
                            for c in list {
                                self.card(ui, c, &mut s, &mut cx);
                            }
                        });
                    }
                });
            });
        });
        if s != v.settings {
            s.call = s.call.trim().to_string();
            self.contest.lock().unwrap().apply(s);
        }
        if cx.start {
            self.start_typed_code();
        }
    }

    /// The Vail adapter: set it to passthrough, or back to its own keyer, over MIDI.
    fn vail_adapter(&mut self, ui: &mut Ui, s: &mut Settings, lang: Lang) {
        let t = |en, de| tr(lang, en, de);
        let st = &mut self.vail;
        let ports = st.ports.get_or_insert_with(vail::ports);
        ui.add_space(4.0);
        ui.label(RichText::new(t("Vail adapter", "Vail-Adapter")).color(pal().glow_dim));
        let mut search = false;
        let mut send = None;
        match ports {
            Ok(list) if !list.is_empty() => {
                st.chosen = st.chosen.min(list.len() - 1);
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_salt("vail port").width(180.0).selected_text(list[st.chosen].as_str()).show_ui(ui, |ui| {
                        for (i, name) in list.iter().enumerate() {
                            ui.selectable_value(&mut st.chosen, i, name.as_str());
                        }
                    });
                    search = ui.small_button(t("Search again", "Neu suchen")).clicked();
                });
                ui.horizontal(|ui| {
                    let pass = ui.button("Passthrough").on_hover_text(t(
                        "The adapter only reports the paddles and this program keys. Needs firmware 5.1 or newer. The adapter keeps it.",
                        "Der Adapter meldet nur die Paddles, dieses Programm tastet. Braucht Firmware 5.1 oder neuer. Der Adapter behält das.",
                    ));
                    let own = ui.button(t("Its own keyer", "Eigener Keyer")).on_hover_text(t(
                        "Back to the adapter's iambic B, for older firmware. The keyer here becomes Straight key.",
                        "Zurück auf Iambic B im Adapter, für ältere Firmware. Der Keyer hier wird Handtaste.",
                    ));
                    if pass.clicked() {
                        send = Some((list[st.chosen].clone(), vail::PASSTHROUGH));
                    } else if own.clicked() {
                        send = Some((list[st.chosen].clone(), vail::IAMBIC_B));
                    }
                });
            }
            Ok(_) | Err(_) => {
                ui.horizontal(|ui| {
                    let why = match ports {
                        Err(e) => format!("{}: {e}", t("MIDI error", "MIDI-Fehler")),
                        Ok(_) => t("No MIDI output found. Plug the adapter in.", "Kein MIDI-Ausgang gefunden. Adapter einstecken.").to_string(),
                    };
                    note(ui, &why);
                    search = ui.small_button(t("Search again", "Neu suchen")).clicked();
                });
            }
        }
        if search {
            st.ports = Some(vail::ports());
            st.status = None;
        }
        if let Some((port, keyer)) = send {
            st.status = Some(match vail::set_keyer(&port, keyer) {
                Ok(()) if keyer == vail::PASSTHROUGH => {
                    if s.keyer.mode == Mode::Straight {
                        s.keyer.mode = Mode::IambicB;
                    }
                    (true, t("Passthrough is set. Key both paddles to check.", "Passthrough ist gesetzt. Zur Probe beide Paddles geben.").to_string())
                }
                Ok(()) => {
                    s.keyer.mode = Mode::Straight;
                    (true, t("The adapter keys by itself again, the keyer here is Straight key.", "Der Adapter tastet wieder selbst, der Keyer hier ist Handtaste.").to_string())
                }
                Err(e) => (false, format!("{}: {e}", t("MIDI error", "MIDI-Fehler"))),
            });
        }
        if let Some((ok, text)) = &st.status {
            ui.label(RichText::new(text).size(12.5).color(if *ok { pal().good } else { pal().signal }));
        }
    }

    fn card(&mut self, ui: &mut Ui, c: Card, s: &mut Settings, cx: &mut Ctx) {
        let lang = cx.lang;
        let t = |en, de| tr(lang, en, de);
        let v = cx.v;
        let in_run = (!cx.stopped).then(|| t("locked during a run", "im Lauf gesperrt"));
        let by_code = v.fixed.then(|| t("fixed by code", "fest per Code"));
        match c {
            Card::Station => card(ui, t("Station", "Station"), in_run, |ui| {
                ui.add_enabled_ui(cx.stopped, |ui| {
                    row(ui, t("Call", "Rufzeichen"), |ui| {
                        if ui.text_edit_singleline(&mut s.call).changed() {
                            s.call = s.call.to_uppercase();
                        }
                    });
                    row(ui, t("Operator", "Operator"), |ui| {
                        ui.text_edit_singleline(&mut s.operator);
                    });
                });
            }),
            Card::Signal => card(ui, t("Speed and sound", "Tempo und Ton"), by_code, |ui| {
                ui.add_enabled_ui(!v.fixed, |ui| {
                    row(ui, t("Speed", "Tempo"), |ui| {
                        ui.add(egui::Slider::new(&mut s.wpm, WPM_MIN..=WPM_MAX).suffix(" wpm"));
                    });
                    row(ui, t("Pitch", "Tonhöhe"), |ui| {
                        ui.add(egui::Slider::new(&mut s.pitch, PITCH_MIN..=PITCH_MAX).step_by(STEP_HZ as f64).suffix(" Hz"));
                    });
                });
                // the callers' speeds in percent of yours; competitions keep the original
                ui.add_enabled_ui(!cx.competition && !v.fixed, |ui| {
                    let why = t("Callers who answer from now on, those already calling keep their speed.", "Für Anrufer, die ab jetzt antworten. Wer schon ruft, behält sein Tempo.");
                    row(ui, t("Callers from", "Anrufer ab"), |ui| {
                        ui.add(egui::Slider::new(&mut s.caller_slow, CALLER_SLOW_MIN..=100).suffix(" %")).on_hover_text(why);
                    });
                    row(ui, t("Callers up to", "Anrufer bis"), |ui| {
                        ui.add(egui::Slider::new(&mut s.caller_fast, 100..=CALLER_FAST_MAX).suffix(" %")).on_hover_text(why);
                    });
                    if !cx.hst {
                        let at = |pct: u32| (s.wpm as f64 * pct as f64 / 100.0).round().clamp(5.0, WPM_MAX as f64) as u32;
                        row(ui, "", |ui| {
                            ui.label(RichText::new(format!("{} – {} wpm", at(s.caller_slow), at(s.caller_fast))).color(pal().muted));
                        });
                    }
                });
                ui.add_enabled_ui(!cx.hst && !v.fixed, |ui| {
                    row(ui, t("Bandwidth", "Bandbreite"), |ui| {
                        ui.add(egui::Slider::new(&mut s.bandwidth, BW_MIN..=BW_MAX).step_by(STEP_HZ as f64).suffix(" Hz"));
                    });
                });
                row(ui, t("Sidetone", "Mithörton"), |ui| {
                    ui.add(egui::Slider::new(&mut s.mon_db, -60.0..=20.0).suffix(" dB"));
                });
                ui.add_enabled_ui(!v.fixed, |ui| {
                    ui.checkbox(&mut s.qsk, t("QSK: hear the band between elements", "QSK: Band zwischen den Zeichen hören"));
                });
            }),
            Card::Band => {
                let why = if cx.competition { Some(t("set by the competition", "vom Wettbewerb vorgegeben")) } else { by_code };
                card(ui, t("Band conditions", "Bandbedingungen"), why, |ui| {
                    ui.add_enabled_ui(!cx.competition && !v.fixed, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.checkbox(&mut s.qrn, "QRN");
                            ui.checkbox(&mut s.qrm, "QRM");
                            ui.checkbox(&mut s.qsb, "QSB");
                            ui.checkbox(&mut s.flutter, "Flutter");
                            ui.checkbox(&mut s.lids, "LIDs");
                        });
                    });
                    ui.add_enabled_ui(!cx.hst && !v.fixed, |ui| {
                        row(ui, t("Activity", "Aktivität"), |ui| {
                            ui.add(egui::Slider::new(&mut s.activity, 1..=ACTIVITY_MAX));
                        });
                    });
                    // it says how the run began: set before the start, like the run length
                    ui.add_enabled_ui(cx.stopped, |ui| {
                        row(ui, t("Contest running for", "Contest läuft seit"), |ui| {
                            ui.add(egui::Slider::new(&mut s.head_start, 0..=HEAD_START_MAX).suffix(" h")).on_hover_text(t(
                                "The callers' serial numbers start as high as after this many hours. At 0, as in the original, everybody starts with you.",
                                "Die Nummern der Anrufer stehen so hoch wie nach so vielen Stunden. Bei 0 fangen alle mit dir an, wie im Original.",
                            ));
                        });
                        ui.checkbox(&mut s.waves, t("Activity comes in waves", "Aktivität schwankt")).on_hover_text(t(
                            "Callers come in waves, with lulls and short openings, as in a real contest. Not in the original.",
                            "Die Anrufer kommen in Wellen, mit ruhigen Phasen und kurzen Öffnungen, wie in einem echten Contest. Nicht im Original.",
                        ));
                    });
                })
            }
            Card::Length => card(ui, t("Run length", "Laufzeit"), in_run, |ui| {
                ui.add_enabled_ui(cx.stopped, |ui| {
                    row(ui, t("Practice", "Übung"), |ui| {
                        ui.add(egui::Slider::new(&mut s.duration, 1..=DURATION_MAX).suffix(" min"));
                    });
                    row(ui, t("Competition", "Wettbewerb"), |ui| {
                        ui.add(egui::Slider::new(&mut s.competition_duration, 1..=60).suffix(" min"));
                    });
                });
            }),
            Card::Regions => card(ui, t("Where callers come from", "Woher die Anrufer kommen"), in_run, |ui| {
                ui.add_enabled_ui(cx.stopped, |ui| {
                    for r in Region::ALL {
                        row(ui, region_name(r, lang), |ui| {
                            ui.add(egui::Slider::new(&mut s.region_mix[r as usize], 0.0..=100.0).show_value(false));
                        });
                    }
                });
            }),
            Card::Paddle => card(ui, "Paddle", None, |ui| {
                ui.checkbox(&mut s.paddle, t("Key by hand", "Von Hand geben"));
                ui.add_enabled_ui(s.paddle, |ui| {
                    ui.checkbox(&mut s.paddle_tu_saves, t("A keyed TU logs the contact", "Ein gegebenes TU loggt das QSO"));
                    row(ui, t("Keyer", "Keyer"), |ui| {
                        egui::ComboBox::from_id_salt("keyer").selected_text(mode_name(s.keyer.mode, lang)).show_ui(ui, |ui| {
                            for m in [Mode::IambicB, Mode::IambicA, Mode::Ultimatic, Mode::Bug, Mode::Straight] {
                                ui.selectable_value(&mut s.keyer.mode, m, mode_name(m, lang));
                            }
                        });
                    });
                    ui.checkbox(&mut s.keyer.swap, t("Swap paddles", "Paddles tauschen"));
                    for (which, label, name) in [(Paddle::Dit, "Dit", &s.dit_key), (Paddle::Dah, "Dah", &s.dah_key)] {
                        row(ui, label, |ui| {
                            let text = if self.capture == Some(which) { t("Press a key", "Taste drücken") } else { name.as_str() };
                            if ui.button(text).clicked() {
                                self.capture = Some(which);
                            }
                        });
                    }
                    if s.paddle && (s.dit_key.starts_with("Control") || s.dah_key.starts_with("Control")) {
                        note(ui, t("Ctrl keys the paddle now. Use Alt for the shortcuts.", "Strg gibt jetzt Morse. Für Tastenkürzel Alt nehmen."));
                    }
                    self.vail_adapter(ui, s, lang);
                });
            }),
            Card::Code => card(ui, t("Shared run", "Geteilter Lauf"), None, |ui| {
                if let Some(code) = &v.code {
                    row(ui, t("This run", "Dieser Lauf"), |ui| {
                        ui.label(RichText::new(code.as_str()).monospace().size(13.0));
                        if ui.small_button(t("Copy", "Kopieren")).clicked() {
                            ui.ctx().copy_text(code.clone());
                        }
                    });
                }
                note(ui, t("Enter a code to work the same stations under the same conditions.", "Code eingeben, um dieselben Stationen unter denselben Bedingungen zu arbeiten."));
                ui.add_enabled_ui(cx.stopped, |ui| {
                    ui.horizontal(|ui| {
                        let edit = ui.add(egui::TextEdit::singleline(&mut self.code_input).hint_text("MR-").desired_width(200.0));
                        let enter = edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                        if ui.button(t("Start", "Starten")).clicked() || enter {
                            cx.start = true;
                        }
                    });
                });
                if let Some(e) = self.code_error {
                    let text = match e {
                        CodeError::Format => t("That is not a run code.", "Das ist kein Lauf-Code.").to_string(),
                        CodeError::Checksum => t("A character is wrong. Check the code.", "Ein Zeichen stimmt nicht. Prüfe den Code.").to_string(),
                        CodeError::Version(n) => format!(
                            "{} {n}, {} {SIM_VERSION}.",
                            t("Code from simulation version", "Code von Simulationsversion"),
                            t("this program has", "dieses Programm hat")
                        ),
                        CodeError::Range => t("The code holds a value this program can't use.", "Der Code enthält einen Wert, den dieses Programm nicht kann.").to_string(),
                    };
                    ui.label(RichText::new(text).color(pal().signal).size(13.0));
                }
            }),
            Card::Recording => card(ui, t("Recording and keyer", "Aufnahme und Keyer"), None, |ui| {
                ui.add_enabled_ui(cx.stopped, |ui| {
                    ui.checkbox(&mut s.save_wav, t("Record each run", "Jeden Lauf aufnehmen"));
                    let wav = store::wav_path().filter(|p| p.exists());
                    if ui.add_enabled(wav.is_some(), egui::Button::new(t("Play last recording", "Letzte Aufnahme abspielen"))).clicked()
                        && let Some(p) = wav
                        && let Err(e) = std::process::Command::new(OPEN).arg(p).spawn()
                    {
                        eprintln!("{OPEN}: {e}");
                    }
                });
                ui.checkbox(&mut s.calls_from_keyer, t("An external keyer sends the call", "Externer Keyer sendet das Rufzeichen"));
            }),
            Card::Display => card(ui, t("Display", "Anzeige"), None, |ui| {
                row(ui, t("Language", "Sprache"), |ui| {
                    egui::ComboBox::from_id_salt("lang").selected_text(s.lang.name()).show_ui(ui, |ui| {
                        for l in Lang::ALL {
                            ui.selectable_value(&mut s.lang, l, l.name());
                        }
                    });
                });
                row(ui, t("Colours", "Farben"), |ui| {
                    egui::ComboBox::from_id_salt("theme").selected_text(theme_name(s.theme, s.lang)).show_ui(ui, |ui| {
                        for th in Theme::ALL {
                            ui.selectable_value(&mut s.theme, th, theme_name(th, s.lang));
                        }
                    });
                });
                ui.checkbox(&mut s.debug, t("Show what is sent in a tab beside the log", "Gesendetes als Reiter neben dem Log zeigen"));
                let warn = t("It gives away every call. A run with it on at any time scores nothing and stays out of the high scores.", "Das verrät jedes Rufzeichen. Ein Lauf, in dem das irgendwann an war, bekommt keine Punkte und keinen Platz in der Bestenliste.");
                ui.label(RichText::new(warn).size(12.5).color(if s.debug { pal().signal } else { pal().muted }));
                let under = self.shared.underruns.load(std::sync::atomic::Ordering::Relaxed);
                if under > 0 {
                    note(ui, &format!("{}: {under}", t("Audio dropouts", "Tonaussetzer")));
                }
            }),
        }
    }
}

fn theme_name(th: Theme, lang: Lang) -> &'static str {
    match th {
        Theme::Night => tr(lang, "Morsehunter dark", "Morsehunter dunkel"),
        Theme::Day => tr(lang, "Morsehunter light", "Morsehunter hell"),
        Theme::Dl8ugLight => tr(lang, "DL8UG light", "DL8UG hell"),
        Theme::Dl8ugDark => tr(lang, "DL8UG dark", "DL8UG dunkel"),
        Theme::HighContrast => tr(lang, "High contrast", "Hoher Kontrast"),
    }
}
