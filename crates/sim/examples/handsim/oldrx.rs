//! The decoder as it was in the last commit, for comparison.
#![allow(dead_code)]
//! Turns the key signal of the operator into text. Times are in samples.

use mr_sim::morse;

/// Characters and spaces go into `text`, keyed errors take the last word out again.
pub struct RxText {
    /// Dit length in samples: the keyer's in paddle modes, measured otherwise.
    unit: f64,
    adaptive: bool,
    down_at: Option<u64>,
    up_at: Option<u64>,
    pattern: String,
    word_open: bool,
    pub text: String,
}

impl RxText {
    pub fn new(unit: f64, adaptive: bool) -> Self {
        RxText {
            unit,
            adaptive,
            down_at: None,
            up_at: None,
            pattern: String::new(),
            word_open: false,
            text: String::new(),
        }
    }

    pub fn unit(&self) -> f64 {
        self.unit
    }

    pub fn set_unit(&mut self, unit: f64, adaptive: bool) {
        self.adaptive = adaptive;
        if !adaptive || self.up_at.is_none() {
            self.unit = unit;
        }
    }

    pub fn key(&mut self, t: u64, down: bool) {
        self.poll(t);
        if down {
            self.down_at.get_or_insert(t);
            return;
        }
        let Some(start) = self.down_at.take() else { return };
        let len = (t - start) as f64;
        let dah = len > 2.0 * self.unit;
        self.pattern.push(if dah { '-' } else { '.' });
        if self.adaptive {
            let sample = if dah { len / 3.0 } else { len };
            self.unit += 0.2 * (sample - self.unit);
        }
        self.up_at = Some(t);
    }

    /// Closes the character and the word once the silence is long enough.
    pub fn poll(&mut self, t: u64) {
        if self.down_at.is_some() {
            return;
        }
        let Some(up) = self.up_at else { return };
        let gap = (t - up) as f64;
        if gap >= 2.0 * self.unit && !self.pattern.is_empty() {
            self.close_char();
        }
        if gap >= 5.0 * self.unit && self.word_open {
            self.text.push(' ');
            self.word_open = false;
        }
    }

    fn close_char(&mut self) {
        let p = std::mem::take(&mut self.pattern);
        // eight dits or more is the error sign, however many were sent
        if p.len() >= 7 && p.bytes().all(|b| b == b'.') {
            let kept = self.text.trim_end().rfind(' ').map_or(0, |i| i + 1);
            self.text.truncate(kept);
            self.word_open = false;
            return;
        }
        self.text.push_str(&morse::decode(&p).unwrap_or_else(|| "*".into()));
        self.word_open = true;
    }

    /// True while a character or the key is still open.
    pub fn busy(&self) -> bool {
        self.down_at.is_some() || !self.pattern.is_empty()
    }

    pub fn last_key(&self) -> Option<u64> {
        self.up_at
    }

    pub fn take_text(&mut self) -> String {
        self.word_open = false;
        self.up_at = None;
        std::mem::take(&mut self.text)
    }
}

