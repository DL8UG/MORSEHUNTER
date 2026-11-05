//! The three fields of the log line and everything the keyboard does, as one piece of plain
//! logic. The window only forwards keys, so tests can type.

use crate::contest::{Contest, RunMode};
use crate::msg::Msg;
use crate::settings::{BW_MAX, BW_MIN, RIT_MAX, STEP_HZ, WPM_MAX, WPM_MIN};

/// Digits the number field takes.
pub const NR_DIGITS: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    Call,
    Rst,
    Nr,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Char(char),
    Backspace,
    Delete,
    Left,
    Right,
    Home,
    End,
    Enter,
    /// Log without sending anything.
    SaveEnter,
    Tab,
    BackTab,
    Esc,
    /// F1 to F8.
    F(u8),
    Insert,
    Wipe,
    Rit(i32),
    Bandwidth(i32),
    Speed(i32),
}

/// One text field with a cursor and a selection of `sel` characters behind it.
#[derive(Clone, Debug, Default)]
pub struct Edit {
    pub text: String,
    pub cursor: usize,
    pub sel: usize,
}

impl Edit {
    fn set(&mut self, s: &str) {
        self.text = s.to_string();
        self.cursor = s.len();
        self.sel = 0;
    }

    fn clear(&mut self) {
        self.set("");
    }

    fn select(&mut self, at: usize, n: usize) {
        self.cursor = at;
        self.sel = n;
    }

    fn insert(&mut self, c: char, max: usize) -> bool {
        self.text.replace_range(self.cursor..self.cursor + self.sel, "");
        self.sel = 0;
        if self.text.len() >= max {
            return true;
        }
        self.text.insert(self.cursor, c);
        self.cursor += 1;
        true
    }

    fn backspace(&mut self) {
        if self.sel > 0 {
            self.text.replace_range(self.cursor..self.cursor + self.sel, "");
            self.sel = 0;
        } else if self.cursor > 0 {
            self.cursor -= 1;
            self.text.remove(self.cursor);
        }
    }

    fn delete(&mut self) {
        if self.sel > 0 {
            self.text.replace_range(self.cursor..self.cursor + self.sel, "");
            self.sel = 0;
        } else if self.cursor < self.text.len() {
            self.text.remove(self.cursor);
        }
    }

    fn go(&mut self, to: usize) {
        self.cursor = to.min(self.text.len());
        self.sel = 0;
    }
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub call: Edit,
    pub rst: Edit,
    pub nr: Edit,
    pub focus: Field,
    pub call_sent: bool,
    pub nr_sent: bool,
    must_advance: bool,
}

impl Default for Entry {
    fn default() -> Self {
        Entry {
            call: Edit::default(),
            rst: Edit::default(),
            nr: Edit::default(),
            focus: Field::Call,
            call_sent: false,
            nr_sent: false,
            must_advance: false,
        }
    }
}

impl Entry {
    fn edit(&mut self) -> &mut Edit {
        match self.focus {
            Field::Call => &mut self.call,
            Field::Rst => &mut self.rst,
            Field::Nr => &mut self.nr,
        }
    }

    pub fn wipe(&mut self) {
        self.call.clear();
        self.rst.clear();
        self.nr.clear();
        self.focus = Field::Call;
        self.call_sent = false;
        self.nr_sent = false;
    }

    /// Enough for a log entry?
    pub fn complete(&self) -> bool {
        self.call.text.len() >= 3 && self.rst.text.len() == 3 && !self.nr.text.is_empty()
    }

    pub fn set_focus(&mut self, f: Field) {
        self.focus = f;
        match f {
            // straight to the character in doubt
            Field::Call => {
                if let Some(p) = self.call.text.find('?')
                    && p > 0
                {
                    self.call.select(p, 1);
                }
            }
            // the middle digit is the one that changes
            Field::Rst => {
                if self.rst.text.len() == 3 {
                    self.rst.select(1, 1);
                }
            }
            Field::Nr => {}
        }
    }

    fn default_rst(&mut self) {
        if self.rst.text.is_empty() {
            self.rst.set("599");
        }
    }

    fn space(&mut self) {
        self.must_advance = false;
        match self.focus {
            Field::Call | Field::Rst => {
                self.default_rst();
                self.set_focus(Field::Nr);
            }
            Field::Nr => self.set_focus(Field::Call),
        }
    }

    fn tab(&mut self, back: bool) {
        let next = match (self.focus, back) {
            (Field::Call, false) | (Field::Nr, true) => Field::Rst,
            (Field::Rst, false) | (Field::Call, true) => Field::Nr,
            (Field::Nr, false) | (Field::Rst, true) => Field::Call,
        };
        self.set_focus(next);
    }

