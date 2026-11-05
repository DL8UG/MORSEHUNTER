//! The band: noise, stations, my own transmitter and the log, one audio block at a time.
//! Everything runs from `block`, so the timing of a contest is the sample count and nothing else.

use std::collections::{HashSet, VecDeque};
use std::f32::consts::TAU;

use crate::callgen;
use crate::code::RunCode;
use crate::dsp::{Agc, Filter, Modulator};
use crate::dx::{Dx, Env, OpState};
use crate::entry::Entry;
use crate::log::{Log, Qso, QsoErr, Stats, extract_prefix};
use crate::msg::{Msg, MsgSet};
use crate::paddle::Paddle;
use crate::rng::Rng;
use crate::sent::{SentLog, Sender};
use crate::settings::{CALLER_FAST_DEFAULT, CALLER_SLOW_DEFAULT, Settings};
use crate::skill::{self, Skill};
use crate::spectrum::Spectrum;
use crate::station::{Event, NEVER, State, Station};
use crate::{BLOCK, RATE, blocks_to_seconds, seconds_to_blocks};
use crate::analysis;
use crate::waves::Waves;

const NOISE_AMP: f64 = 6000.0;
/// The original rolls these per block of 512 samples.
const BLOCK_SCALE: f64 = BLOCK as f64 / 512.0;
const QRN_BURST: f64 = 0.01 * BLOCK_SCALE;
const QRM_START: f64 = 0.0002 * BLOCK_SCALE;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunMode {
    Stop,
    Pileup,
    Single,
    Wpx,
    Hst,
}

impl RunMode {
    pub fn competition(self) -> bool {
        matches!(self, RunMode::Wpx | RunMode::Hst)
    }
}

pub enum Kind {
    Dx(Box<Dx>),
    Qrm { patience: i32 },
    Qrn,
}

pub struct Stn {
    pub st: Station,
    pub kind: Kind,
    /// Everything this station leaves to chance. It is its own stream, so one operator's
    /// timing can't change what the next station does.
    rng: Rng,
    dead: bool,
}

impl Stn {
    pub fn dx(&self) -> Option<&Dx> {
        if self.dead {
            return None;
        }
        match &self.kind {
            Kind::Dx(d) => Some(d),
            _ => None,
        }
    }
}

/// My own transmitter. A message is cut into pieces around the other call, so that the
/// call can still be changed while it goes out.
pub struct Me {
    pub st: Station,
    pieces: VecDeque<String>,
    /// The pieces of this message that are through, for the record of what was sent.
    said: String,
    rng: Rng,
    /// Keyed by hand: no envelope, the over ends when the key has been quiet.
    pub paddle_over: bool,
}

const HIS_CALL: &str = "@";

impl Me {
    fn add_to_pieces(&mut self, text: &str) {
        let mut rest = text;
        while let Some(p) = rest.find("<his>") {
            if p > 0 {
                self.pieces.push_back(rest[..p].to_string());
            }
            self.pieces.push_back(HIS_CALL.to_string());
            rest = &rest[p + 5..];
        }
        if !rest.is_empty() {
            self.pieces.push_back(rest.to_string());
        }
    }
}

/// The end of a run with a score to show.
#[derive(Clone, Debug)]
pub struct Finished {
    pub mode: RunMode,
    pub raw: Stats,
    pub verified: Stats,
    pub hst_score: u64,
    pub new_record: bool,
    pub code: Option<String>,
    /// What was sent was shown during the run: no points, no record, no score table.
    pub unscored: bool,
    /// Skill points, for a run that ended by itself and was scored.
    pub skill: Option<Skill>,
    /// The run came from a code.
    pub fixed: bool,
}

/// One independent random stream of a run.
fn stream(seed: u32, kind: u64, n: u64) -> Rng {
    let mut r = Rng::new((kind << 56) ^ ((seed as u64) << 24) ^ n);
    for _ in 0..4 {
        r.next_u64();
    }
    r
}

