//! The window. Keys never pass through here, see `main.rs`: this only shows the contest
//! and offers the same actions to the mouse. One window, three pages: the operating position
//! the options and the help. Nothing opens on top or beside it.

mod analysis;
mod countries;
mod debug;
mod fields;
mod help;
mod i18n;
mod main_view;
mod setup;
pub mod theme;
mod waterfall;

use std::sync::atomic::Ordering::Relaxed;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use mr_sim::code::{CodeError, RunCode};
use mr_sim::contest::{Contest, Finished, RunMode};
use mr_sim::entry::{Edit, Field, Key};
use mr_sim::log::{HISTO_BINS, Qso, Stats};
use mr_sim::settings::{Lang, Settings};

use crate::audio::Shared;
use crate::store::{self, ScoreLine};

const LOG_SHOWN: usize = 400;
const FLASH: Duration = Duration::from_millis(250);
/// Height of the page tabs at the top of the window.
const PAGES_H: f32 = 30.0;

#[derive(Clone, Copy, PartialEq)]
pub enum Paddle {
    Dit,
    Dah,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Page {
    Operate,
    Options,
    Help,
}

/// Which list the score table shows. Runs only compare within one.
#[derive(Clone, Copy, PartialEq)]
enum Board {
    /// Every run that ended by itself, by skill points per hour.
    Skill,
    Wpx,
    Hst,
    /// Pile-up and single calls, they end with a result only when started from a code.
    Coded,
    /// The best skill per hour of each day, as a curve.
    Progress,
}

impl Board {
    /// The contest score table of a line.
    fn of(mode: &str) -> Board {
        match mode {
            "WPX" => Board::Wpx,
            "HST" => Board::Hst,
            _ => Board::Coded,
        }
    }

    fn holds(self, line: &ScoreLine) -> bool {
        match self {
            Board::Skill | Board::Progress => line.skill.is_some(),
            b => line.has_score() && Board::of(&line.mode) == b,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Log,
    Scores,
    /// Everything that is sent, only while the debug option is on.
    Sent,
    /// Copying errors and keying of the run.
    Analysis,
}

/// What a frame shows, copied so that the mutex is free while it is drawn.
struct View {
    mode: RunMode,
    secs: f64,
    rate: u32,
    dx: usize,
    raw: Stats,
    verified: Stats,
    hst: (u64, u64),
    call: Edit,
    rst: Edit,
    nr: Edit,
    focus: Field,
    rit: i32,
    next_nr: u32,
    qsos: Vec<Qso>,
    total: usize,
    histo: [u32; HISTO_BINS],
    keying: String,
    settings: Settings,
    code: Option<String>,
    fixed: bool,
    /// This run scores nothing, or the next one won't: what is sent is or was on show.
    unscored: bool,
    skill: mr_sim::skill::Skill,
    analysis: mr_sim::analysis::Analysis,
    fist: mr_sim::fist::Fist,
}

pub struct App {
    contest: Arc<Mutex<Contest>>,
    pub shared: Arc<Shared>,
    pub page: Page,
    /// Waiting for a key to put on a paddle.
    pub capture: Option<Paddle>,
    audio_error: Option<String>,
    scores: Vec<ScoreLine>,
    result: Option<Finished>,
    flash_until: Option<Instant>,
    tab: Tab,
    board: Board,
    /// The line of the run that ended last in this session, marked in the table.
    latest: Option<usize>,
    code_input: String,
    /// The language of the help page, until picked there it follows the window.
    help_lang: Option<Lang>,
    code_error: Option<CodeError>,
    vail: Vail,
    waterfall: waterfall::Waterfall,
}

/// The MIDI outputs seen in the Paddle card, and what the last command to the adapter did.
#[derive(Default)]
struct Vail {
    ports: Option<Result<Vec<String>, String>>,
    chosen: usize,
    status: Option<(bool, String)>,
}

/// A text in the language of the window. English and German stand where the text is
/// used, the other languages come from the table in `i18n`, by the English text.
pub fn tr(lang: Lang, en: &'static str, de: &'static str) -> &'static str {
    match lang {
        Lang::En => en,
        Lang::De => de,
        _ => i18n::lookup(lang, en),
    }
}

impl App {
    pub fn new(contest: Arc<Mutex<Contest>>, shared: Arc<Shared>, first_run: bool, audio_error: Option<String>) -> Self {
        App {
            contest,
            shared,
            page: if first_run { Page::Options } else { Page::Operate },
            capture: None,
            audio_error,
            scores: store::load_scores(),
            result: None,
            flash_until: None,
            tab: Tab::Log,
            board: Board::Skill,
            latest: None,
            code_input: String::new(),
            help_lang: None,
            code_error: None,
            vail: Vail::default(),
            waterfall: waterfall::Waterfall::default(),
        }
    }

