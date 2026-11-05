//! Settings and scores on disk.

use std::fs;
use std::io::Write;
use std::path::PathBuf;

use directories::ProjectDirs;
use mr_sim::settings::{Lang, Settings};
use serde::{Deserialize, Serialize};

fn dirs() -> Option<ProjectDirs> {
    static MOVED: std::sync::Once = std::sync::Once::new();
    MOVED.call_once(move_old_dirs);
    ProjectDirs::from("", "", "morsehunter")
}

/// The program was called morse-runner-rs before: its settings and scores move along once.
fn move_old_dirs() {
    let (Some(old), Some(new)) = (ProjectDirs::from("", "", "morse-runner-rs"), ProjectDirs::from("", "", "morsehunter")) else { return };
    for (from, to) in [(old.config_dir(), new.config_dir()), (old.data_dir(), new.data_dir())] {
        if from.is_dir() && !to.exists() {
            if let Some(parent) = to.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = fs::rename(from, to);
        }
    }
}

pub fn config_path() -> Option<PathBuf> {
    Some(dirs()?.config_dir().join("config.toml"))
}

pub fn scores_path() -> Option<PathBuf> {
    Some(dirs()?.data_dir().join("scores.jsonl"))
}

pub fn wav_path() -> Option<PathBuf> {
    Some(dirs()?.data_dir().join("recording.wav"))
}

/// Language follows the system until the operator picks one.
fn system_lang() -> Lang {
    let lang = std::env::var("LC_ALL")
        .or_else(|_| std::env::var("LC_MESSAGES"))
        .or_else(|_| std::env::var("LANG"))
        .ok()
        .filter(|l| !l.is_empty() && l != "C" && l != "POSIX")
        // an app started from the Finder has no LANG, the system knows it all the same
        .or_else(sys_locale::get_locale)
        .unwrap_or_default();
    Lang::from_locale(&lang)
}

/// The settings, and whether this is the first start.
pub fn load_settings() -> (Settings, bool) {
    let text = config_path().and_then(|p| fs::read_to_string(p).ok());
    match text.and_then(|t| toml::from_str::<Settings>(&t).ok()) {
        Some(mut s) => {
            s.clamp();
            (s, false)
        }
        None => (Settings { lang: system_lang(), ..Settings::default() }, true),
    }
}

pub fn save_settings(s: &Settings) {
    let Some(p) = config_path() else { return };
    let result = toml::to_string_pretty(s).map_err(|e| e.to_string()).and_then(|t| {
        if let Some(d) = p.parent() {
            fs::create_dir_all(d).map_err(|e| e.to_string())?;
        }
        fs::write(&p, t).map_err(|e| e.to_string())
    });
    if let Err(e) = result {
        eprintln!("settings: {e}");
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScoreLine {
    pub date: String,
    pub call: String,
    pub operator: String,
    pub mode: String,
    pub qsos: u64,
    pub mults: u64,
    pub score: u64,
    /// The run as a code, to try it again or pass it on.
    #[serde(default)]
    pub code: String,
    /// Started from a code. Missing in old lines: those were all WPX, HST or from a code.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixed: Option<bool>,
    /// Skill points, if the run ended by itself.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skill: Option<SkillLine>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkillLine {
    pub per_hour: u64,
    pub points: u64,
    pub good: u32,
    pub busted: u32,
    pub asks: u32,
    pub wpm: u32,
}

impl From<mr_sim::skill::Skill> for SkillLine {
    fn from(s: mr_sim::skill::Skill) -> Self {
        SkillLine { per_hour: s.per_hour, points: s.points, good: s.good, busted: s.busted, asks: s.asks, wpm: s.wpm }
    }
}

impl ScoreLine {
    /// Does the line belong into a table of contest scores? A practice run without a code
    /// has only skill points.
    pub fn has_score(&self) -> bool {
        matches!(self.mode.as_str(), "WPX" | "HST") || self.fixed.unwrap_or(true)
    }
}

pub fn load_scores() -> Vec<ScoreLine> {
    let text = scores_path().and_then(|p| fs::read_to_string(p).ok()).unwrap_or_default();
    text.lines().filter_map(|l| serde_json::from_str(l).ok()).collect()
}

pub fn append_score(s: &ScoreLine) {
    let Some(p) = scores_path() else { return };
    let Ok(line) = serde_json::to_string(s) else { return };
    if let Some(d) = p.parent() {
        let _ = fs::create_dir_all(d);
    }
    if let Ok(mut f) = fs::OpenOptions::new().create(true).append(true).open(p) {
        let _ = writeln!(f, "{line}");
    }
}

/// Today as `YYYY-MM-DD`, UTC.
pub fn today() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let z = (secs / 86400) as i64 + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}