    /// After a message went out: on to where the next input belongs.
    pub fn advance(&mut self) {
        if !self.must_advance {
            return;
        }
        self.default_rst();
        if !self.call.text.contains('?') {
            self.set_focus(Field::Nr);
        } else {
            self.set_focus(Field::Call);
        }
        self.must_advance = false;
    }

    fn type_char(&mut self, c: char) -> bool {
        let c = c.to_ascii_uppercase();
        let (ok, max) = match self.focus {
            Field::Call => (c.is_ascii_uppercase() || c.is_ascii_digit() || c == '/' || c == '?', 12),
            Field::Rst => (c.is_ascii_digit(), 3),
            Field::Nr => (c.is_ascii_digit(), NR_DIGITS),
        };
        ok && self.edit().insert(c, max)
    }
}

impl Contest {
    /// The call changed: if it goes out right now, the rest of the message follows the edit.
    fn call_edited(&mut self) {
        let call = self.entry.call.text.clone();
        if call.is_empty() {
            self.entry.nr_sent = false;
        }
        if !self.update_call_in_message(&call) {
            self.entry.call_sent = false;
        }
    }

    /// Sends a message on behalf of the operator.
    pub fn send(&mut self, m: Msg) {
        if m == Msg::HisCall {
            if !self.entry.call.text.is_empty() {
                self.me.st.his_call = self.entry.call.text.clone();
            }
            self.entry.call_sent = true;
        }
        if m == Msg::Nr {
            self.entry.nr_sent = true;
        }
        self.me_send(m);
    }

    fn enter(&mut self) {
        self.entry.must_advance = false;
        if self.entry.call.text.is_empty() {
            self.send(Msg::Cq);
            return;
        }
        // a call with a gap goes out alone: the station fills it in before the exchange
        if self.entry.call.text.contains('?') {
            self.send(Msg::HisCall);
            self.entry.must_advance = true;
            return;
        }
        let (c, n, r) = (self.entry.call_sent, self.entry.nr_sent, !self.entry.nr.text.is_empty());
        if !c || (!n && !r) {
            self.send(Msg::HisCall);
        }
        if !n {
            self.send(Msg::Nr);
        }
        if n && !r {
            self.send(Msg::Qm);
        }
        if r && (c || n) {
            self.send(Msg::Tu);
            self.save_qso();
        } else {
            self.entry.must_advance = true;
        }
    }

    fn tu_and_save(&mut self) {
        if !self.entry.call_sent {
            self.send(Msg::HisCall);
        }
        self.send(Msg::Tu);
        self.save_qso();
    }

    fn call_and_number(&mut self) {
        self.send(Msg::HisCall);
        // no exchange to a call that is not complete yet
        if !self.entry.call.text.contains('?') {
            self.send(Msg::Nr);
        }
    }

    fn inc_rit(&mut self, d: i32) {
        self.rit = (self.rit + d * STEP_HZ as i32).clamp(-RIT_MAX, RIT_MAX);
    }

    /// Tunes the RIT onto a station `hz` off the pitch, to the nearest step.
    pub fn tune_to(&mut self, hz: f64) {
        let step = STEP_HZ as f64;
        self.rit = ((hz / step).round() * step).clamp(-RIT_MAX as f64, RIT_MAX as f64) as i32;
    }

    /// Next multiple of 5 up or down.
    fn step_speed(&mut self, dir: i32) {
        if self.fixed {
            return;
        }
        let w = self.settings.wpm;
        let v = if dir > 0 { w / 5 * 5 + 5 } else { w.div_ceil(5) * 5 - 5 };
        self.settings.wpm = v.clamp(WPM_MIN, WPM_MAX);
        self.me.st.wpm = self.settings.wpm;
        self.paddle_speed_changed();
        self.check_code();
    }

    fn step_bandwidth(&mut self, dir: i32) {
        if self.mode == RunMode::Hst || self.fixed {
            return;
        }
        let v = self.settings.bandwidth as i32 + dir * STEP_HZ as i32;
        if (BW_MIN as i32..=BW_MAX as i32).contains(&v) {
            self.settings.bandwidth = v as u32;
            self.filter_changed();
            self.check_code();
        }
    }