    fn view(&self) -> View {
        let c = self.contest.lock().unwrap();
        let from = c.log.qsos.len().saturating_sub(LOG_SHOWN);
        let mut keying = c.paddle.text().to_string();
        if keying.is_empty() {
            keying = c.paddle.last_over.clone();
        }
        View {
            mode: c.mode,
            secs: c.seconds(),
            rate: c.rate(),
            dx: c.dx_count(),
            raw: c.log.raw(),
            verified: c.log.verified(),
            hst: c.log.hst(),
            call: c.entry.call.clone(),
            rst: c.entry.rst.clone(),
            nr: c.entry.nr.clone(),
            focus: c.entry.focus,
            rit: c.rit,
            next_nr: c.me.st.nr,
            qsos: c.log.qsos[from..].to_vec(),
            total: c.log.qsos.len(),
            histo: c.log.histogram(),
            keying,
            settings: c.settings.clone(),
            code: c.code.as_ref().map(RunCode::encode),
            fixed: c.fixed,
            unscored: c.peeked || (c.mode == RunMode::Stop && c.settings.debug),
            skill: mr_sim::skill::skill(&c.log.qsos, c.seconds()),
            analysis: mr_sim::analysis::analyse(&c.log.qsos),
            fist: c.paddle.fist().clone(),
        }
    }

    pub fn running(&self) -> bool {
        self.contest.lock().unwrap().mode != RunMode::Stop
    }

    /// A key from the window, handled before the next frame.
    pub fn key(&mut self, k: Key) {
        let beep = {
            let mut c = self.contest.lock().unwrap();
            c.key(k);
            std::mem::take(&mut c.beep)
        };
        if beep {
            self.flash_until = Some(Instant::now() + FLASH);
            self.shared.beep.store(true, Relaxed);
        }
    }

    pub fn start(&mut self, mode: RunMode) {
        if self.audio_error.is_none() {
            self.contest.lock().unwrap().run(mode);
            self.result = None;
            self.page = Page::Operate;
            if self.tab == Tab::Scores {
                self.tab = Tab::Log;
            }
        }
    }

    /// Starts the run a code describes.
    pub fn start_code(&mut self, code: &RunCode) {
        if self.audio_error.is_none() {
            self.contest.lock().unwrap().run_code(code);
            self.result = None;
            self.page = Page::Operate;
            if self.tab == Tab::Scores {
                self.tab = Tab::Log;
            }
        }
    }

    fn start_typed_code(&mut self) {
        match RunCode::decode(&self.code_input) {
            Ok(code) => {
                self.code_error = None;
                self.code_input.clear();
                self.start_code(&code);
            }
            Err(e) => self.code_error = Some(e),
        }
    }

    pub fn stop(&mut self) {
        self.contest.lock().unwrap().request_stop();
    }

    pub fn paddle_of(&self, name: &str) -> Option<u8> {
        let c = self.contest.lock().unwrap();
        let s = &c.settings;
        if !s.paddle {
            None
        } else if name == s.dit_key {
            Some(mr_sim::keyer::DIT)
        } else if name == s.dah_key {
            Some(mr_sim::keyer::DAH)
        } else {
            None
        }
    }

    /// Do Ctrl presses belong to the paddles? Then Ctrl can't be a modifier.
    pub fn ctrl_is_paddle(&self) -> bool {
        let c = self.contest.lock().unwrap();
        let s = &c.settings;
        s.paddle && (s.dit_key.starts_with("Control") || s.dah_key.starts_with("Control"))
    }