const STREAM_BAND: u64 = 1;
const STREAM_CQ: u64 = 2;
const STREAM_ME: u64 = 3;
const STREAM_CALLER: u64 = 4;
const STREAM_WAVES: u64 = 5;

pub struct Contest {
    pub settings: Settings,
    pub mode: RunMode,
    pub rit: i32,
    pub me: Me,
    pub stations: Vec<Stn>,
    pub log: Log,
    pub entry: Entry,
    pub paddle: Paddle,
    /// What went out on the air in this run.
    pub sent: SentLog,
    /// Gives each new run its seed.
    seeds: Rng,
    seed: u32,
    /// Noise, QRN and QRM: runs on the clock, whatever the operator does.
    pub band: Rng,
    /// How many stations answer each CQ.
    cq: Rng,
    /// The tide of callers, when it is on.
    waves: Option<Waves>,
    /// Callers so far: caller number n is the same station in every run with this seed.
    caller_no: u64,
    /// The run as a code. Gone when a condition changes during the run.
    pub code: Option<RunCode>,
    /// Started from a code: its conditions can't be changed.
    pub fixed: bool,
    /// What is sent was shown at some point of this run, so it scores nothing.
    pub peeked: bool,
    /// Questions asked since the last contact went into the log or the last CQ.
    asks: u32,
    block_no: u64,
    rit_phase: f32,
    stop_pressed: bool,
    filter: Filter,
    modul: Modulator,
    agc: Agc,
    /// Calls already used, HST never repeats one.
    used: HashSet<String>,
    /// Settings from before a competition or a coded run.
    before_run: Option<Settings>,
    /// Set when an incomplete log line was refused.
    pub beep: bool,
    pub finished: Option<Finished>,
    /// The band before the filter, for the waterfall.
    pub spectrum: Spectrum,
}

impl Contest {
    pub fn new(settings: Settings, seed: u64) -> Self {
        let filter = Filter::new(settings.bandwidth);
        let modul = Modulator::new(settings.pitch);
        let mut st = Station::new();
        st.rst = 599;
        st.nr = 1;
        st.amplitude = 300000.0;
        st.my_call = settings.call.clone();
        st.wpm = settings.wpm;
        let paddle = Paddle::new(&settings);
        Contest {
            settings,
            mode: RunMode::Stop,
            rit: 0,
            me: Me { st, pieces: VecDeque::new(), said: String::new(), rng: stream(0, STREAM_ME, 0), paddle_over: false },
            stations: Vec::new(),
            log: Log::default(),
            entry: Entry::default(),
            paddle,
            sent: SentLog::default(),
            seeds: Rng::new(seed),
            seed: 0,
            band: stream(0, STREAM_BAND, 0),
            cq: stream(0, STREAM_CQ, 0),
            waves: None,
            caller_no: 0,
            code: None,
            fixed: false,
            peeked: false,
            asks: 0,
            block_no: 0,
            rit_phase: 0.0,
            stop_pressed: false,
            filter,
            modul,
            agc: Agc::new(),
            spectrum: Spectrum::default(),
            used: HashSet::new(),
            before_run: None,
            beep: false,
            finished: None,
        }
    }

    pub fn seconds(&self) -> f64 {
        blocks_to_seconds(self.block_no)
    }

    fn env(&self) -> Env {
        Env {
            mode: self.mode,
            lids: self.settings.lids,
            wpm: self.settings.wpm,
            minute: self.seconds() / 60.0 + self.settings.head_start as f64 * 60.0,
            slow: self.settings.caller_slow,
            fast: self.settings.caller_fast,
        }
    }

    pub fn rate(&self) -> u32 {
        self.log.rate(self.seconds())
    }

    /// Stations still in a QSO or trying to get one.
    pub fn dx_count(&self) -> usize {
        self.stations.iter().filter(|s| s.dx().is_some_and(|d| d.op.state != OpState::Done)).count()
    }

    pub fn filter_changed(&mut self) {
        self.filter = Filter::new(self.settings.bandwidth);
    }

