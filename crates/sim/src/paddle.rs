//! Keying by hand. The stations only understand message sets, so a keyed over is read like a
//! contest operator would read it and turned into one.

use crate::contest::{Contest, RunMode};
use crate::keyer::Mode;
use crate::msg::{Msg, MsgSet};
use crate::parser;
use crate::rxtext::RxText;
use crate::settings::Settings;
use crate::station::State;

/// Key rate: edges come in samples of the sound card.
pub struct Paddle {
    rx: RxText,
    rate: f64,
    wpm: u32,
    mode: Mode,
    /// The time of the last poll.
    now: u64,
    /// What came out of the last over, for the display.
    pub last_over: String,
}

fn unit(wpm: u32, rate: f64) -> f64 {
    1.2 / wpm.max(1) as f64 * rate
}

fn adaptive(mode: Mode) -> bool {
    matches!(mode, Mode::Straight | Mode::Bug)
}

impl Paddle {
    pub fn new(s: &Settings) -> Self {
        let rate = 48000.0;
        Paddle {
            rx: RxText::new(unit(s.wpm, rate), adaptive(s.keyer.mode)),
            rate,
            wpm: s.wpm,
            mode: s.keyer.mode,
            now: 0,
            last_over: String::new(),
        }
    }

    pub fn set_rate(&mut self, rate: u32) {
        self.rate = rate as f64;
        self.rx.set_unit(unit(self.wpm, self.rate), adaptive(self.mode));
    }

    fn set_speed(&mut self, wpm: u32, mode: Mode) {
        self.wpm = wpm;
        self.mode = mode;
        self.rx.set_unit(unit(wpm, self.rate), adaptive(mode));
    }

    /// A new run.
    pub fn reset(&mut self) {
        self.rx.take_text();
        self.rx.reset_fist();
        self.last_over.clear();
    }

    /// How the operator keyed since the run began.
    pub fn fist(&self) -> &crate::fist::Fist {
        &self.rx.fist
    }

    /// The over so far.
    pub fn text(&self) -> &str {
        &self.rx.text
    }

    fn over_wait(&self) -> u64 {
        self.rx.over_gap(self.rate) as u64
    }
}

/// `W1?` and `?BC` as they were sent, the parser keeps only the letters.
fn partial_call(text: &str, partial: &str) -> String {
    let joined = text.replace(' ', "");
    if joined.contains(&format!("?{partial}")) { format!("?{partial}") } else { format!("{partial}?") }
}

fn own_base(call: &str) -> &str {
    call.split('/').max_by_key(|p| p.len()).unwrap_or(call)
}

/// What a keyed over amounts to.
pub fn read_over(text: &str, own_call: &str) -> (MsgSet, Option<String>) {
    let o = parser::parse(text, own_base(own_call));
    let has = |word: &str| text.split_whitespace().any(|t| t == word);
    let mut set = MsgSet::empty();
    let mut call = None;
    if o.cq || has("TEST") {
        set.add(Msg::Cq);
    }
    if let Some(c) = o.calls.last() {
        set.add(Msg::HisCall);
        call = Some(c.clone());
    } else if let Some(p) = o.partials.last() {
        set.add(Msg::HisCall);
        call = Some(partial_call(text, p));
    }
    let number = o.serial().is_some() || o.rst.is_some();
    if number {
        set.add(Msg::Nr);
    }
    if o.closing || (has("R") && !number) {
        set.add(Msg::Tu);
    }
    if o.own_call {
        set.add(Msg::MyCall);
    }
    if o.before {
        set.add(Msg::B4);
    }
    if has("NIL") {
        set.add(Msg::Nil);
    }
    if o.repeat {
        set.add(Msg::Qm);
    }
    if set.is_empty() {
        set.add(Msg::Garbage);
    }
    (set, call)
}

impl Contest {
    pub fn paddle_speed_changed(&mut self) {
        let (wpm, mode) = (self.settings.wpm, self.settings.keyer.mode);
        self.paddle.set_speed(wpm, mode);
    }