    pub fn captured(&mut self, name: String) {
        let which = self.capture.take();
        let mut c = self.contest.lock().unwrap();
        match which {
            Some(Paddle::Dit) => c.settings.dit_key = name,
            Some(Paddle::Dah) => c.settings.dah_key = name,
            None => {}
        }
    }

    pub fn save(&self) {
        store::save_settings(&self.contest.lock().unwrap().settings_to_keep());
    }

    fn take_finished(&mut self) {
        let Some(f) = self.contest.lock().unwrap().finished.take() else { return };
        let s = self.contest.lock().unwrap().settings.clone();
        let (mode, score) = match f.mode {
            RunMode::Hst => ("HST", f.hst_score),
            RunMode::Single => ("One", f.verified.score),
            RunMode::Pileup => ("Pile", f.verified.score),
            _ => ("WPX", f.verified.score),
        };
        // a run with what is sent on show leaves no trace in the table, and a practice run
        // without a good contact has nothing to rank
        let line = ScoreLine {
            date: store::today(),
            call: s.call.clone(),
            operator: s.operator.clone(),
            mode: mode.to_string(),
            qsos: f.verified.points,
            mults: f.verified.mults,
            score,
            code: f.code.clone().unwrap_or_default(),
            fixed: Some(f.fixed),
            skill: f.skill.filter(|k| k.good > 0).map(Into::into),
        };
        if !f.unscored && (line.has_score() || line.skill.is_some()) {
            store::append_score(&line);
            // a contest score shows in its own table, a practice run by its skill
            self.board = if line.has_score() { Board::of(&line.mode) } else { Board::Skill };
            self.latest = Some(self.scores.len());
            self.scores.push(line);
            // straight to the place the run got
            self.tab = Tab::Scores;
        }
        store::save_settings(&s);
        self.result = Some(f);
        // the result shows on the operating position
        self.page = Page::Operate;
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        self.take_finished();
        let view = self.view();
        if theme::select(ui.ctx(), view.settings.theme) {
            // this frame began in the old colours
            ui.ctx().request_repaint();
        }
        if !view.settings.debug && self.tab == Tab::Sent {
            self.tab = Tab::Log;
        }
        let area = ui.max_rect().shrink(16.0);
        let bar = egui::Rect::from_min_size(area.min, egui::vec2(area.width(), PAGES_H));
        self.page_tabs(ui, bar, &view);
        let rest = egui::Rect::from_min_max(egui::pos2(area.left(), bar.bottom() + 10.0), area.max);
        match self.page {
            Page::Operate => self.main_view(ui, rest, &view),
            Page::Options => self.setup(ui, rest, &view),
            Page::Help => self.help(ui, rest, &view),
        }
    }

    /// Operating position and options as two tabs, with a line under them.
    fn page_tabs(&mut self, ui: &mut egui::Ui, r: egui::Rect, v: &View) {
        let lang = v.settings.lang;
        ui.painter().line_segment([r.left_bottom(), r.right_bottom()], egui::Stroke::new(1.0, theme::pal().line));
        ui.scope_builder(egui::UiBuilder::new().max_rect(r), |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().button_padding = egui::vec2(14.0, 5.0);
                for (page, label) in [(Page::Operate, tr(lang, "Operate", "Betrieb")), (Page::Options, tr(lang, "Options", "Optionen")), (Page::Help, tr(lang, "Help", "Hilfe"))] {
                    let text = egui::RichText::new(label).size(15.0);
                    if ui.selectable_label(self.page == page, text).clicked() {
                        self.page = page;
                    }
                }
                if self.page != Page::Operate && v.mode != RunMode::Stop {
                    ui.add_space(12.0);
                    ui.label(egui::RichText::new(tr(lang, "A run is on, the keys still work it.", "Ein Lauf läuft, die Tasten bedienen ihn weiter.")).color(theme::pal().muted).size(13.0));
                }
            });
        });
    }
}