    /// New settings from the window, in force at once.
    pub fn apply(&mut self, mut s: Settings) {
        if self.fixed
            && let Some(c) = &self.code
        {
            c.apply(&mut s);
        }
        let bw = s.bandwidth != self.settings.bandwidth;
        let pitch = s.pitch != self.settings.pitch;
        self.settings = s;
        if self.mode != RunMode::Stop && self.settings.debug {
            self.peeked = true;
        }
        if bw {
            self.filter_changed();
        }
        if pitch {
            self.modul.set_pitch(self.settings.pitch);
        }
        self.me.st.wpm = self.settings.wpm;
        if self.mode == RunMode::Stop {
            self.me.st.my_call = self.settings.call.clone();
        }
        self.paddle_speed_changed();
        self.check_code();
    }

    /// A condition changed during the run: the code no longer describes it.
    pub(crate) fn check_code(&mut self) {
        if self.mode == RunMode::Stop {
            return;
        }
        if let Some(c) = &self.code
            && RunCode::from_settings(c.mode, &self.settings, c.seed) != *c
        {
            self.code = None;
        }
    }

    pub fn request_stop(&mut self) {
        self.stop_pressed = true;
    }

    /// Starts a mode with a new seed or, with `Stop`, ends the run.
    pub fn run(&mut self, mode: RunMode) {
        if mode == self.mode {
            return;
        }
        if mode == RunMode::Stop {
            self.mode = mode;
            if let Some(b) = self.before_run.take() {
                restore(&mut self.settings, &b, self.fixed);
                if self.fixed {
                    self.modul.set_pitch(self.settings.pitch);
                    self.me.st.wpm = self.settings.wpm;
                }
                self.fixed = false;
                self.filter_changed();
                self.paddle_speed_changed();
            }
            self.silence_me();
            self.sent.quiet();
            return;
        }
        if mode.competition() {
            self.before_run = Some(self.settings.clone());
            let on = mode == RunMode::Wpx;
            let s = &mut self.settings;
            (s.qrn, s.qrm, s.qsb, s.flutter, s.lids) = (on, on, on, on, on);
            s.duration = s.competition_duration;
            // everybody starts with you, as in the original: scores stay comparable
            s.head_start = 0;
            (s.caller_slow, s.caller_fast) = (CALLER_SLOW_DEFAULT, CALLER_FAST_DEFAULT);
            s.waves = false;
        }
        if mode == RunMode::Hst {
            self.settings.activity = 4;
            self.settings.bandwidth = 600;
        }
        let seed = self.seeds.next_u64() as u32;
        let code = RunCode::from_settings(mode, &self.settings, seed);
        code.apply(&mut self.settings);
        self.begin(code);
    }

    /// The settings to keep when the program ends now: during a competition or a run from
    /// a code the operator's own values, not the ones the run forced.
    pub fn settings_to_keep(&self) -> Settings {
        let mut s = self.settings.clone();
        if let Some(b) = &self.before_run {
            restore(&mut s, b, self.fixed);
        }
        s
    }

    /// Starts exactly the run a code describes. Its conditions hold until the end.
    pub fn run_code(&mut self, code: &RunCode) {
        if self.mode != RunMode::Stop {
            return;
        }
        self.before_run = Some(self.settings.clone());
        code.apply(&mut self.settings);
        self.fixed = true;
        self.begin(code.clone());
    }

    fn begin(&mut self, code: RunCode) {
        self.mode = code.mode;
        self.seed = code.seed;
        self.band = stream(code.seed, STREAM_BAND, 0);
        self.cq = stream(code.seed, STREAM_CQ, 0);
        self.waves = self.settings.waves.then(|| Waves::new(&mut stream(code.seed, STREAM_WAVES, 0), self.settings.duration as f64));
        self.me.rng = stream(code.seed, STREAM_ME, 0);
        self.caller_no = 0;
        self.code = Some(code);
        self.filter_changed();
        self.modul.set_pitch(self.settings.pitch);
        self.paddle_speed_changed();
        self.silence_me();
        self.block_no = 0;
        self.rit = 0;
        self.spectrum.reset();
        self.me.st.nr = 1;
        self.me.st.my_call = self.settings.call.clone();
        self.me.st.wpm = self.settings.wpm;
        self.log = Log::default();
        self.sent.clear();
        self.stations.clear();
        self.used.clear();
        self.entry.wipe();
        self.finished = None;
        self.peeked = self.settings.debug;
        self.asks = 0;
        self.paddle.reset();
    }

