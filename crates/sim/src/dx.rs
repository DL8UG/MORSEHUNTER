//! The operator of a calling station. Delay, speed and exchange follow from his skills,
//! what he does next from what he thinks he heard.

use crate::contest::RunMode;
use crate::dsp::Qsb;
use crate::msg::{Msg, MsgSet};
use crate::rng::Rng;
use crate::seconds_to_blocks;
use crate::station::{NEVER, State, Station};

const FULL_PATIENCE: i32 = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpState {
    NeedPrevEnd,
    NeedQso,
    NeedNr,
    NeedCall,
    NeedCallNr,
    NeedEnd,
    Done,
    Failed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CallCheck {
    No,
    Yes,
    Almost,
}

/// What an operator knows about the contest around him.
// the highest number a caller can send must fit the number field
const _: () = assert!(
    1 + (crate::settings::HEAD_START_MAX * 60 + crate::settings::DURATION_MAX) * 3 < 10u32.pow(crate::entry::NR_DIGITS as u32),
    "caller numbers outgrow the number field"
);

#[derive(Clone, Copy, Debug)]
pub struct Env {
    pub mode: RunMode,
    pub lids: bool,
    pub wpm: u32,
    /// Minutes the contest has been going, the head start included.
    pub minute: f64,
    /// The slowest and the fastest callers in percent of `wpm`.
    pub slow: u32,
    pub fast: u32,
}

impl Env {
    fn hst(&self) -> bool {
        self.mode == RunMode::Hst
    }

    fn auto_callers(&self) -> bool {
        matches!(self.mode, RunMode::Single | RunMode::Hst)
    }
}

#[derive(Clone, Debug)]
pub struct Operator {
    pub call: String,
    pub skills: i32,
    pub patience: i32,
    pub repeat_cnt: u32,
    pub state: OpState,
}

/// Penalty of a sent call against the own one. `?` stands for any number of characters, the
/// start and the end of the sent call may be missing.
fn call_penalty(sent: &[u8], own: &[u8]) -> i32 {
    const W: i32 = 2;
    let (nx, ny) = (sent.len(), own.len());
    let mut m = vec![vec![0; ny + 1]; nx + 1];
    for x in 1..=nx {
        m[x][0] = m[x - 1][0] + W;
    }
    for x in 1..=nx {
        let c = sent[x - 1];
        for y in 1..=ny {
            let mut t = m[x][y - 1];
            if x < nx && c != b'?' {
                t += W;
            }
            let mut l = m[x - 1][y];
            if c != b'?' {
                l += W;
            }
            let mut d = m[x - 1][y - 1];
            if c != own[y - 1] && c != b'?' {
                d += W;
            }
            m[x][y] = t.min(l).min(d);
        }
    }
    m[nx][ny]
}

impl Operator {
    pub fn new(call: String, rng: &mut Rng, env: &Env) -> Self {
        let mut o = Operator {
            call,
            skills: 1 + rng.below(3) as i32,
            patience: 0,
            repeat_cnt: 1,
            state: OpState::NeedPrevEnd,
        };
        o.set_state(OpState::NeedPrevEnd, rng, env);
        o
    }

    pub fn send_delay(&self, rng: &mut Rng, env: &Env) -> i32 {
        if self.state == OpState::NeedPrevEnd {
            NEVER
        } else if env.hst() {
            seconds_to_blocks(0.05 + 0.5 * rng.f64() * 10.0 / env.wpm as f64)
        } else {
            seconds_to_blocks(0.1 + 0.5 * rng.f64())
        }
    }

    pub fn pick_wpm(rng: &mut Rng, env: &Env) -> u32 {
        if env.hst() {
            return env.wpm;
        }
        // with 50 and 100 % this is the original 0.5 * (1 + r), to the last bit
        let (lo, hi) = (env.slow as f64 / 100.0, env.fast as f64 / 100.0);
        ((env.wpm as f64 * (lo + (hi - lo) * rng.f64())).round() as u32).clamp(5, crate::settings::WPM_MAX)
    }

    /// As in the original: up to one to three contacts a minute since the contest began.
    pub fn pick_nr(&self, rng: &mut Rng, env: &Env) -> u32 {
        1 + (rng.f64() * env.minute * self.skills as f64).round() as u32
    }

    pub fn reply_timeout(&self, rng: &mut Rng, env: &Env) -> i32 {
        let t = if env.hst() { seconds_to_blocks(60.0 / env.wpm as f64) } else { seconds_to_blocks((6 - self.skills) as f64) };
        rng.gauss_lim(t as f64, t as f64 / 2.0).round() as i32
    }

    fn dec_patience(&mut self) {
        if self.state == OpState::Done {
            return;
        }
        self.patience -= 1;
        if self.patience < 1 {
            self.state = OpState::Failed;
        }
    }

    pub fn set_state(&mut self, s: OpState, rng: &mut Rng, env: &Env) {
        self.state = s;
        self.patience = if s == OpState::NeedQso { rng.rayleigh(4.0).round() as i32 } else { FULL_PATIENCE };
        self.repeat_cnt = if s == OpState::NeedQso && !env.auto_callers() && rng.chance(0.1) { 2 } else { 1 };
    }

    pub fn is_my_call(&self, sent: &str, rng: &mut Rng, env: &Env) -> CallCheck {
        let mut r = match call_penalty(sent.as_bytes(), self.call.as_bytes()) {
            0 => CallCheck::Yes,
            1 | 2 => CallCheck::Almost,
            _ => CallCheck::No,
        };
        if !env.lids && sent.len() == 2 && r == CallCheck::Almost {
            r = CallCheck::No;
        }
        // partial and wildcard matches cost nothing but are not exact
        if r == CallCheck::Yes && (sent.len() != self.call.len() || sent.contains('?')) {
            r = CallCheck::Almost;
        }
        if sent.chars().filter(|c| *c != '?').count() < 2 {
            r = CallCheck::No;
        }
        // accept a wrong call, or reject the correct one
        if env.lids && sent.len() > 3 {
            match r {
                CallCheck::Yes if rng.chance(0.01) => r = CallCheck::Almost,
                CallCheck::Almost if rng.chance(0.04) => r = CallCheck::Yes,
                _ => {}
            }
        }
        r
    }

    pub fn msg_received(&mut self, m: MsgSet, sent_call: &str, rng: &mut Rng, env: &Env) {
        use OpState::*;
        // a CQ means anybody can call, whatever else was sent
        if m.has(Msg::Cq) {
            match self.state {
                NeedPrevEnd => self.set_state(NeedQso, rng, env),
                NeedQso => self.dec_patience(),
                NeedNr | NeedCall | NeedCallNr => self.state = Failed,
                NeedEnd => self.state = Done,
                _ => {}
            }
            return;
        }
        if m.has(Msg::Nil) {
            match self.state {
                NeedPrevEnd => self.set_state(NeedQso, rng, env),
                NeedQso => self.dec_patience(),
                NeedNr | NeedCall | NeedCallNr | NeedEnd => self.state = Failed,
                _ => {}
            }
            return;
        }

        if m.has(Msg::HisCall) {
            match (self.is_my_call(sent_call, rng, env), self.state) {
                (CallCheck::Yes, NeedPrevEnd | NeedQso | NeedCallNr) => self.set_state(NeedNr, rng, env),
                (CallCheck::Yes, NeedCall) => self.set_state(NeedEnd, rng, env),
                (CallCheck::Almost, NeedPrevEnd | NeedQso | NeedNr) => self.set_state(NeedCallNr, rng, env),
                (CallCheck::Almost, NeedEnd) => self.set_state(NeedCall, rng, env),
                (CallCheck::No, NeedQso) => self.state = NeedPrevEnd,
                (CallCheck::No, NeedNr | NeedCall | NeedCallNr) => self.state = Failed,
                (CallCheck::No, NeedEnd) => self.state = Done,
                _ => {}
            }
        }

        if m.has(Msg::B4) {
            match self.state {
                NeedPrevEnd | NeedQso => self.set_state(NeedQso, rng, env),
                NeedNr | NeedEnd => self.state = Failed,
                _ => {}
            }
        }

        if m.has(Msg::Nr) {
            match self.state {
                NeedQso => self.state = NeedPrevEnd,
                NeedNr if rng.chance(0.9) || env.hst() => self.set_state(NeedEnd, rng, env),
                NeedCallNr if rng.chance(0.9) || env.hst() => self.set_state(NeedCall, rng, env),
                _ => {}
            }
        }

        if m.has(Msg::Tu) {
            match self.state {
                NeedPrevEnd => self.set_state(NeedQso, rng, env),
                NeedEnd => self.state = Done,
                _ => {}
            }
        }

        if !env.lids && m == MsgSet::of(Msg::Garbage) {
            self.state = NeedPrevEnd;
        }

        if self.state != NeedPrevEnd {
            self.dec_patience();
        }
    }

    pub fn reply(&self, rng: &mut Rng, env: &Env) -> Msg {
        use OpState::*;
        match self.state {
            NeedPrevEnd | Done | Failed => Msg::None,
            NeedQso => Msg::MyCall,
            NeedNr => {
                if self.patience == FULL_PATIENCE - 1 || rng.chance(0.3) {
                    Msg::NrQm
                } else {
                    Msg::Agn
                }
            }
            NeedCall => {
                if env.hst() || rng.f64() > 0.5 {
                    Msg::DeMyCallNr1
                } else if rng.f64() > 0.25 {
                    Msg::DeMyCallNr2
                } else {
                    Msg::MyCallNr2
                }
            }
            NeedCallNr => {
                if env.hst() || rng.f64() > 0.5 {
                    Msg::DeMyCall1
                } else {
                    Msg::DeMyCall2
                }
            }
            NeedEnd => {
                if self.patience < FULL_PATIENCE - 1 {
                    Msg::Nr
                } else if env.hst() || rng.chance(0.9) {
                    Msg::RNr
                } else {
                    Msg::RNr2
                }
            }
        }
    }
}

/// A station that answers the CQ.
pub struct Dx {
    pub op: Operator,
    pub qsb: Qsb,
}

impl Dx {
    pub fn new(call: String, own_call: &str, lids: bool, flutter: bool, rng: &mut Rng, env: &Env) -> (Station, Dx) {
        let op = Operator::new(call.clone(), rng, env);
        let mut st = Station::new();
        st.his_call = own_call.to_string();
        st.my_call = call;
        st.nr_with_error = lids && rng.chance(0.1);
        st.wpm = Operator::pick_wpm(rng, env);
        st.nr = op.pick_nr(rng, env);
        st.rst = if lids && rng.chance(0.03) { 559 + 10 * rng.below(4) as u32 } else { 599 };
        let mut bw = 0.1 + rng.f64() / 2.0;
        if flutter && rng.chance(0.3) {
            bw = 3.0 + rng.f64() * 30.0;
        }
        let qsb = Qsb::new(bw, rng);
        st.amplitude = (9000.0 + 18000.0 * (1.0 + rng.u_shaped())) as f32;
        st.set_pitch(rng.gauss_lim(0.0, 300.0).round() as i32);
        // the event that I finished sending follows at once
        st.timeout = NEVER;
        st.state = State::Copying;
        (st, Dx { op, qsb })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use OpState::*;

    fn env(mode: RunMode, lids: bool) -> Env {
        Env { mode, lids, wpm: 30, minute: 10.0, slow: 50, fast: 100 }
    }

    fn op(state: OpState) -> Operator {
        Operator { call: "W1ABC".into(), skills: 2, patience: FULL_PATIENCE, repeat_cnt: 1, state }
    }

    /// Runs one message without any luck involved: LIDs off, HST for the sure things.
    fn after(state: OpState, m: Msg, sent: &str) -> OpState {
        let mut o = op(state);
        let mut rng = Rng::new(1);
        o.msg_received(MsgSet::of(m), sent, &mut rng, &env(RunMode::Hst, false));
        o.state
    }

    #[test]
    fn penalties() {
        for (sent, want) in [
            ("W1ABC", 0),
            ("W1AB", 0),
            ("1ABC", 0),
            ("W1?", 0),
            ("?ABC", 0),
            ("W?C", 0),
            ("W1ABD", 2),
            ("K1ABC", 2),
            ("W1XYZ", 6),
        ] {
            assert_eq!(call_penalty(sent.as_bytes(), b"W1ABC"), want, "{sent}");
        }
    }

    #[test]
    fn is_my_call_classes() {
        let o = op(NeedQso);
        let mut rng = Rng::new(1);
        let e = env(RunMode::Pileup, false);
        for (sent, want) in [
            ("W1ABC", CallCheck::Yes),
            ("W1AB", CallCheck::Almost),
            ("W1?", CallCheck::Almost),
            ("W1ABD", CallCheck::Almost),
            ("W1XYZ", CallCheck::No),
            ("W", CallCheck::No),
            ("?", CallCheck::No),
            ("", CallCheck::No),
            ("XY", CallCheck::No),
        ] {
            assert_eq!(o.is_my_call(sent, &mut rng, &e), want, "{sent}");
        }
    }

    #[test]
    fn lids_sometimes_misjudge() {
        let o = op(NeedQso);
        let mut rng = Rng::new(1);
        let e = env(RunMode::Pileup, true);
        let n = 20000;
        let doubted = (0..n).filter(|_| o.is_my_call("W1ABC", &mut rng, &e) != CallCheck::Yes).count();
        let taken = (0..n).filter(|_| o.is_my_call("W1ABD", &mut rng, &e) == CallCheck::Yes).count();
        assert!((100..300).contains(&doubted), "{doubted}");
        assert!((600..1000).contains(&taken), "{taken}");
    }

    #[test]
    fn state_table_his_call() {
        for (from, sent, want) in [
            (NeedQso, "W1ABC", NeedNr),
            (NeedPrevEnd, "W1ABC", NeedNr),
            (NeedCallNr, "W1ABC", NeedNr),
            (NeedCall, "W1ABC", NeedEnd),
            (NeedEnd, "W1ABC", NeedEnd),
            (NeedQso, "W1AB", NeedCallNr),
            (NeedNr, "W1AB", NeedCallNr),
            (NeedEnd, "W1AB", NeedCall),
            (NeedQso, "W9XYZ", NeedPrevEnd),
            (NeedNr, "W9XYZ", Failed),
            (NeedCall, "W9XYZ", Failed),
            (NeedEnd, "W9XYZ", Done),
        ] {
            assert_eq!(after(from, Msg::HisCall, sent), want, "{from:?} {sent}");
        }
    }

    #[test]
    fn state_table_cq_nil_b4() {
        for (from, want) in [(NeedPrevEnd, NeedQso), (NeedNr, Failed), (NeedCall, Failed), (NeedCallNr, Failed), (NeedEnd, Done)] {
            assert_eq!(after(from, Msg::Cq, ""), want, "{from:?}");
        }
        for (from, want) in [(NeedPrevEnd, NeedQso), (NeedNr, Failed), (NeedEnd, Failed), (NeedCall, Failed)] {
            assert_eq!(after(from, Msg::Nil, ""), want, "{from:?}");
        }
        for (from, want) in [(NeedNr, Failed), (NeedEnd, Failed), (NeedCall, NeedCall), (NeedCallNr, NeedCallNr)] {
            assert_eq!(after(from, Msg::B4, ""), want, "{from:?}");
        }
    }

    #[test]
    fn state_table_nr_tu() {
        for (from, want) in [(NeedQso, NeedPrevEnd), (NeedNr, NeedEnd), (NeedCallNr, NeedCall), (NeedPrevEnd, NeedPrevEnd)] {
            assert_eq!(after(from, Msg::Nr, ""), want, "{from:?}");
        }
        assert_eq!(after(NeedEnd, Msg::Tu, ""), Done);
        assert_eq!(after(NeedPrevEnd, Msg::Tu, ""), NeedQso);
    }

    #[test]
    fn garbage_without_lids_puts_him_on_standby() {
        assert_eq!(after(NeedNr, Msg::Garbage, ""), NeedPrevEnd);
    }

    #[test]
    fn patience_runs_out() {
        let mut o = op(NeedNr);
        let mut rng = Rng::new(1);
        let e = env(RunMode::Pileup, true);
        for _ in 0..4 {
            o.msg_received(MsgSet::of(Msg::None), "", &mut rng, &e);
            assert_eq!(o.state, NeedNr);
        }
        o.msg_received(MsgSet::of(Msg::None), "", &mut rng, &e);
        assert_eq!(o.state, Failed);
    }

    #[test]
    fn replies_fit_the_state() {
        let mut rng = Rng::new(1);
        let e = env(RunMode::Hst, false);
        assert_eq!(op(NeedQso).reply(&mut rng, &e), Msg::MyCall);
        assert_eq!(op(NeedPrevEnd).reply(&mut rng, &e), Msg::None);
        assert_eq!(op(NeedCall).reply(&mut rng, &e), Msg::DeMyCallNr1);
        assert_eq!(op(NeedCallNr).reply(&mut rng, &e), Msg::DeMyCall1);
        assert_eq!(op(NeedEnd).reply(&mut rng, &e), Msg::RNr);
        let patient = Operator { patience: 2, ..op(NeedEnd) };
        assert_eq!(patient.reply(&mut rng, &e), Msg::Nr);
        let first = Operator { patience: FULL_PATIENCE - 1, ..op(NeedNr) };
        assert_eq!(first.reply(&mut rng, &e), Msg::NrQm);
    }

    #[test]
    fn speed_stays_within_its_range() {
        let mut rng = Rng::new(1);
        let e = env(RunMode::Pileup, false);
        for _ in 0..1000 {
            assert!((15..=30).contains(&Operator::pick_wpm(&mut rng, &e)));
        }
        // plus or minus 10 %: at 30 wpm from 27 to 33, both ends reached
        let e = Env { slow: 90, fast: 110, ..env(RunMode::Pileup, false) };
        let seen: Vec<u32> = (0..2000).map(|_| Operator::pick_wpm(&mut rng, &e)).collect();
        assert!(seen.iter().all(|w| (27..=33).contains(w)), "{seen:?}");
        assert!(seen.contains(&27) && seen.contains(&33));
    }
}
