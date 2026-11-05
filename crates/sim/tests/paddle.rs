//! Contacts keyed by hand: a script produces the key edges a keyer would.

use mr_sim::contest::{Contest, RunMode};
use mr_sim::dx::OpState;
use mr_sim::entry::Key;
use mr_sim::log::QsoErr;
use mr_sim::morse;
use mr_sim::msg::Msg;
use mr_sim::sent::Sender;
use mr_sim::settings::Settings;
use mr_sim::station::State;
use mr_sim::{BLOCK, RATE, seconds_to_blocks};

const CARD: u64 = 48000;

struct Hand {
    c: Contest,
    t: u64,
    edges: Vec<(u64, bool)>,
}

impl Hand {
    fn new(seed: u64) -> Self {
        let s = Settings {
            call: "DL8UG".into(),
            duration: 60,
            wpm: 25,
            paddle: true,
            qrn: false,
            qrm: false,
            qsb: false,
            flutter: false,
            lids: false,
            ..Settings::default()
        };
        let mut c = Contest::new(s, seed);
        c.run(RunMode::Pileup);
        Hand { c, t: 0, edges: Vec::new() }
    }

    fn tick(&mut self) {
        let mut b = [0f32; BLOCK];
        self.t += BLOCK as u64 * CARD / RATE as u64;
        let (now, later): (Vec<_>, Vec<_>) = self.edges.iter().partition(|e| e.0 <= self.t);
        self.edges = later;
        for (t, down) in now {
            self.c.paddle_edge(t, down);
        }
        self.c.block(&mut b);
        self.c.paddle_poll(self.t);
    }

    /// Queues the edges of a text at the set speed, from now on.
    fn key(&mut self, text: &str) {
        let unit = CARD * 12 / (10 * self.c.settings.wpm as u64);
        let mut t = self.t + unit;
        for p in morse::patterns(text) {
            let Some(p) = p else {
                t += 4 * unit;
                continue;
            };
            for e in p.chars() {
                self.edges.push((t, true));
                t += if e == '-' { 3 * unit } else { unit };
                self.edges.push((t, false));
                t += unit;
            }
            t += 2 * unit;
        }
    }

    /// Until the over went out and everybody who wants to has answered.
    fn exchange(&mut self, text: &str) {
        self.key(text);
        let quiet = |h: &Hand| {
            h.edges.is_empty() && h.c.me.st.state != State::Sending && h.c.stations.iter().all(|s| s.st.state != State::Sending)
        };
        let mut still = 0;
        for _ in 0..seconds_to_blocks(60.0) {
            self.tick();
            still = if quiet(self) { still + 1 } else { 0 };
            if still > seconds_to_blocks(1.0) {
                return;
            }
        }
        panic!("the band never went quiet");
    }

    fn caller(&mut self) -> String {
        for _ in 0..30 {
            self.exchange("CQ TEST DL8UG");
            let found = self.c.stations.iter().find(|s| s.dx().is_some_and(|d| d.op.state == OpState::NeedQso));
            if let Some(s) = found {
                return s.st.my_call.clone();
            }
        }
        panic!("nobody called");
    }

    fn typed(&mut self, t: &str) {
        for c in t.chars() {
            self.c.key(Key::Char(c));
        }
    }

    fn state_of(&self, call: &str) -> Option<OpState> {
        self.c.stations.iter().find(|s| s.st.my_call == call).and_then(|s| s.dx()).map(|d| d.op.state)
    }

    fn nr_of(&self, call: &str) -> u32 {
        self.c.stations.iter().find(|s| s.st.my_call == call).map(|s| s.st.nr).unwrap()
    }
}

#[test]
fn a_keyed_contact_ends_in_a_clean_log() {
    let mut h = Hand::new(1);
    let call = h.caller();
    h.typed(&call);
    h.exchange(&format!("{call} 5NN 001"));
    assert!(h.c.entry.call_sent && h.c.entry.nr_sent);
    // one time in ten he misses the number and asks
    if h.state_of(&call) == Some(OpState::NeedNr) {
        h.exchange("5NN 001");
    }
    assert_eq!(h.state_of(&call), Some(OpState::NeedEnd));
    h.c.key(Key::Char(' '));
    let nr = h.nr_of(&call).to_string();
    h.typed(&nr);
    h.exchange("TU");
    assert_eq!(h.c.log.qsos.len(), 1, "keyed TU logs it");
    assert_eq!(h.c.log.qsos[0].err, QsoErr::Ok);
    assert!(h.c.entry.call.text.is_empty());
    assert_eq!(h.c.me.st.nr, 2);
}