    fn silence_me(&mut self) {
        let m = &mut self.me;
        m.st.envelope.clear();
        m.st.msg_text.clear();
        m.pieces.clear();
        m.said.clear();
        m.paddle_over = false;
        m.st.state = State::Listening;
        self.sent.close(&mut m.st.sent_line, true);
    }

    fn add_caller(&mut self) {
        let mut rng = stream(self.seed, STREAM_CALLER, self.caller_no);
        self.caller_no += 1;
        let mix = self.settings.region_mix;
        let call = loop {
            let c = callgen::pick_call(&mix, &mut rng);
            if self.mode != RunMode::Hst || self.used.insert(c.clone()) {
                break c;
            }
        };
        let env = self.env();
        let s = &self.settings;
        let (st, dx) = Dx::new(call, &s.call, s.lids, s.flutter, &mut rng, &env);
        self.stations.push(Stn { st, kind: Kind::Dx(Box::new(dx)), rng, dead: false });
    }

    fn add_qrn(&mut self) {
        let mut st = Station::new();
        let dur = seconds_to_blocks(self.band.f64()).max(1) as usize * BLOCK;
        let amp = 1e5 * 10f64.powf(2.0 * self.band.f64());
        st.envelope = (0..dur).map(|_| if self.band.chance(0.01) { ((self.band.f64() - 0.5) * amp) as f32 } else { 0.0 }).collect();
        st.state = State::Sending;
        let rng = Rng::new(self.band.next_u64());
        self.stations.push(Stn { st, kind: Kind::Qrn, rng, dead: false });
    }

    fn add_qrm(&mut self) {
        let mut st = Station::new();
        let mut rng = Rng::new(self.band.next_u64());
        let patience = 1 + rng.below(5) as i32;
        st.my_call = callgen::pick_call(&self.settings.region_mix, &mut rng);
        st.his_call = self.settings.call.clone();
        st.amplitude = (5000.0 + 25000.0 * rng.f64()) as f32;
        st.set_pitch(rng.gauss_lim(0.0, 300.0).round() as i32);
        st.wpm = 30 + rng.below(20) as u32;
        let m = match rng.below(7) {
            0 => Msg::Qrl,
            1 | 2 => Msg::Qrl2,
            3..=5 => Msg::LongCq,
            _ => Msg::Qsy,
        };
        st.send_msg(m, &mut rng, false);
        self.stations.push(Stn { st, kind: Kind::Qrm { patience }, rng, dead: false });
    }

