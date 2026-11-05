//! What the user sets. Band conditions and the like, as the original keeps them in its ini
//! file. RIT stays out: it starts at zero with every run.

use serde::{Deserialize, Serialize};

use crate::callgen::WORLD_MIX;
use crate::keyer;

pub const WPM_MIN: u32 = 10;
pub const WPM_MAX: u32 = 120;
pub const PITCH_MIN: u32 = 300;
pub const PITCH_MAX: u32 = 900;
pub const BW_MIN: u32 = 100;
pub const BW_MAX: u32 = 600;
pub const STEP_HZ: u32 = 50;
pub const RIT_MAX: i32 = 500;
pub const ACTIVITY_MAX: u32 = 9;
pub const DURATION_MAX: u32 = 240;
/// Hours a contest may have been going when the run starts.
pub const HEAD_START_MAX: u32 = 48;
/// Speeds of the callers in percent of your own: the original goes from half to yours.
pub const CALLER_SLOW_MIN: u32 = 30;
pub const CALLER_SLOW_DEFAULT: u32 = 50;
pub const CALLER_FAST_DEFAULT: u32 = 100;
pub const CALLER_FAST_MAX: u32 = 150;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Lang {
    En,
    De,
    Fr,
    Es,
    Pt,
    It,
    Pl,
    Ru,
}

impl Lang {
    pub const ALL: [Lang; 8] = [Lang::En, Lang::De, Lang::Fr, Lang::Es, Lang::Pt, Lang::It, Lang::Pl, Lang::Ru];

    /// The name of the language in itself.
    pub fn name(self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::De => "Deutsch",
            Lang::Fr => "Français",
            Lang::Es => "Español",
            Lang::Pt => "Português",
            Lang::It => "Italiano",
            Lang::Pl => "Polski",
            Lang::Ru => "Русский",
        }
    }

    /// From a locale like `de_DE.UTF-8`, English for anything else.
    pub fn from_locale(locale: &str) -> Lang {
        let code = locale.get(..2).unwrap_or("").to_ascii_lowercase();
        Lang::ALL.into_iter().find(|l| format!("{l:?}").to_ascii_lowercase() == code).unwrap_or(Lang::En)
    }
}

/// The colours of the window.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Theme {
    /// The slate receiver at night, the look from the start.
    #[default]
    Night,
    /// The same receiver by day.
    Day,
    /// The paper and enamel green of dl8ug.de and its PDFs.
    Dl8ugLight,
    Dl8ugDark,
    /// Black, white and yellow.
    HighContrast,
}

impl Theme {
    pub const ALL: [Theme; 5] = [Theme::Night, Theme::Day, Theme::Dl8ugLight, Theme::Dl8ugDark, Theme::HighContrast];
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub call: String,
    pub operator: String,
    pub wpm: u32,
    pub pitch: u32,
    pub bandwidth: u32,
    pub qsk: bool,
    /// Sidetone level in dB, -60 to 20.
    pub mon_db: f32,
    pub activity: u32,
    pub qrn: bool,
    pub qrm: bool,
    pub qsb: bool,
    pub flutter: bool,
    pub lids: bool,
    /// Hours the contest has been going when the run starts: the callers' serial numbers
    /// start that high. 0 is the original, everybody starts with you.
    pub head_start: u32,
    /// The slowest and the fastest callers in percent of your speed, 30 to 100 and 100 to
    /// 150. The original is 50 and 100.
    pub caller_slow: u32,
    pub caller_fast: u32,
    /// Callers come in waves, with lulls and openings, instead of the same crowd all run.
    pub waves: bool,
    /// Run time in minutes.
    pub duration: u32,
    /// Run time of the two competition modes.
    pub competition_duration: u32,
    pub hi_score: u64,
    pub save_wav: bool,
    /// An external keyer sends the call, the program leaves a gap instead.
    pub calls_from_keyer: bool,
    pub region_mix: [f64; 6],
    pub lang: Lang,
    pub theme: Theme,
    pub paddle: bool,
    /// A keyed TU logs the contact when the fields are complete.
    pub paddle_tu_saves: bool,
    pub keyer: keyer::Config,
    /// winit key names of the paddles.
    pub dit_key: String,
    pub dah_key: String,
    /// Shows everything that goes out in a window of its own.
    pub debug: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            call: "N0CALL".into(),
            operator: String::new(),
            wpm: 30,
            pitch: 600,
            bandwidth: 500,
            qsk: true,
            mon_db: 0.0,
            activity: 2,
            qrn: true,
            qrm: true,
            qsb: true,
            flutter: true,
            lids: true,
            head_start: 0,
            caller_slow: CALLER_SLOW_DEFAULT,
            caller_fast: CALLER_FAST_DEFAULT,
            waves: false,
            duration: 30,
            competition_duration: 60,
            hi_score: 0,
            save_wav: false,
            calls_from_keyer: false,
            region_mix: WORLD_MIX,
            lang: Lang::En,
            theme: Theme::Night,
            paddle: false,
            paddle_tu_saves: true,
            keyer: keyer::Config::default(),
            dit_key: "ControlLeft".into(),
            dah_key: "ControlRight".into(),
            debug: false,
        }
    }
}

impl Settings {
    /// Brings values from a hand-edited file back into range.
    pub fn clamp(&mut self) {
        self.call = self.call.trim().to_uppercase();
        self.wpm = self.wpm.clamp(WPM_MIN, WPM_MAX);
        self.pitch = self.pitch.clamp(PITCH_MIN, PITCH_MAX) / STEP_HZ * STEP_HZ;
        self.bandwidth = self.bandwidth.clamp(BW_MIN, BW_MAX) / STEP_HZ * STEP_HZ;
        self.mon_db = self.mon_db.clamp(-60.0, 20.0);
        self.activity = self.activity.clamp(1, ACTIVITY_MAX);
        self.duration = self.duration.clamp(1, DURATION_MAX);
        self.head_start = self.head_start.min(HEAD_START_MAX);
        self.caller_slow = self.caller_slow.clamp(CALLER_SLOW_MIN, 100);
        self.caller_fast = self.caller_fast.clamp(100, CALLER_FAST_MAX);
        self.competition_duration = self.competition_duration.clamp(1, 60);
    }
}
