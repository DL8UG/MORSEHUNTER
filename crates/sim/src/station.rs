//! What all stations have in common: turning a message into a keyed envelope and the
//! timer that decides what happens next.

use std::f32::consts::TAU;

use crate::msg::{Msg, MsgSet};
use crate::rng::Rng;
use crate::{BLOCK, RATE, morse};

pub const NEVER: i32 = i32::MAX;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Listening,
    Copying,
    PreparingToSend,
    Sending,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Timeout,
    MsgSent,
    MeStarted,
    MeFinished,
}

#[derive(Clone, Debug)]
pub struct Station {
    pub my_call: String,
    pub his_call: String,
    pub nr: u32,
    pub rst: u32,
    pub amplitude: f32,
    pub wpm: u32,
    pub state: State,
    pub timeout: i32,
    pub msg: MsgSet,
    pub msg_text: String,
    pub nr_with_error: bool,
    pub envelope: Vec<f32>,
    pub send_pos: usize,
    /// Its open line in the record of what was sent.
    pub(crate) sent_line: Option<u64>,
    bfo: f32,
    d_phi: f32,
}

impl Station {
    pub fn new() -> Self {
        Station {
            my_call: String::new(),
            his_call: String::new(),
            nr: 0,
            rst: 599,
            amplitude: 0.0,
            wpm: 30,
            state: State::Listening,
            timeout: NEVER,
            msg: MsgSet::empty(),
            msg_text: String::new(),
            nr_with_error: false,
            envelope: Vec::new(),
            send_pos: 0,
            sent_line: None,
            bfo: 0.0,
            d_phi: 0.0,
        }
    }

    /// Offset from the receiver centre in Hz.
    pub fn set_pitch(&mut self, hz: i32) {
        self.d_phi = TAU * hz as f32 / RATE as f32;
    }

    #[inline]
    pub fn next_bfo(&mut self) -> f32 {
        let b = self.bfo;
        self.bfo += self.d_phi;
        if self.bfo > TAU {
            self.bfo -= TAU;
        }
        if self.bfo < 0.0 {
            self.bfo += TAU;
        }
        b
    }

    /// Next piece of the envelope, returns its length. An ended envelope is cleared, the
    /// following `tick` then reports the message as sent.
    pub fn get_block(&mut self, out: &mut [f32; BLOCK]) -> usize {
        let from = self.send_pos.min(self.envelope.len());
        let to = (from + BLOCK).min(self.envelope.len());
        let n = to - from;
        out[..n].copy_from_slice(&self.envelope[from..to]);
        self.send_pos += BLOCK;
        if self.send_pos >= self.envelope.len() {
            self.envelope.clear();
        }
        n
    }

    /// Adds a message to what is being sent.
    pub fn send_msg(&mut self, m: Msg, rng: &mut Rng, hst: bool) {
        if self.envelope.is_empty() {
            self.msg = MsgSet::empty();
        }
        if m == Msg::None {
            self.state = State::Listening;
            return;
        }
        self.msg.add(m);
        self.send_text(m.template(), rng, hst);
    }

    /// Replaces the placeholders and keys the text, after what is still going out.
    pub fn send_text(&mut self, text: &str, rng: &mut Rng, hst: bool) {
        let mut t = text.to_string();
        if t.contains("<#>") {
            // the first one may carry an error, the second one is clean
            let first = self.nr_as_text(rng, hst);
            t = t.replacen("<#>", &first, 1);
            if t.contains("<#>") {
                let again = self.nr_as_text(rng, hst);
                t = t.replace("<#>", &again);
            }
        }
        t = t.replace("<my>", &self.my_call).replace("<his>", &self.his_call);
        if self.msg_text.is_empty() {
            self.msg_text = t;
        } else {
            self.msg_text.push(' ');
            self.msg_text.push_str(&t);
        }
        self.key_text();
    }

    fn key_text(&mut self) {
        if self.envelope.is_empty() {
            self.send_pos = 0;
            self.bfo = 0.0;
        }
        self.envelope = self.make_envelope(&self.msg_text);
        self.state = State::Sending;
        self.timeout = NEVER;
    }

    pub fn make_envelope(&self, text: &str) -> Vec<f32> {
        let mut e = morse::envelope(&morse::encode(text), self.wpm, RATE, BLOCK);
        e.iter_mut().for_each(|v| *v *= self.amplitude);
        e
    }