    fn event(&mut self, i: usize, ev: Event) {
        let env = self.env();
        let hst = self.mode == RunMode::Hst;
        let me_sending = self.me.st.state == State::Sending;
        let me_msg = self.me.st.msg;
        let s = &mut self.stations[i];
        if s.dead {
            return;
        }
        let rng = &mut s.rng;
        match &mut s.kind {
            Kind::Qrn => {
                if ev == Event::MsgSent {
                    s.dead = true;
                }
            }
            Kind::Qrm { patience } => match ev {
                Event::MsgSent => {
                    *patience -= 1;
                    if *patience == 0 {
                        s.dead = true;
                    } else {
                        s.st.timeout = rng.gauss_lim(seconds_to_blocks(4.0) as f64, 2.0 / BLOCK_SCALE).round() as i32;
                    }
                }
                Event::Timeout => s.st.send_msg(Msg::LongCq, rng, hst),
                _ => {}
            },
            Kind::Dx(dx) => {
                let op = &mut dx.op;
                if op.state == OpState::Done {
                    return;
                }
                let st = &mut s.st;
                match ev {
                    Event::MsgSent => {
                        st.timeout = if me_sending { NEVER } else { op.reply_timeout(rng, &env) };
                    }
                    Event::Timeout => {
                        if st.state == State::Listening {
                            op.msg_received(MsgSet::of(Msg::None), &self.me.st.his_call, rng, &env);
                            if op.state == OpState::Failed {
                                s.dead = true;
                                return;
                            }
                            st.state = State::PreparingToSend;
                        }
                        if st.state == State::PreparingToSend {
                            for _ in 0..op.repeat_cnt {
                                let r = op.reply(rng, &env);
                                st.send_msg(r, rng, hst);
                            }
                        }
                    }
                    Event::MeFinished => {
                        if st.state != State::Sending {
                            let his = &self.me.st.his_call;
                            match st.state {
                                State::Copying => op.msg_received(me_msg, his, rng, &env),
                                State::Listening | State::PreparingToSend => {
                                    if me_msg.has(Msg::Cq) || me_msg.has(Msg::Tu) || me_msg.has(Msg::Nil) {
                                        op.msg_received(me_msg, his, rng, &env);
                                    } else {
                                        op.msg_received(MsgSet::of(Msg::Garbage), his, rng, &env);
                                    }
                                }
                                State::Sending => {}
                            }
                            if op.state == OpState::Failed {
                                s.dead = true;
                                return;
                            }
                            st.timeout = op.send_delay(rng, &env);
                            st.state = State::PreparingToSend;
                        }
                    }
                    Event::MeStarted => {
                        if st.state != State::Sending {
                            st.state = State::Copying;
                        }
                        st.timeout = NEVER;
                    }
                }
            }
        }
    }

    pub(crate) fn on_me_started(&mut self) {
        for i in (0..self.stations.len()).rev() {
            self.event(i, Event::MeStarted);
        }
    }

    pub(crate) fn on_me_finished(&mut self) {
        let m = self.me.st.msg;
        if m.has(Msg::Cq) || m.has(Msg::LongCq) {
            // a new start, what was asked before belongs to no contact
            self.asks = 0;
        }
        if m.has(Msg::Qm) || m.has(Msg::NrQm) || m.has(Msg::Agn) {
            self.asks += 1;
        }
        if !matches!(self.mode, RunMode::Single | RunMode::Hst) {
            let m = self.me.st.msg;
            if m.has(Msg::Cq) || (!self.log.qsos.is_empty() && m.has(Msg::Tu) && m.has(Msg::MyCall)) {
                let n = self.cq.poisson(self.settings.activity as f64 / 2.0 * self.tide());
                for _ in 0..n {
                    self.add_caller();
                }
            }
        }
        for i in (0..self.stations.len()).rev() {
            self.event(i, Event::MeFinished);
        }
    }

    fn send_next_piece(&mut self) {
        let hst = self.mode == RunMode::Hst;
        let m = &mut self.me;
        m.st.msg_text.clear();
        let Some(p) = m.pieces.front().cloned() else { return };
        if p != HIS_CALL {
            m.st.send_text(&p, &mut m.rng, hst);
        } else if self.settings.calls_from_keyer && !self.mode.competition() {
            m.st.send_text(" ", &mut m.rng, hst);
        } else {
            let call = m.st.his_call.clone();
            m.st.send_text(&call, &mut m.rng, hst);
        }
    }

    /// Queues a message on my transmitter, it starts at once if nothing is going out.
    pub fn me_send(&mut self, m: Msg) {
        if self.mode == RunMode::Stop {
            return;
        }
        if self.me.paddle_over {
            self.finish_over();
        }
        let me = &mut self.me;
        if me.st.envelope.is_empty() {
            me.st.msg = MsgSet::empty();
        }
        if m == Msg::None {
            me.st.state = State::Listening;
            return;
        }
        me.st.msg.add(m);
        me.add_to_pieces(m.template());
        if me.st.state != State::Sending {
            self.send_next_piece();
            self.on_me_started();
        }
    }

