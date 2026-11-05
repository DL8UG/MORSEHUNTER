//! Turns the key signal of the operator into text. Times are in samples.
//!
//! Letters close after a gap of two dits. Where words end is decided from the hand's own
//! rhythm: a hand keeps its pause between letters even when a keyer makes the elements
//! faster, so the decoder learns that pause and puts a word gap only where the silence is
//! clearly longer. Like someone copying by ear, it may move a word gap once it knows the
//! sender better, so the text of an over is laid out again whenever the rhythm is learned.

use std::collections::VecDeque;

use crate::fist::Fist;
use crate::morse;

// The numbers below come from simulated hands keying contest overs through the keyer
// (`cargo run --release -p morse-runner-sim --example handsim`): textbook spacing, a hand
// with its own rhythm like 300 ms between letters and 650 ms between words at any keyer
// speed, hesitant and restless hands, straight keys off the set speed.

/// A word gap is this much longer than the hand's usual letter gap. 1.6 keeps textbook
/// spacing at exactly five dits.
const WORD_OVER_LETTER: f64 = 1.6;
/// The hand's usual letter gap: this quantile of its recent gaps between characters, low
/// because some of them are word gaps.
const LETTER_QUANTILE: f64 = 0.4;
/// An over ends after this many of the hand's letter gaps of silence ...
const OVER_OVER_LETTER: f64 = 3.6;
/// ... but no later than this, unless twelve dits are longer.
const OVER_MAX_S: f64 = 1.5;
/// Until three of its gaps are known, a hand gets this long before its over ends.
const OVER_NEW_HAND_S: f64 = 1.0;
/// A straight key or bug: the first marks after a speed change teach this fast ...
const FAST_RATE: f64 = 0.35;
const FAST_MARKS: u32 = 12;
/// Letter gaps remembered to learn the rhythm.
const GAPS_KEPT: usize = 24;

/// A decoded character and the silence before it, `None` where a word surely starts.
struct Char {
    text: String,
    gap: Option<f64>,
}

/// Characters and spaces go into `text`, keyed errors take the last word out again.
pub struct RxText {
    /// Dit length in samples: the keyer's in paddle modes, measured otherwise.
    unit: f64,
    /// A hand's dah, measured on its own: hands make them anywhere from 2.5 to 4 dits long.
    dah: f64,
    /// Marks measured since the speed was set: the first ones teach faster.
    learned: u32,
    adaptive: bool,
    down_at: Option<u64>,
    up_at: Option<u64>,
    pattern: String,
    /// The silence before the character being keyed.
    gap_before: Option<f64>,
    chars: Vec<Char>,
    /// Gaps between letters of this hand, the newest last.
    gaps: VecDeque<f64>,
    /// The silence after the last character is a word gap already.
    trailing: bool,
    /// The over so far, laid out with the rhythm known now.
    pub text: String,
    /// The dit of the set speed, what the hand is measured against.
    set: f64,
    /// How the hand keys, since the last reset.
    pub fist: Fist,
}

impl RxText {
    pub fn new(unit: f64, adaptive: bool) -> Self {
        RxText {
            unit,
            dah: 3.0 * unit,
            learned: 0,
            adaptive,
            down_at: None,
            up_at: None,
            pattern: String::new(),
            gap_before: None,
            chars: Vec::new(),
            gaps: VecDeque::new(),
            trailing: false,
            text: String::new(),
            set: unit,
            fist: Fist { by_hand: adaptive, ..Fist::default() },
        }
    }

    /// A new run: the keying is measured afresh.
    pub fn reset_fist(&mut self) {
        self.fist = Fist { by_hand: self.adaptive, ..Fist::default() };
    }

    pub fn unit(&self) -> f64 {
        self.unit
    }

    pub fn set_unit(&mut self, unit: f64, adaptive: bool) {
        self.adaptive = adaptive;
        self.set = unit;
        self.fist.by_hand = adaptive;
        if !adaptive || self.up_at.is_none() {
            if self.unit != unit {
                // a new speed, the hand's rhythm has to be learned again
                self.gaps.clear();
            }
            self.unit = unit;
            self.dah = 3.0 * unit;
            self.learned = 0;
        }
    }

    /// The hand's usual pause between letters: a low quantile of what it did, since most
    /// gaps between characters are letter gaps. Three dits until there is enough to go by.
    fn letter_gap(&self) -> f64 {
        let mut g: Vec<f64> = self.gaps.iter().copied().collect();
        g.push(3.0 * self.unit);
        g.sort_by(f64::total_cmp);
        let q = g[((g.len() - 1) as f64 * LETTER_QUANTILE) as usize];
        q.clamp(2.0 * self.unit, 12.0 * self.unit)
    }

    /// The silence that ends a word: five dits, or more for a hand with wide letter gaps.
    pub fn word_gap(&self) -> f64 {
        (5.0 * self.unit).max(WORD_OVER_LETTER * self.letter_gap())
    }