    pub fn key(&mut self, k: Key) {
        match k {
            Key::Char(';') | Key::Insert => self.call_and_number(),
            Key::Char('.' | '+' | '[' | ',') => self.tu_and_save(),
            Key::Char(' ') => self.entry.space(),
            Key::Char('\\') => self.send(Msg::Cq),
            Key::Char(c) => {
                let call = self.entry.focus == Field::Call;
                if self.entry.type_char(c) && call {
                    self.call_edited();
                }
            }
            Key::Backspace | Key::Delete => {
                let e = self.entry.edit();
                if k == Key::Backspace {
                    e.backspace();
                } else {
                    e.delete();
                }
                if self.entry.focus == Field::Call {
                    self.call_edited();
                }
            }
            Key::Left => {
                let e = self.entry.edit();
                e.go(e.cursor.saturating_sub(1));
            }
            Key::Right => {
                let e = self.entry.edit();
                e.go(e.cursor + 1);
            }
            Key::Home => self.entry.edit().go(0),
            Key::End => self.entry.edit().go(usize::MAX),
            Key::Enter => self.enter(),
            Key::SaveEnter => {
                self.entry.must_advance = false;
                self.save_qso();
            }
            Key::Tab => self.entry.tab(false),
            Key::BackTab => self.entry.tab(true),
            Key::Esc => {
                if self.me.st.msg.has(Msg::HisCall) {
                    self.entry.call_sent = false;
                }
                if self.me.st.msg.has(Msg::Nr) {
                    self.entry.nr_sent = false;
                }
                self.abort_send();
            }
            Key::F(n) => {
                let m = match n {
                    1 => Msg::Cq,
                    2 => Msg::Nr,
                    3 => Msg::Tu,
                    4 => Msg::MyCall,
                    5 => Msg::HisCall,
                    6 => Msg::B4,
                    7 => Msg::Qm,
                    8 => Msg::Nil,
                    _ => return,
                };
                self.send(m);
            }
            Key::Wipe => self.entry.wipe(),
            Key::Rit(d) => self.inc_rit(d),
            Key::Bandwidth(d) => self.step_bandwidth(d),
            Key::Speed(d) => self.step_speed(d),
        }
    }

    /// Zero beat: RIT back to the middle.
    pub fn rit_zero(&mut self) {
        self.rit = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn typed(s: &str) -> Entry {
        let mut e = Entry::default();
        for c in s.chars() {
            e.type_char(c);
        }
        e
    }

    #[test]
    fn call_field_takes_what_a_call_has() {
        assert_eq!(typed("w1-a?/x y").call.text, "W1A?/XY");
        assert_eq!(typed("ABCDEFGHIJKLMNOP").call.text.len(), 12);
    }

    #[test]
    fn numbers_only_in_number_fields() {
        let mut e = Entry { focus: Field::Nr, ..Entry::default() };
        for c in "1a2345".chars() {
            e.type_char(c);
        }
        assert_eq!(e.nr.text, "1234");
    }

    #[test]
    fn space_fills_the_report_and_jumps() {
        let mut e = typed("W1ABC");
        e.space();
        assert_eq!((e.rst.text.as_str(), e.focus), ("599", Field::Nr));
        e.space();
        assert_eq!(e.focus, Field::Call);
    }

    #[test]
    fn entering_the_report_selects_the_middle_digit() {
        let mut e = typed("W1ABC");
        e.rst.set("599");
        e.tab(false);
        assert_eq!(e.focus, Field::Rst);
        e.type_char('7');
        assert_eq!(e.rst.text, "579");
    }

    #[test]
    fn entering_the_call_selects_the_question_mark() {
        let mut e = typed("W1?BC");
        e.focus = Field::Nr;
        e.set_focus(Field::Call);
        e.type_char('A');
        assert_eq!(e.call.text, "W1ABC");
    }

    #[test]
    fn tab_runs_in_a_circle() {
        let mut e = Entry::default();
        for f in [Field::Rst, Field::Nr, Field::Call] {
            e.tab(false);
            assert_eq!(e.focus, f);
        }
        e.tab(true);
        assert_eq!(e.focus, Field::Nr);
    }

    #[test]
    fn editing_keys() {
        let mut e = typed("W1ABC");
        e.call.go(2);
        e.call.backspace();
        e.call.delete();
        assert_eq!(e.call.text, "WBC");
        assert_eq!(e.call.cursor, 1);
    }

    #[test]
    fn complete_needs_all_three() {
        let mut e = typed("W1A");
        assert!(!e.complete());
        e.rst.set("599");
        assert!(!e.complete());
        e.nr.set("7");
        assert!(e.complete());
        e.rst.set("59");
        assert!(!e.complete());
    }
}