    pub fn abort_send(&mut self) {
        if self.mode == RunMode::Stop {
            return;
        }
        self.silence_me();
        self.me.st.msg = MsgSet::of(Msg::Garbage);
        self.on_me_finished();
    }

    /// Takes a changed call into the message in progress where that is still possible.
    pub fn update_call_in_message(&mut self, call: &str) -> bool {
        if call.is_empty() {
            return false;
        }
        let m = &mut self.me;
        let mut ok = m.pieces.front().is_some_and(|p| p == HIS_CALL);
        if ok {
            let new = m.st.make_envelope(call);
            ok = new.len() >= m.st.send_pos && new[..m.st.send_pos] == m.st.envelope[..m.st.send_pos.min(m.st.envelope.len())];
            if ok {
                m.st.envelope = new;
                m.st.his_call = call.to_string();
                m.st.msg_text = call.to_string();
            }
        }
        // it may still be waiting further down the message
        if !ok && m.pieces.iter().skip(1).any(|p| p == HIS_CALL) {
            ok = true;
            m.st.his_call = call.to_string();
        }
        ok
    }

    /// Takes the fields into the log. Without a complete line it only beeps.
    pub fn save_qso(&mut self) {
        if self.mode == RunMode::Stop {
            return;
        }
        if !self.entry.complete() {
            self.beep = true;
            return;
        }
        let call = self.entry.call.text.replace('?', "");
        let mut q = Qso {
            t: self.seconds(),
            pfx: extract_prefix(&call),
            call,
            true_call: String::new(),
            rst: self.entry.rst.text.parse().unwrap_or(0),
            true_rst: 0,
            nr: self.entry.nr.text.parse().unwrap_or(0),
            true_nr: 0,
            sent_rst: self.me.st.rst,
            sent_nr: self.me.st.nr,
            dupe: false,
            err: QsoErr::Ok,
            wpm: self.settings.wpm,
            asks: self.asks,
            conditions: skill::conditions(&self.settings),
            meant: None,
        };
        q.dupe = self.log.qsos.iter().any(|o| o.call == q.call && o.err == QsoErr::Ok);
        // what does the other log say?
        if let Some(s) = self
            .stations
            .iter_mut()
            .rev()
            .find(|s| s.dx().is_some_and(|d| d.op.state == OpState::Done) && s.st.my_call == q.call)
        {
            q.true_call = s.st.my_call.clone();
            q.true_rst = s.st.rst;
            q.true_nr = s.st.nr;
            s.dead = true;
        }
        q.check();
        if q.true_call.is_empty() {
            q.meant = self.meant_for(&q.call);
        }
        self.log.qsos.push(q);
        self.asks = 0;
        self.entry.wipe();
        self.me.st.nr += 1;
    }

    /// How many callers there are just now against the usual crowd: 1 without the tide.
    pub fn tide(&self) -> f64 {
        self.waves.as_ref().map_or(1.0, |w| w.factor(self.seconds() / 60.0))
    }

    /// The station a call no station had was most likely meant for: the caller with the
    /// closest call. One that heard its call wrong waits for a correction, so any state.
    fn meant_for(&self, call: &str) -> Option<analysis::Meant> {
        self.stations
            .iter()
            .filter(|s| !s.dead && s.dx().is_some())
            .filter(|s| analysis::misheard(call, &s.st.my_call))
            .min_by_key(|s| analysis::distance(call, &s.st.my_call))
            .map(|s| analysis::Meant { call: s.st.my_call.clone(), rst: s.st.rst, nr: s.st.nr })
    }

    /// A station that got everything goes into the log entry made for it.
    fn dx_to_log(&mut self) {
        if let Some(last) = self.log.qsos.last()
            && last.true_call.is_empty()
            && last.meant.is_none()
        {
            let meant = self.meant_for(&last.call.clone());
            if let Some(last) = self.log.qsos.last_mut() {
                last.meant = meant;
            }
        }
        let Some(last) = self.log.qsos.last_mut() else { return };
        for s in self.stations.iter_mut().rev() {
            if s.dx().is_some_and(|d| d.op.state == OpState::Done) && s.st.my_call == last.call {
                last.true_call = s.st.my_call.clone();
                last.true_rst = s.st.rst;
                last.true_nr = s.st.nr;
                last.check();
                s.dead = true;
            }
        }
    }