    /// A key edge, time in samples of the sound card.
    pub fn paddle_edge(&mut self, t: u64, down: bool) {
        if self.mode == RunMode::Stop {
            return;
        }
        self.paddle.now = t;
        if down && !self.me.paddle_over {
            // the hand on the key stops the machine
            if self.me.st.state == State::Sending {
                self.abort_send();
            }
            self.me.paddle_over = true;
            self.me.st.state = State::Sending;
            self.me.st.msg = MsgSet::empty();
            self.me.st.envelope.clear();
            self.on_me_started();
        }
        self.paddle.rx.key(t, down);
    }

    /// Time passes without an edge. Ends the over after a clear pause.
    pub fn paddle_poll(&mut self, t: u64) {
        if self.mode == RunMode::Stop {
            return;
        }
        self.paddle.now = t;
        self.paddle.rx.poll(t);
        if self.me.paddle_over
            && !self.paddle.rx.busy()
            && self.paddle.rx.last_key().is_some_and(|up| t.saturating_sub(up) >= self.paddle.over_wait())
        {
            self.finish_over();
        }
    }

    /// The over is complete, or a key on the keyboard wants the transmitter.
    pub fn finish_over(&mut self) {
        let t = self.paddle.now + (20.0 * self.paddle.rx.unit()) as u64;
        self.paddle.rx.poll(t);
        let text = self.paddle.rx.take_text();
        let text = text.trim().to_string();
        if !text.is_empty() {
            self.show_me(&text, true);
        }
        self.sent.close(&mut self.me.st.sent_line, false);
        let (set, call) = read_over(&text, &self.settings.call);
        self.paddle.last_over = text;
        self.me.paddle_over = false;
        self.me.st.state = State::Listening;
        self.me.st.msg = set;
        if let Some(c) = call {
            self.me.st.his_call = c;
        }
        if set.has(Msg::HisCall) {
            self.entry.call_sent = true;
        }
        if set.has(Msg::Nr) {
            self.entry.nr_sent = true;
        }
        if set.has(Msg::Tu) && self.settings.paddle_tu_saves && self.entry.complete() {
            self.save_qso();
        }
        self.on_me_finished();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(t: &str) -> MsgSet {
        read_over(t, "DL8UG").0
    }

    #[test]
    fn overs_become_messages() {
        for (t, want) in [
            ("CQ TEST DL8UG", vec![Msg::Cq, Msg::MyCall]),
            ("TEST DL8UG", vec![Msg::Cq, Msg::MyCall]),
            ("W1ABC 5NN 001", vec![Msg::HisCall, Msg::Nr]),
            ("5NN 12", vec![Msg::Nr]),
            ("TU", vec![Msg::Tu]),
            ("R", vec![Msg::Tu]),
            ("TU DL8UG", vec![Msg::Tu, Msg::MyCall]),
            ("DL8UG", vec![Msg::MyCall]),
            ("QSO B4", vec![Msg::B4]),
            ("NIL", vec![Msg::Nil]),
            ("?", vec![Msg::Qm]),
            ("AGN", vec![Msg::Qm]),
            ("NR?", vec![Msg::Qm]),
            ("W1?", vec![Msg::HisCall]),
        ] {
            let got = read(t);
            for m in &want {
                assert!(got.has(*m), "{t}: {m:?}");
            }
            assert!(!got.has(Msg::Garbage), "{t}");
        }
    }

    #[test]
    fn nothing_to_read_is_garbage() {
        assert!(read("XQZ *").has(Msg::Garbage));
        assert!(read("").has(Msg::Garbage));
    }

    #[test]
    fn calls_come_out_as_sent() {
        assert_eq!(read_over("W1ABC 5NN 001", "DL8UG").1.as_deref(), Some("W1ABC"));
        assert_eq!(read_over("W1?", "DL8UG").1.as_deref(), Some("W1?"));
        assert_eq!(read_over("?BC", "DL8UG").1.as_deref(), Some("?BC"));
        assert_eq!(read_over("TU", "DL8UG").1, None);
    }

    #[test]
    fn portable_own_call_is_recognised() {
        assert!(read_over("CQ DL8UG", "DL8UG/P").0.has(Msg::MyCall));
    }
}