    /// Report and serial number as this station keys them.
    pub fn nr_as_text(&mut self, rng: &mut Rng, hst: bool) -> String {
        let mut s = format!("{}{:03}", self.rst, self.nr);
        if self.nr_with_error {
            self.nr_with_error = false;
            let in_range = |b: u8| (b'2'..=b'7').contains(&b);
            let mut i = s.len() - 1;
            if !in_range(s.as_bytes()[i]) {
                i -= 1;
            }
            let b = s.as_bytes()[i];
            if in_range(b) {
                let wrong = if rng.chance(0.5) { b - 1 } else { b + 1 };
                s.replace_range(i..=i, &(wrong as char).to_string());
                s += &format!("EEEEE {:03}", self.nr);
            }
        }
        s = s.replace("599", "5NN");
        if !hst {
            s = s.replace("000", "TTT").replace("00", "TT");
            if rng.chance(0.4) {
                s = s.replace('0', "O");
            } else if rng.chance(0.97) {
                s = s.replace('0', "T");
            }
            if rng.chance(0.97) {
                s = s.replace('9', "N");
            }
        }
        s
    }

    /// One block of time passed; the event that results from it.
    pub fn tick(&mut self) -> Option<Event> {
        if self.state == State::Sending {
            if self.envelope.is_empty() {
                self.msg_text.clear();
                self.state = State::Listening;
                return Some(Event::MsgSent);
            }
            return None;
        }
        if self.timeout > -1 {
            self.timeout -= 1;
        }
        (self.timeout == 0).then_some(Event::Timeout)
    }
}

impl Default for Station {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nr(rst: u32, nr: u32, error: bool, seed: u64, hst: bool) -> String {
        let mut st = Station { rst, nr, nr_with_error: error, ..Station::new() };
        st.nr_as_text(&mut Rng::new(seed), hst)
    }

    #[test]
    fn cut_numbers() {
        let seen: Vec<String> = (0..200).map(|s| nr(599, 100, false, s, false)).collect();
        assert!(seen.iter().any(|s| s == "5NN1TT"));
        for s in &seen {
            assert!(s.starts_with("5NN1") || s.starts_with("5991"), "{s}");
            assert!(s.ends_with("TT"), "{s}");
        }
        assert_eq!(nr(599, 7, false, 1, true), "5NN007");
    }

    #[test]
    fn hst_keeps_digits() {
        assert_eq!(nr(599, 100, false, 3, true), "5NN100");
    }

    #[test]
    fn error_is_corrected() {
        let mut found = false;
        for s in 0..50 {
            let t = nr(599, 123, true, s, true);
            assert!(t.ends_with("EEEEE 123"), "{t}");
            assert!(!t.starts_with("5NN123"), "{t}");
            found = true;
        }
        assert!(found);
    }

    #[test]
    fn second_number_is_clean() {
        let mut st = Station { rst: 599, nr: 123, nr_with_error: true, amplitude: 1.0, ..Station::new() };
        st.send_msg(Msg::RNr2, &mut Rng::new(9), true);
        assert_eq!(st.msg_text.matches("EEEEE").count(), 1);
        assert!(st.msg_text.ends_with("5NN123"), "{}", st.msg_text);
    }

    #[test]
    fn message_runs_out_and_reports() {
        let mut st = Station { my_call: "W1AW".into(), amplitude: 1.0, ..Station::new() };
        st.send_msg(Msg::MyCall, &mut Rng::new(1), false);
        let mut blk = [0.0; BLOCK];
        let mut n = 0;
        while !st.envelope.is_empty() {
            assert_eq!(st.tick(), None);
            st.get_block(&mut blk);
            n += 1;
        }
        assert!(n > 10);
        assert_eq!(st.tick(), Some(Event::MsgSent));
        assert_eq!(st.state, State::Listening);
    }

    #[test]
    fn repeated_message_goes_on_after_the_first() {
        let mut st = Station { my_call: "W1AW".into(), amplitude: 1.0, ..Station::new() };
        let mut rng = Rng::new(1);
        st.send_msg(Msg::MyCall, &mut rng, false);
        st.send_msg(Msg::MyCall, &mut rng, false);
        assert_eq!(st.msg_text, "W1AW W1AW");
    }

    #[test]
    fn timeout_fires_once() {
        let mut st = Station { timeout: 2, ..Station::new() };
        assert_eq!(st.tick(), None);
        assert_eq!(st.tick(), Some(Event::Timeout));
        assert_eq!(st.tick(), None);
        assert_eq!(st.tick(), None);
    }
}