    fn finish_run(&mut self) {
        let pressed = self.stop_pressed;
        self.stop_pressed = false;
        let mode = self.mode;
        // every run that ends by itself has a result, a stopped one only in HST
        let report = mode == RunMode::Hst || !pressed;
        if report {
            let verified = self.log.verified();
            let unscored = self.peeked;
            let new_record = mode == RunMode::Wpx && !self.fixed && !unscored && verified.score > self.settings.hi_score;
            if new_record {
                self.settings.hi_score = verified.score;
            }
            let code = self.code.as_ref().map(RunCode::encode);
            let skill = (!pressed && !unscored).then(|| skill::skill(&self.log.qsos, self.seconds()));
            self.finished = Some(Finished { mode, raw: self.log.raw(), verified, hst_score: self.log.hst().1, new_record, code, unscored, skill, fixed: self.fixed });
        }
        self.run(RunMode::Stop);
    }

    /// One block of receiver audio, -1 to 1, and all timers one block on.
    pub fn block(&mut self, out: &mut [f32; BLOCK]) {
        if self.mode == RunMode::Stop {
            out.fill(0.0);
            return;
        }
        self.block_no += 1;

        let (mut re, mut im) = ([0f32; BLOCK], [0f32; BLOCK]);
        for i in 0..BLOCK {
            re[i] = (3.0 * NOISE_AMP * (self.band.f64() - 0.5)) as f32;
            im[i] = (3.0 * NOISE_AMP * (self.band.f64() - 0.5)) as f32;
        }
        if self.settings.qrn {
            for v in re.iter_mut() {
                if self.band.chance(0.01) {
                    *v = (60.0 * NOISE_AMP * (self.band.f64() - 0.5)) as f32;
                }
            }
            if self.band.chance(QRN_BURST) {
                self.add_qrn();
            }
        }
        if self.settings.qrm && self.band.chance(QRM_START) {
            self.add_qrm();
        }

        let rit_step = TAU * self.rit as f32 / RATE as f32;
        let qsb = self.settings.qsb;
        let mut blk = [0f32; BLOCK];
        for s in &mut self.stations {
            if s.dead || s.st.state != State::Sending {
                continue;
            }
            let n = s.st.get_block(&mut blk);
            if qsb && let Kind::Dx(d) = &mut s.kind {
                d.qsb.apply(&mut blk[..n], &mut s.rng);
            }
            for i in 0..n {
                let bfo = s.st.next_bfo() - self.rit_phase - i as f32 * rit_step;
                re[i] += blk[i] * bfo.cos();
                im[i] -= blk[i] * bfo.sin();
            }
        }
        self.rit_phase = (self.rit_phase + BLOCK as f32 * rit_step) % TAU;

        // the band without me, as a waterfall shows it
        self.spectrum.push(&re, &im, self.rit);

        if self.me.st.state == State::Sending {
            if self.me.paddle_over {
                if !self.settings.qsk {
                    re.fill(0.0);
                    im.fill(0.0);
                }
            } else {
                self.my_audio(&mut re, &mut im, &mut blk);
            }
        }

        self.filter.run(&mut re, &mut im);
        self.modul.run(&re, &im, out);
        self.agc.run(out);
        out.iter_mut().for_each(|v| *v /= 32768.0);

        // a keyed over has no envelope that could run out, the pause in the keying ends it
        if !self.me.paddle_over && self.me.st.tick() == Some(Event::MsgSent) {
            self.on_me_finished();
        }
        for i in (0..self.stations.len()).rev() {
            if !self.stations[i].dead
                && let Some(ev) = self.stations[i].st.tick()
            {
                self.event(i, ev);
            }
        }

        self.dx_to_log();
        self.stations.retain(|s| !s.dead);
        self.track_sent();

        match self.mode {
            RunMode::Single if self.dx_count() == 0 => {
                self.me.st.msg = MsgSet::of(Msg::Cq);
                self.add_caller();
                let last = self.stations.len() - 1;
                self.event(last, Event::MeFinished);
            }
            RunMode::Hst => {
                let missing = (self.settings.activity as usize).saturating_sub(self.dx_count());
                if missing > 0 {
                    self.me.st.msg = MsgSet::of(Msg::Cq);
                    for _ in 0..missing {
                        self.add_caller();
                        let last = self.stations.len() - 1;
                        self.event(last, Event::MeFinished);
                    }
                }
            }
            _ => {}
        }

        if self.seconds() >= self.settings.duration as f64 * 60.0 || self.stop_pressed {
            self.finish_run();
        }
    }