    /// The silence that ends an over, in samples at `rate`: twelve dits, or a few of this
    /// hand's letter gaps up to 1.5 s, as a station on the air waits for the other side to
    /// be really done.
    pub fn over_gap(&self, rate: f64) -> f64 {
        let by_hand = (OVER_OVER_LETTER * self.letter_gap()).min(OVER_MAX_S * rate);
        // a hand not known yet gets a second, as one waits longer for a new sender
        let floor = if self.gaps.len() < 3 { OVER_NEW_HAND_S * rate } else { 0.35 * rate };
        (12.0 * self.unit).max(by_hand).max(floor)
    }

    pub fn key(&mut self, t: u64, down: bool) {
        self.poll(t);
        if down {
            if self.down_at.is_none() && self.pattern.is_empty() {
                // a new character begins: remember the silence before it
                self.gap_before = self.up_at.map(|up| (t - up) as f64);
            } else if self.down_at.is_none()
                && let Some(up) = self.up_at
            {
                self.fist.element_gap((t - up) as f64 / self.set);
            }
            self.trailing = false;
            self.down_at.get_or_insert(t);
            return;
        }
        let Some(start) = self.down_at.take() else { return };
        let len = (t - start) as f64;
        // a dah is closer to the hand's dah than to its dit; the keyer's are exactly 1 and 3
        let dah = len > (self.unit + self.dah) / 2.0;
        self.pattern.push(if dah { '-' } else { '.' });
        self.fist.mark(len / self.set, dah);
        if self.adaptive {
            let k = if self.learned < FAST_MARKS { FAST_RATE } else { 0.2 };
            self.learned += 1;
            if dah {
                self.dah += k * (len - self.dah);
            } else {
                self.unit += k * (len - self.unit);
            }
            // keep the two apart, or one noisy element could pull them together
            self.dah = self.dah.clamp(1.8 * self.unit, 5.0 * self.unit);
        }
        self.up_at = Some(t);
    }

    /// Closes the character, and shows the word gap, once the silence is long enough.
    pub fn poll(&mut self, t: u64) {
        if self.down_at.is_some() {
            return;
        }
        let Some(up) = self.up_at else { return };
        let gap = (t - up) as f64;
        if gap >= 2.0 * self.unit && !self.pattern.is_empty() {
            self.close_char();
        }
        if !self.trailing && !self.chars.is_empty() && self.pattern.is_empty() && gap >= self.word_gap() {
            self.trailing = true;
            self.layout();
        }
    }

    fn close_char(&mut self) {
        let p = std::mem::take(&mut self.pattern);
        let gap = self.gap_before.take();
        // eight dits or more is the error sign, however many were sent
        if p.len() >= 7 && p.bytes().all(|b| b == b'.') {
            self.fist.error();
            let thr = self.word_gap();
            while let Some(c) = self.chars.pop() {
                if c.gap.is_none_or(|g| g >= thr) {
                    break;
                }
            }
            // what comes next starts a word of its own
            self.gap_before = None;
            self.trailing = false;
            self.layout();
            return;
        }
        if let Some(g) = gap {
            self.fist.gap(g / self.set, g >= self.word_gap());
        }
        if let Some(g) = gap
            && g >= 2.0 * self.unit
        {
            self.gaps.push_back(g);
            if self.gaps.len() > GAPS_KEPT {
                self.gaps.pop_front();
            }
        }
        let decoded = morse::decode(&p);
        self.fist.char(decoded.is_some());
        let text = decoded.unwrap_or_else(|| "*".into());
        // a character after an error sign starts a new word
        let gap = if self.chars.is_empty() { None } else { gap.or(Some(f64::INFINITY)) };
        self.chars.push(Char { text, gap });
        self.layout();
    }

    /// Writes the over out again with the word gap known now.
    fn layout(&mut self) {
        let thr = self.word_gap();
        let mut out = String::new();
        for (i, c) in self.chars.iter().enumerate() {
            if i > 0 && c.gap.is_none_or(|g| g >= thr) {
                out.push(' ');
            }
            out.push_str(&c.text);
        }
        if self.trailing && !out.is_empty() {
            out.push(' ');
        }
        self.text = out;
    }

    /// True while a character or the key is still open.
    pub fn busy(&self) -> bool {
        self.down_at.is_some() || !self.pattern.is_empty()
    }

    pub fn last_key(&self) -> Option<u64> {
        self.up_at
    }