#[test]
fn keyed_tu_without_the_option_leaves_the_fields() {
    let mut h = Hand::new(2);
    h.c.settings.paddle_tu_saves = false;
    let call = h.caller();
    h.typed(&call);
    h.exchange(&format!("{call} 5NN 001"));
    if h.state_of(&call) == Some(OpState::NeedNr) {
        h.exchange("5NN 001");
    }
    h.c.key(Key::Char(' '));
    let nr = h.nr_of(&call).to_string();
    h.typed(&nr);
    h.exchange("TU");
    assert!(h.c.log.qsos.is_empty());
    h.c.key(Key::SaveEnter);
    assert_eq!(h.c.log.qsos.len(), 1);
    h.exchange("CQ TEST DL8UG");
    assert_eq!(h.c.log.qsos[0].err, QsoErr::Ok);
}

#[test]
fn keyed_wrong_call_is_noticed_by_the_station() {
    let mut h = Hand::new(3);
    let call = h.caller();
    let mut wrong = call.clone();
    let last = wrong.pop().unwrap();
    wrong.push(if last == 'X' { 'Y' } else { 'X' });
    h.exchange(&format!("{wrong} 5NN 001"));
    // he asks for the correction with his call
    assert_eq!(h.state_of(&call), Some(OpState::NeedCall));
}

#[test]
fn keyed_partial_call_gets_a_reply() {
    let mut h = Hand::new(4);
    let call = h.caller();
    h.exchange(&format!("{}?", &call[..2]));
    assert!(matches!(h.state_of(&call), Some(OpState::NeedCallNr | OpState::NeedNr)), "{:?}", h.state_of(&call));
    assert!(h.c.me.st.his_call.ends_with('?'));
}

#[test]
fn a_keyed_partial_call_works_like_on_the_keyboard() {
    for seed in [4, 5, 6, 7] {
        let mut h = Hand::new(seed);
        let call = h.caller();
        let partial = format!("{}?", &call[..2]);
        h.typed(&partial);
        h.exchange(&partial);
        assert!(!h.c.me.st.msg.has(Msg::Qm) && !h.c.me.st.msg.has(Msg::Nr), "only the call: {:?}", h.c.me.st.msg);
        assert_eq!(h.state_of(&call), Some(OpState::NeedCallNr), "seed {seed}");
        let repeated: Vec<String> = h.c.sent.lines().filter(|l| l.who == Sender::Caller && l.call == call).map(|l| l.text.clone()).collect();
        let last = repeated.last().expect("he answers");
        assert!(last.contains(&call) && !last.contains("5NN"), "he gives his call again: {last}");
        // the call is complete now, in the field and on the air
        h.c.key(Key::End);
        h.c.key(Key::Backspace);
        h.typed(&call[2..]);
        h.exchange(&format!("{call} 5NN 001"));
        if h.state_of(&call) == Some(OpState::NeedNr) {
            h.exchange("5NN 001");
        }
        assert_eq!(h.state_of(&call), Some(OpState::NeedEnd), "seed {seed}");
        h.c.key(Key::Char(' '));
        let nr = h.nr_of(&call).to_string();
        h.typed(&nr);
        h.exchange("TU");
        h.exchange("CQ TEST DL8UG");
        let q = &h.c.log.qsos[0];
        assert_eq!((q.call.as_str(), q.err, q.asks), (call.as_str(), QsoErr::Ok, 0), "seed {seed}");
    }
}

#[test]
fn the_hand_stops_a_message_from_the_keyboard() {
    let mut h = Hand::new(5);
    h.c.key(Key::F(1));
    for _ in 0..5 {
        h.tick();
    }
    assert_eq!(h.c.me.st.state, State::Sending);
    assert!(!h.c.me.paddle_over);
    h.key("E");
    for _ in 0..5 {
        h.tick();
    }
    assert!(h.c.me.paddle_over);
    assert!(h.c.me.st.envelope.is_empty());
}