    /// My transmitter as the record of what was sent shows it.
    pub(crate) fn show_me(&mut self, text: &str, hand: bool) {
        let t = self.seconds();
        self.sent.show(&mut self.me.st.sent_line, t, Sender::Me, &self.settings.call, text, hand);
    }

    /// Brings the record up to what is on the air after this block.
    fn track_sent(&mut self) {
        let t = self.seconds();
        self.sent.quiet();
        for s in &mut self.stations {
            let st = &mut s.st;
            if st.state == State::Sending && !st.msg_text.is_empty() {
                let who = if matches!(s.kind, Kind::Qrm { .. }) { Sender::Qrm } else { Sender::Caller };
                self.sent.show(&mut st.sent_line, t, who, &st.my_call, &st.msg_text, false);
            } else {
                self.sent.close(&mut st.sent_line, false);
            }
        }
        if self.me.st.state != State::Sending {
            self.me.said.clear();
            self.sent.close(&mut self.me.st.sent_line, false);
        } else if self.me.paddle_over {
            let text = self.paddle.text().trim().to_string();
            if !text.is_empty() {
                self.show_me(&text, true);
            }
        } else {
            let text = format!("{} {}", self.me.said, self.me.st.msg_text);
            self.show_me(text.trim(), false);
        }
    }

    fn my_audio(&mut self, re: &mut [f32; BLOCK], im: &mut [f32; BLOCK], blk: &mut [f32; BLOCK]) {
        let n = self.me.st.get_block(blk);
        if self.me.st.envelope.is_empty() {
            self.me.pieces.pop_front();
            if !self.me.pieces.is_empty() {
                let m = &mut self.me;
                if !m.said.is_empty() {
                    m.said.push(' ');
                }
                m.said.push_str(m.st.msg_text.trim());
                self.send_next_piece();
            }
            self.entry.advance();
        }
        let smg = 10f32.powf(self.settings.mon_db / 20.0);
        let amp = self.me.st.amplitude;
        let mut rfg = 1f32;
        for i in 0..n {
            if self.settings.qsk {
                let open = 1.0 - blk[i] / amp;
                rfg = if rfg > open { open } else { rfg * 0.997 + 0.003 };
                re[i] = smg * blk[i] + rfg * re[i];
                im[i] = smg * blk[i] + rfg * im[i];
            } else {
                re[i] = smg * blk[i];
                im[i] = smg * blk[i];
            }
        }
    }
}

/// Gives back what a competition or a run from a code took over. A code fixes speed,
/// pitch, QSK and regions as well.
fn restore(s: &mut Settings, b: &Settings, fixed: bool) {
    (s.qrn, s.qrm, s.qsb, s.flutter, s.lids) = (b.qrn, b.qrm, b.qsb, b.flutter, b.lids);
    (s.duration, s.activity, s.bandwidth, s.head_start) = (b.duration, b.activity, b.bandwidth, b.head_start);
    (s.caller_slow, s.caller_fast, s.waves) = (b.caller_slow, b.caller_fast, b.waves);
    if fixed {
        (s.wpm, s.pitch, s.qsk, s.region_mix) = (b.wpm, b.pitch, b.qsk, b.region_mix);
    }
}