    /// The over is done: its text, and a clean start for the next one. The hand's rhythm
    /// stays learned.
    pub fn take_text(&mut self) -> String {
        self.up_at = None;
        self.gap_before = None;
        self.trailing = false;
        self.chars.clear();
        std::mem::take(&mut self.text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Keys a text with exact timing, unit 1000 samples.
    fn send(rx: &mut RxText, t: &mut u64, text: &str, unit: u64) {
        for p in morse::patterns(text) {
            let Some(p) = p else {
                *t += 4 * unit;
                continue;
            };
            for e in p.chars() {
                rx.key(*t, true);
                *t += if e == '-' { 3 * unit } else { unit };
                rx.key(*t, false);
                *t += unit;
            }
            *t += 2 * unit;
        }
    }

    #[test]
    fn the_fist_of_a_textbook_hand() {
        let mut rx = RxText::new(1000.0, true);
        let mut t = 0;
        send(&mut rx, &mut t, "CQ TEST DL8UG", 1000);
        rx.poll(t + 20_000);
        let f = &rx.fist;
        let near = |v: Option<f64>, want: f64| v.is_some_and(|v| (v - want).abs() < 0.01);
        assert!(near(f.dits.mean(), 1.0) && near(f.dahs.mean(), 3.0), "{f:?}");
        assert!(near(f.element_gaps.mean(), 1.0) && near(f.letter_gaps.mean(), 3.0), "{f:?}");
        assert!(near(f.word_gaps.mean(), 7.0), "{f:?}");
        assert_eq!((f.chars, f.unknown, f.errors), (11, 0, 0));
        assert!(f.advice().is_empty(), "{:?}", f.advice());
        rx.reset_fist();
        assert_eq!(rx.fist.chars, 0);
    }

    #[test]
    fn round_trip() {
        let mut rx = RxText::new(1000.0, false);
        let mut t = 0;
        send(&mut rx, &mut t, "CQ POTA DE W1ABC/P <BK> 5NN?", 1000);
        rx.poll(t + 10_000);
        assert_eq!(rx.text.trim(), "CQ POTA DE W1ABC/P <BK> 5NN?");
    }

    #[test]
    fn error_sign_takes_out_the_word() {
        let mut rx = RxText::new(1000.0, false);
        let mut t = 0;
        send(&mut rx, &mut t, "W1ABC W1ABD <HH> W1ABE", 1000);
        rx.poll(t + 10_000);
        assert_eq!(rx.text.trim(), "W1ABC W1ABE");
    }

    /// Elements from a keyer at 24 wpm, the hand pausing about 300 ms between letters and
    /// 650 ms between words, as recorded from a real operator.
    #[test]
    fn a_hand_with_its_own_letter_rhythm_keeps_its_words() {
        let rate = 48000.0;
        let unit = 1.2 / 24.0 * rate;
        let mut rx = RxText::new(unit, false);
        let mut t = 0u64;
        let ms = |m: f64| (m * rate / 1000.0) as u64;
        for (wi, word) in "CQ CQ TEST DE DL8UG".split(' ').enumerate() {
            if wi > 0 {
                t += ms(650.0) - ms(300.0);
            }
            for p in morse::patterns(word).into_iter().flatten() {
                for e in p.chars() {
                    rx.key(t, true);
                    t += if e == '-' { 3 * unit as u64 } else { unit as u64 };
                    rx.key(t, false);
                    t += unit as u64;
                }
                t += ms(300.0) - unit as u64;
            }
        }
        rx.poll(t + ms(3000.0));
        assert_eq!(rx.text.trim(), "CQ CQ TEST DE DL8UG");
    }

    /// A straight key hand with short dahs, 2.5 dits, 30 % slower than the setting.
    #[test]
    fn short_dahs_of_a_slow_hand_are_dahs() {
        let mut rx = RxText::new(1000.0, true);
        let mut t = 0u64;
        let (dit, dah) = (1430u64, 3575u64);
        for p in morse::patterns("PARIS PARIS CQ TEST") {
            let Some(p) = p else {
                t += 4 * dit;
                continue;
            };
            for e in p.chars() {
                rx.key(t, true);
                t += if e == '-' { dah } else { dit };
                rx.key(t, false);
                t += dit;
            }
            t += 2 * dit;
        }
        rx.poll(t + 40_000);
        assert!(rx.text.ends_with("CQ TEST "), "{}", rx.text);
    }

    #[test]
    fn a_new_hand_gets_time_before_its_over_ends() {
        let rate = 48000.0;
        let mut rx = RxText::new(1.2 / 35.0 * rate, false);
        assert!(rx.over_gap(rate) >= OVER_NEW_HAND_S * rate);
        let mut t = 0;
        send(&mut rx, &mut t, "CQ TEST DL8UG", (1.2 / 35.0 * rate) as u64);
        // known now: twelve dits at 35 wpm, about 0.4 s
        assert!(rx.over_gap(rate) < 0.5 * rate, "{}", rx.over_gap(rate) / rate);
    }

    #[test]
    fn hand_speed_is_followed() {
        // decoder starts at 1000, the hand is at 1500
        let mut rx = RxText::new(1000.0, true);
        let mut t = 0;
        send(&mut rx, &mut t, "EEEEEETTTTTT PARIS PARIS", 1500);
        rx.poll(t + 20_000);
        assert!(rx.text.ends_with("PARIS PARIS "), "{}", rx.text);
    }
}