#[test]
fn keyboard_takes_over_after_keying() {
    let mut h = Hand::new(6);
    h.key("TEST");
    for _ in 0..seconds_to_blocks(0.8) {
        h.tick();
    }
    assert!(h.c.me.paddle_over);
    while !h.edges.is_empty() {
        h.tick();
    }
    h.c.key(Key::F(1));
    assert!(!h.c.me.paddle_over);
    assert!(h.c.me.st.msg.has(Msg::Cq));
    assert!(!h.c.me.st.envelope.is_empty());
}

#[test]
fn noise_keyed_into_an_exchange_counts_as_garbage() {
    let mut h = Hand::new(7);
    h.exchange("XQZ");
    assert!(h.c.me.st.msg.has(Msg::Garbage));
    assert_eq!(h.c.paddle.last_over, "XQZ");
}

#[test]
fn receiver_goes_quiet_without_qsk() {
    let mut h = Hand::new(8);
    h.c.settings.qsk = false;
    h.key("T");
    let mut level = 0.0;
    for _ in 0..9 {
        h.tick();
    }
    for _ in 0..3 {
        let mut b = [0f32; BLOCK];
        h.c.block(&mut b);
        level = b.iter().fold(level, |p: f32, v| p.max(v.abs()));
    }
    // the filter has run empty
    assert!(h.c.me.paddle_over);
    assert!(level < 0.001, "{level}");
}

#[test]
fn the_record_shows_a_keyed_over_as_read() {
    let mut h = Hand::new(5);
    h.exchange("CQ TEST DL8UG");
    let mine: Vec<_> = h.c.sent.lines().filter(|l| l.who == Sender::Me).collect();
    assert_eq!(mine.len(), 1);
    assert_eq!(mine[0].text, "CQ TEST DL8UG");
    assert!(mine[0].hand && !mine[0].on_air && !mine[0].cut);
}

#[test]
fn the_record_shows_a_keyed_over_while_it_goes_out() {
    let mut h = Hand::new(6);
    h.key("TEST");
    let mut seen = false;
    for _ in 0..seconds_to_blocks(0.8) {
        h.tick();
        seen |= h.c.sent.lines().any(|l| l.hand && l.on_air && !l.text.is_empty());
    }
    assert!(seen);
}

#[test]
fn the_record_keeps_a_keyed_over_and_the_message_after_it_apart() {
    let mut h = Hand::new(6);
    h.key("TEST");
    while !h.edges.is_empty() {
        h.tick();
    }
    h.c.key(Key::F(1));
    for _ in 0..seconds_to_blocks(0.3) {
        h.tick();
    }
    let mine: Vec<_> = h.c.sent.lines().filter(|l| l.who == Sender::Me).collect();
    assert_eq!(mine.len(), 2);
    assert!(mine[0].hand && !mine[1].hand);
    assert_eq!(mine[0].text, "TEST");
    assert_eq!(mine[1].text, "CQ DL8UG TEST");
}

#[test]
fn the_record_shows_the_message_a_hand_stopped() {
    let mut h = Hand::new(5);
    h.c.key(Key::F(1));
    for _ in 0..5 {
        h.tick();
    }
    h.key("E");
    for _ in 0..seconds_to_blocks(0.2) {
        h.tick();
    }
    let first = h.c.sent.lines().next().unwrap();
    assert!(first.cut && !first.hand);
}

#[test]
fn the_keying_is_measured_and_starts_afresh_with_a_run() {
    let mut h = Hand::new(2);
    h.exchange("CQ TEST DL8UG");
    let f = h.c.paddle.fist().clone();
    let near = |v: Option<f64>, want: f64| v.is_some_and(|v| (v - want).abs() < 0.15);
    assert_eq!(f.chars, 11, "{f:?}");
    assert!(near(f.letter_gaps.mean(), 3.0) && near(f.word_gaps.mean(), 7.0), "{f:?}");
    assert!(f.advice().is_empty(), "{:?}", f.advice());
    h.c.run(RunMode::Stop);
    h.c.run(RunMode::Pileup);
    assert_eq!(h.c.paddle.fist().chars, 0);
}
