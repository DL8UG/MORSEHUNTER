//! Whole contests, typed by a scripted operator, faster than real time.

use mr_sim::bot::Bot;
use mr_sim::code::RunCode;
use mr_sim::contest::{Contest, RunMode};
use mr_sim::dx::OpState;
use mr_sim::entry::{Field, Key};
use mr_sim::log::QsoErr;
use mr_sim::msg::Msg;
use mr_sim::sent::Sender;
use mr_sim::settings::Settings;
use mr_sim::station::State;
use mr_sim::{BLOCK, blocks_to_seconds, seconds_to_blocks};

fn settings() -> Settings {
    Settings { call: "DL8UG".into(), duration: 240, ..Settings::default() }
}

fn clean() -> Settings {
    Settings { qrn: false, qrm: false, qsb: false, flutter: false, lids: false, ..settings() }
}

fn run_secs(c: &mut Contest, bot: Option<&mut Bot>, secs: f64) {
    let mut b = [0f32; BLOCK];
    let n = seconds_to_blocks(secs);
    match bot {
        Some(bot) => {
            for _ in 0..n {
                c.block(&mut b);
                bot.step(c);
            }
        }
        None => {
            for _ in 0..n {
                c.block(&mut b);
            }
        }
    }
}

fn type_text(c: &mut Contest, t: &str) {
    for ch in t.chars() {
        c.key(Key::Char(ch));
    }
}

/// Runs until no station sends and my transmitter is quiet, a pause for the reply included.
fn settle(c: &mut Contest) {
    let quiet = |c: &Contest| c.me.st.state != State::Sending && c.stations.iter().all(|s| s.st.state != State::Sending);
    let mut b = [0f32; BLOCK];
    let mut still = 0;
    for _ in 0..seconds_to_blocks(60.0) {
        c.block(&mut b);
        still = if quiet(c) { still + 1 } else { 0 };
        if still > seconds_to_blocks(1.0) {
            return;
        }
    }
    panic!("the band never went quiet");
}

/// A caller after a CQ, with a good chance on any seed.
fn get_caller(c: &mut Contest) -> String {
    for _ in 0..30 {
        c.key(Key::F(1));
        settle(c);
        if let Some(s) = c.stations.iter().find(|s| s.dx().is_some_and(|d| d.op.state == OpState::NeedQso)) {
            return s.st.my_call.clone();
        }
    }
    panic!("nobody called in 30 CQs");
}

fn dx_of<'a>(c: &'a Contest, call: &str) -> &'a mr_sim::contest::Stn {
    c.stations.iter().find(|s| s.st.my_call == call).expect("station")
}

#[test]
fn bot_runs_a_clean_pileup() {
    for seed in 0..3 {
        let mut c = Contest::new(clean(), seed);
        c.run(RunMode::Pileup);
        let mut bot = Bot::new(seed);
        run_secs(&mut c, Some(&mut bot), 600.0);
        let raw = c.log.raw();
        let ver = c.log.verified();
        assert!(raw.points >= 30, "seed {seed}: only {} QSOs", raw.points);
        let bad: Vec<_> = c.log.qsos.iter().filter(|q| q.err != QsoErr::Ok).map(|q| (&q.call, q.err)).collect();
        assert!(ver.points * 100 >= raw.points * 90, "seed {seed}: {} of {}: {bad:?}", ver.points, raw.points);
        assert_eq!(c.me.st.nr as u64, raw.points + 1);
    }
}

#[test]
fn every_mode_survives_full_conditions() {
    for seed in 0..2 {
        for mode in [RunMode::Pileup, RunMode::Single, RunMode::Wpx, RunMode::Hst] {
            let mut c = Contest::new(Settings { competition_duration: 4, ..settings() }, seed);
            c.run(mode);
            let mut bot = Bot::new(seed);
            bot.miscopy = 0.05;
            run_secs(&mut c, Some(&mut bot), 260.0);
            assert_eq!(c.mode, if mode.competition() { RunMode::Stop } else { mode }, "{mode:?}");
            assert!(!c.log.qsos.is_empty() || mode.competition(), "{mode:?}");
        }
    }
}

#[test]
fn same_seed_same_audio() {
    let take = || {
        let mut c = Contest::new(settings(), 42);
        c.run(RunMode::Pileup);
        let mut bot = Bot::new(1);
        let mut b = [0f32; BLOCK];
        let mut all = Vec::new();
        for _ in 0..3000 {
            c.block(&mut b);
            bot.step(&mut c);
            all.extend_from_slice(&b);
        }
        all
    };
    assert!(take() == take());
}

#[test]
fn audio_stays_in_range() {
    let mut c = Contest::new(settings(), 7);
    c.run(RunMode::Pileup);
    let mut bot = Bot::new(1);
    let mut b = [0f32; BLOCK];
    let mut rms = 0f64;
    let n = 6000;
    for _ in 0..n {
        c.block(&mut b);
        bot.step(&mut c);
        for v in b {
            assert!(v.is_finite() && v.abs() <= 1.0, "{v}");
            rms += (v as f64).powi(2);
        }
    }
    let rms = (rms / (n * BLOCK) as f64).sqrt();
    assert!((0.03..0.6).contains(&rms), "{rms}");
}

#[test]
fn callers_per_cq_follow_the_activity() {
    let mut c = Contest::new(Settings { activity: 6, ..clean() }, 3);
    c.run(RunMode::Pileup);
    let mut total = 0;
    let n = 300;
    for _ in 0..n {
        c.stations.clear();
        c.key(Key::F(1));
        while c.me.st.state == State::Sending {
            run_secs(&mut c, None, 0.05);
        }
        total += c.stations.len();
    }
    let mean = total as f64 / n as f64;
    assert!((2.5..3.5).contains(&mean), "{mean}");
}

#[test]
fn single_calls_need_no_cq() {
    let mut c = Contest::new(clean(), 1);
    c.run(RunMode::Single);
    run_secs(&mut c, None, 0.1);
    assert_eq!(c.dx_count(), 1);
    run_secs(&mut c, None, 3.0);
    assert!(c.stations.iter().any(|s| s.dx().is_some()));
}

#[test]
fn a_contact_by_hand() {
    let mut c = Contest::new(clean(), 5);
    c.run(RunMode::Pileup);
    let call = get_caller(&mut c);
    type_text(&mut c, &call);
    c.key(Key::Enter); // call and exchange
    assert!(c.entry.call_sent && c.entry.nr_sent);
    settle(&mut c);
    // the cursor waits in the number field, the report is filled in
    assert_eq!(c.entry.focus, Field::Nr);
    assert_eq!(c.entry.rst.text, "599");
    let (nr, rst) = {
        let s = dx_of(&c, &call);
        assert_eq!(s.dx().unwrap().op.state, OpState::NeedEnd);
        (s.st.nr.to_string(), s.st.rst)
    };
    assert_eq!(rst, 599);
    type_text(&mut c, &nr);
    c.key(Key::Enter); // TU and into the log
    assert!(c.entry.call.text.is_empty());
    assert_eq!(c.log.qsos.len(), 1);
    assert_eq!(c.log.qsos[0].err, QsoErr::Nil, "not yet confirmed while the TU goes out");
    settle(&mut c);
    assert_eq!(c.log.qsos[0].err, QsoErr::Ok);
    assert_eq!(c.log.verified().points, 1);
    assert_eq!(c.me.st.nr, 2);
}

#[test]
fn wrong_call_ends_as_nil() {
    let mut c = Contest::new(clean(), 6);
    c.run(RunMode::Pileup);
    get_caller(&mut c);
    type_text(&mut c, "W9XYZ");
    c.key(Key::Enter);
    settle(&mut c);
    c.key(Key::Char('1'));
    c.key(Key::Enter);
    settle(&mut c);
    assert_eq!(c.log.qsos[0].err, QsoErr::Nil);
}

#[test]
fn a_question_counts_for_the_contact_it_was_asked_in() {
    let mut c = Contest::new(clean(), 6);
    c.run(RunMode::Pileup);
    let call = get_caller(&mut c);
    type_text(&mut c, &call);
    c.key(Key::Enter);
    settle(&mut c);
    c.key(Key::F(7));
    settle(&mut c);
    let nr = dx_of(&c, &call).st.nr;
    type_text(&mut c, &nr.to_string());
    c.key(Key::Enter);
    settle(&mut c);
    let q = &c.log.qsos[0];
    assert_eq!((q.err, q.asks, q.wpm), (QsoErr::Ok, 1, c.settings.wpm));
    assert_eq!(q.conditions, 1.0);
    let clean_worth = mr_sim::skill::BASE * q.wpm as f64 / mr_sim::skill::REF_WPM;
    assert!((mr_sim::skill::value(q) - 0.8 * clean_worth).abs() < 1e-9);
    // the next contact starts with a clean slate
    c.key(Key::F(1));
    settle(&mut c);
    let call = get_caller(&mut c);
    type_text(&mut c, &call);
    c.key(Key::Enter);
    settle(&mut c);
    let nr = dx_of(&c, &call).st.nr;
    type_text(&mut c, &nr.to_string());
    c.key(Key::Enter);
    settle(&mut c);
    assert_eq!(c.log.qsos[1].asks, 0);
}

#[test]
fn a_practice_run_that_ends_by_itself_has_skill_points() {
    let mut c = Contest::new(Settings { duration: 1, ..clean() }, 33);
    let mut bot = Bot::new(33);
    c.run(RunMode::Pileup);
    run_secs(&mut c, Some(&mut bot), 61.0);
    let f = c.finished.as_ref().expect("a run that ends by itself reports");
    let skill = f.skill.expect("and has skill points");
    assert!(skill.good > 0 && skill.points > 0);
    assert!(skill.per_hour >= skill.points * 59 && skill.per_hour <= skill.points * 61, "{skill:?}");
    assert!(!f.fixed);
}

#[test]
fn a_stopped_hst_run_has_a_result_but_no_skill_points() {
    let mut c = Contest::new(clean(), 34);
    c.run(RunMode::Hst);
    run_secs(&mut c, None, 1.0);
    c.request_stop();
    run_secs(&mut c, None, 0.1);
    assert!(c.finished.as_ref().unwrap().skill.is_none());
}

#[test]
fn a_partial_call_goes_out_alone_and_the_exchange_follows_the_full_one() {
    let mut c = Contest::new(clean(), 6);
    c.run(RunMode::Pileup);
    let call = get_caller(&mut c);
    let partial = format!("{}?", &call[..2]);
    type_text(&mut c, &partial);
    c.key(Key::Enter);
    settle(&mut c);
    assert_eq!(sent_by(&c, Sender::Me).last().map(String::as_str), Some(partial.as_str()), "no report to a call with a gap");
    assert_eq!(dx_of(&c, &call).dx().unwrap().op.state, OpState::NeedCallNr);
    let repeated = sent_by(&c, Sender::Caller).last().cloned().unwrap();
    assert!(repeated.contains(&call) && !repeated.contains("5NN"), "he gives his call again: {repeated}");
    assert_eq!(c.entry.focus, Field::Call);
    // fill in the gap, the full call and the report go out
    c.key(Key::End);
    c.key(Key::Backspace);
    type_text(&mut c, &call[2..]);
    c.key(Key::Enter);
    settle(&mut c);
    let me = sent_by(&c, Sender::Me);
    assert!(me.last().unwrap().starts_with(&call) && me.last().unwrap().contains("5NN"), "{me:?}");
    // now and then he misses the report and asks NR?, F2 gives it again
    for _ in 0..5 {
        if dx_of(&c, &call).dx().unwrap().op.state != OpState::NeedNr {
            break;
        }
        c.key(Key::F(2));
        settle(&mut c);
    }
    assert_eq!(dx_of(&c, &call).dx().unwrap().op.state, OpState::NeedEnd);
    let nr = dx_of(&c, &call).st.nr;
    type_text(&mut c, &nr.to_string());
    c.key(Key::Enter);
    settle(&mut c);
    assert_eq!(c.log.qsos[0].err, QsoErr::Ok);
    assert_eq!(c.log.qsos[0].asks, 0, "a gap in the call is no question");
}

#[test]
fn wrong_number_and_report_are_marked() {
    for (rst_wrong, want) in [(false, QsoErr::Nr), (true, QsoErr::Rst)] {
        let mut c = Contest::new(clean(), 7);
        c.run(RunMode::Pileup);
        let call = get_caller(&mut c);
        type_text(&mut c, &call);
        c.key(Key::Enter);
        settle(&mut c);
        let nr = dx_of(&c, &call).st.nr + 1;
        if rst_wrong {
            c.key(Key::BackTab);
            c.key(Key::Char('8'));
            c.key(Key::Tab);
            type_text(&mut c, &(nr - 1).to_string());
        } else {
            type_text(&mut c, &nr.to_string());
        }
        c.key(Key::Enter);
        settle(&mut c);
        assert_eq!(c.log.qsos[0].err, want);
    }
}

#[test]
fn second_contact_with_a_call_is_a_dupe() {
    let mut c = Contest::new(clean(), 8);
    c.run(RunMode::Pileup);
    let call = get_caller(&mut c);
    for _ in 0..2 {
        type_text(&mut c, &call);
        c.key(Key::Enter);
        settle(&mut c);
        let nr = c.stations.iter().find(|s| s.st.my_call == call).map_or(1, |s| s.st.nr);
        type_text(&mut c, &nr.to_string());
        c.key(Key::Enter);
        settle(&mut c);
    }
    assert_eq!(c.log.qsos.len(), 2);
    assert_eq!(c.log.qsos[0].err, QsoErr::Ok);
    assert_ne!(c.log.qsos[1].err, QsoErr::Ok);
}

#[test]
fn incomplete_line_is_refused() {
    let mut c = Contest::new(clean(), 9);
    c.run(RunMode::Pileup);
    type_text(&mut c, "W1");
    c.key(Key::SaveEnter);
    assert!(c.beep);
    assert!(c.log.qsos.is_empty());
    assert_eq!(c.entry.call.text, "W1");
}

#[test]
fn escape_stops_the_message_and_clears_the_flags() {
    let mut c = Contest::new(clean(), 10);
    c.run(RunMode::Pileup);
    type_text(&mut c, "W1ABC");
    c.key(Key::F(5));
    assert!(c.entry.call_sent);
    run_secs(&mut c, None, 0.2);
    assert_eq!(c.me.st.state, State::Sending);
    c.key(Key::Esc);
    assert_ne!(c.me.st.state, State::Sending);
    assert!(!c.entry.call_sent);
    assert!(c.me.st.msg.has(Msg::Garbage));
}

#[test]
fn call_can_be_fixed_while_it_goes_out() {
    let mut c = Contest::new(Settings { wpm: 20, ..clean() }, 11);
    c.run(RunMode::Pileup);
    type_text(&mut c, "W1ABC");
    c.key(Key::F(5));
    run_secs(&mut c, None, 0.4);
    // the end has not gone out yet
    c.key(Key::Backspace);
    c.key(Key::Char('D'));
    assert_eq!(c.me.st.his_call, "W1ABD");
    assert!(c.entry.call_sent, "the fix went into the running message");

    // the front has: too late, the call must go out again
    let mut c = Contest::new(Settings { wpm: 20, ..clean() }, 11);
    c.run(RunMode::Pileup);
    type_text(&mut c, "W1ABC");
    c.key(Key::F(5));
    run_secs(&mut c, None, 1.2);
    c.key(Key::Home);
    c.key(Key::Delete);
    c.key(Key::Char('K'));
    assert_eq!(c.me.st.his_call, "W1ABC");
    assert!(!c.entry.call_sent);
}

#[test]
fn enter_with_a_doubtful_call_goes_back_to_it() {
    let mut c = Contest::new(clean(), 12);
    c.run(RunMode::Pileup);
    get_caller(&mut c);
    type_text(&mut c, "W1?C");
    c.key(Key::Enter);
    settle(&mut c);
    assert_eq!(c.entry.focus, Field::Call);
    assert_eq!(c.entry.rst.text, "599");
}

#[test]
fn enter_alone_sends_cq_and_second_enter_asks_again() {
    let mut c = Contest::new(clean(), 13);
    c.run(RunMode::Pileup);
    c.key(Key::Enter);
    assert!(c.me.st.msg.has(Msg::Cq));
    settle(&mut c);
    type_text(&mut c, "W1ABC");
    c.key(Key::Enter);
    settle(&mut c);
    c.key(Key::Enter); // no number copied yet
    assert!(c.me.st.msg.has(Msg::Qm));
}

#[test]
fn shortcuts_work() {
    let mut c = Contest::new(clean(), 14);
    c.run(RunMode::Pileup);
    type_text(&mut c, "W1ABC");
    c.key(Key::Char(';'));
    assert!(c.me.st.msg.has(Msg::HisCall) && c.me.st.msg.has(Msg::Nr));
    settle(&mut c);
    c.key(Key::Char(' '));
    assert_eq!(c.entry.rst.text, "599");
    type_text(&mut c, "123");
    c.key(Key::Char('+'));
    assert_eq!(c.log.qsos.len(), 1);
    assert!(c.me.st.msg.has(Msg::Tu));
    c.key(Key::Char('\\'));
    assert!(c.me.st.msg.has(Msg::Cq));
}

#[test]
fn speed_rit_and_bandwidth_keys() {
    let mut c = Contest::new(Settings { wpm: 32, bandwidth: 300, ..clean() }, 15);
    c.run(RunMode::Pileup);
    c.key(Key::Speed(1));
    assert_eq!(c.settings.wpm, 35);
    c.key(Key::Speed(-1));
    assert_eq!(c.settings.wpm, 30);
    c.key(Key::Speed(-1));
    assert_eq!(c.settings.wpm, 25);
    assert_eq!(c.me.st.wpm, 25);
    for _ in 0..30 {
        c.key(Key::Rit(1));
    }
    assert_eq!(c.rit, 500);
    c.key(Key::Bandwidth(1));
    assert_eq!(c.settings.bandwidth, 350);
    for _ in 0..30 {
        c.key(Key::Bandwidth(-1));
    }
    assert_eq!(c.settings.bandwidth, 100);
}

#[test]
fn competition_sets_conditions_and_gives_them_back() {
    let s = Settings { activity: 7, bandwidth: 300, duration: 11, competition_duration: 1, qsb: false, lids: false, ..settings() };
    for mode in [RunMode::Wpx, RunMode::Hst] {
        let mut c = Contest::new(s.clone(), 16);
        c.run(mode);
        let on = mode == RunMode::Wpx;
        assert_eq!((c.settings.qsb, c.settings.lids, c.settings.qrn), (on, on, on));
        assert_eq!(c.settings.duration, 1);
        if mode == RunMode::Hst {
            assert_eq!((c.settings.activity, c.settings.bandwidth), (4, 600));
        }
        run_secs(&mut c, None, 61.0);
        assert_eq!(c.mode, RunMode::Stop);
        assert_eq!(c.settings, s);
        assert_eq!(c.finished.as_ref().unwrap().mode, mode);
    }
}

#[test]
fn stopping_a_wpx_run_gives_no_score_but_hst_does() {
    for (mode, score) in [(RunMode::Wpx, false), (RunMode::Hst, true), (RunMode::Pileup, false)] {
        let mut c = Contest::new(clean(), 17);
        c.run(mode);
        run_secs(&mut c, None, 1.0);
        c.request_stop();
        run_secs(&mut c, None, 0.1);
        assert_eq!(c.mode, RunMode::Stop);
        assert_eq!(c.finished.is_some(), score, "{mode:?}");
    }
}

/// A worked WPX run of one minute, with what is sent shown never, from the start, or only
/// for a moment in the middle.
fn wpx_minute(debug_from_start: bool, debug_in_middle: bool) -> (mr_sim::contest::Finished, u64) {
    let mut c = Contest::new(Settings { competition_duration: 1, debug: debug_from_start, ..settings() }, 31);
    let mut bot = Bot::new(31);
    c.run(RunMode::Wpx);
    run_secs(&mut c, Some(&mut bot), 20.0);
    if debug_in_middle {
        let mut s = c.settings.clone();
        s.debug = true;
        c.apply(s.clone());
        s.debug = false;
        c.apply(s);
    }
    run_secs(&mut c, Some(&mut bot), 41.0);
    (c.finished.clone().expect("a finished WPX run reports"), c.settings.hi_score)
}

#[test]
fn showing_what_is_sent_costs_the_score() {
    let (f, hi) = wpx_minute(false, false);
    assert!(!f.unscored);
    assert!(f.verified.score > 0, "the bot works some stations");
    assert_eq!(hi, f.verified.score);
    assert!(f.skill.is_some());
    for (start, middle) in [(true, false), (false, true)] {
        let (f, hi) = wpx_minute(start, middle);
        assert!(f.unscored, "debug from start {start}, in the middle {middle}");
        assert!(f.skill.is_none());
        assert!(!f.new_record);
        assert_eq!(hi, 0, "no record from such a run");
    }
}

#[test]
fn debug_while_stopped_does_not_mark_the_next_run() {
    let mut c = Contest::new(settings(), 32);
    let mut s = c.settings.clone();
    s.debug = true;
    c.apply(s.clone());
    s.debug = false;
    c.apply(s);
    c.run(RunMode::Pileup);
    assert!(!c.peeked);
}

#[test]
fn hst_never_repeats_a_call() {
    let mut c = Contest::new(Settings { competition_duration: 10, ..settings() }, 18);
    c.run(RunMode::Hst);
    let mut bot = Bot::new(1);
    let mut seen = std::collections::HashSet::new();
    let mut b = [0f32; BLOCK];
    for _ in 0..seconds_to_blocks(300.0) {
        c.block(&mut b);
        bot.step(&mut c);
        for q in &c.log.qsos {
            seen.insert(q.call.clone());
        }
    }
    assert_eq!(seen.len(), c.log.qsos.len());
    assert!(c.log.qsos.len() > 10, "{}", c.log.qsos.len());
}

#[test]
fn keys_do_nothing_harmful_when_stopped() {
    let mut c = Contest::new(settings(), 19);
    for k in [Key::F(1), Key::Enter, Key::Esc, Key::SaveEnter, Key::Char('.'), Key::Insert] {
        c.key(k);
    }
    let mut b = [1f32; BLOCK];
    c.block(&mut b);
    assert!(b.iter().all(|v| *v == 0.0));
    assert_eq!(blocks_to_seconds(0), 0.0);
    assert!(c.log.qsos.is_empty());
}

/// The calls of the stations in the order they came on.
fn calls_heard(c: &mut Contest, bot: &mut Bot, secs: f64) -> Vec<String> {
    let mut seen = Vec::new();
    let mut b = [0f32; BLOCK];
    for _ in 0..seconds_to_blocks(secs) {
        c.block(&mut b);
        bot.step(c);
        for s in &c.stations {
            if s.dx().is_some() && !seen.contains(&s.st.my_call) {
                seen.push(s.st.my_call.clone());
            }
        }
    }
    seen
}

fn coded(mode: RunMode, seed: u32, base: &Settings) -> RunCode {
    RunCode::from_settings(mode, base, seed)
}

#[test]
fn a_code_gives_every_operator_the_same_callers_in_the_same_order() {
    let base = Settings { activity: 4, ..settings() };
    let code = coded(RunMode::Pileup, 77, &base);
    let mut heard = Vec::new();
    // different programs start, different owners, different ways of working
    for (seed, slow) in [(1, 0.0), (2, 0.02), (3, 0.15)] {
        let mut c = Contest::new(Settings { call: format!("W{seed}XYZ"), ..clean() }, seed);
        c.run_code(&RunCode::decode(&code.encode()).unwrap());
        let mut bot = Bot::new(seed);
        bot.miscopy = slow;
        heard.push(calls_heard(&mut c, &mut bot, 240.0));
    }
    let n = heard.iter().map(Vec::len).min().unwrap();
    assert!(n >= 10, "{n}");
    for h in &heard[1..] {
        assert_eq!(h[..n], heard[0][..n]);
    }
}

#[test]
fn the_band_does_not_care_what_the_operator_does() {
    let code = coded(RunMode::Pileup, 5, &settings());
    let mut after = Vec::new();
    for busy in [false, true] {
        let mut c = Contest::new(settings(), 1);
        c.run_code(&code);
        let mut b = [0f32; BLOCK];
        let mut bot = Bot::new(1);
        for _ in 0..seconds_to_blocks(120.0) {
            c.block(&mut b);
            if busy {
                bot.step(&mut c);
            }
        }
        after.push(c.band.next_u64());
    }
    assert_eq!(after[0], after[1]);
}

#[test]
fn same_code_same_keys_same_samples() {
    let code = coded(RunMode::Wpx, 9, &Settings { duration: 5, competition_duration: 5, ..settings() });
    let take = |own: u64| {
        let mut c = Contest::new(settings(), own);
        c.run_code(&code);
        let mut bot = Bot::new(1);
        let mut b = [0f32; BLOCK];
        let mut all = Vec::new();
        for _ in 0..3000 {
            c.block(&mut b);
            bot.step(&mut c);
            all.extend_from_slice(&b);
        }
        all
    };
    assert!(take(1) == take(2));
}

#[test]
fn different_seeds_give_different_callers() {
    let run = |seed| {
        let mut c = Contest::new(clean(), 1);
        c.run_code(&coded(RunMode::Single, seed, &clean()));
        run_secs(&mut c, None, 1.0);
        c.stations[0].st.my_call.clone()
    };
    let calls: std::collections::HashSet<_> = (0..20).map(run).collect();
    assert!(calls.len() >= 18, "{calls:?}");
}

#[test]
fn a_plain_run_has_a_code_that_repeats_it() {
    let mut a = Contest::new(clean(), 21);
    a.run(RunMode::Single);
    let code = a.code.clone().unwrap();
    run_secs(&mut a, None, 1.0);
    let mut b = Contest::new(clean(), 22);
    b.run_code(&code);
    run_secs(&mut b, None, 1.0);
    assert_eq!(a.stations[0].st.my_call, b.stations[0].st.my_call);
}

#[test]
fn a_change_during_a_plain_run_ends_its_code() {
    let mut c = Contest::new(clean(), 23);
    c.run(RunMode::Pileup);
    assert!(c.code.is_some());
    c.key(Key::Rit(1));
    assert!(c.code.is_some(), "RIT is the operator's own business");
    c.key(Key::Speed(1));
    assert!(c.code.is_none());
}

#[test]
fn a_coded_run_holds_its_conditions_and_gives_the_settings_back() {
    let mine = Settings { wpm: 22, bandwidth: 300, pitch: 500, activity: 6, duration: 11, qsb: true, qsk: false, ..clean() };
    let code = coded(RunMode::Pileup, 3, &Settings { wpm: 35, bandwidth: 400, pitch: 700, activity: 2, duration: 1, ..clean() });
    let mut c = Contest::new(mine.clone(), 24);
    c.run_code(&code);
    assert_eq!((c.settings.wpm, c.settings.bandwidth, c.settings.pitch), (35, 400, 700));
    assert_eq!(c.me.st.wpm, 35);
    c.key(Key::Speed(1));
    c.key(Key::Bandwidth(-1));
    let mut changed = c.settings.clone();
    changed.wpm = 12;
    changed.qrn = true;
    changed.mon_db = -6.0;
    c.apply(changed);
    assert_eq!((c.settings.wpm, c.settings.bandwidth, c.settings.qrn), (35, 400, false));
    assert_eq!(c.settings.mon_db, -6.0, "personal settings stay free");
    assert_eq!(c.code.as_ref(), Some(&code));
    run_secs(&mut c, None, 61.0);
    assert_eq!(c.mode, RunMode::Stop);
    assert_eq!(c.finished.as_ref().unwrap().code, Some(code.encode()));
    let mut back = c.settings.clone();
    back.mon_db = mine.mon_db;
    assert_eq!(back, mine);
    assert!(!c.fixed);
}

#[test]
fn a_coded_competition_keeps_its_own_time() {
    let code = coded(RunMode::Wpx, 4, &Settings { duration: 2, ..clean() });
    let mut c = Contest::new(Settings { competition_duration: 60, ..clean() }, 25);
    c.run_code(&code);
    assert_eq!(c.settings.duration, 2);
    assert!(!c.settings.qsb, "the code rules, not the mode");
}

#[test]
fn stopping_a_coded_run_by_hand_gives_no_result() {
    let mut c = Contest::new(clean(), 26);
    c.run_code(&coded(RunMode::Pileup, 1, &clean()));
    run_secs(&mut c, None, 1.0);
    c.request_stop();
    run_secs(&mut c, None, 0.1);
    assert!(c.finished.is_none());
}

fn sent_by(c: &Contest, who: Sender) -> Vec<String> {
    c.sent.lines().filter(|l| l.who == who).map(|l| l.text.clone()).collect()
}

#[test]
fn the_record_shows_my_message_and_the_answers() {
    let mut c = Contest::new(clean(), 3);
    c.settings.activity = 9;
    c.run(RunMode::Pileup);
    c.me_send(Msg::Cq);
    run_secs(&mut c, None, 8.0);
    let me = sent_by(&c, Sender::Me);
    assert_eq!(me, ["CQ DL8UG TEST"]);
    assert!(c.sent.lines().next().unwrap().t < 0.1);
    assert!(!sent_by(&c, Sender::Caller).is_empty());
    for l in c.sent.lines().filter(|l| l.who == Sender::Caller) {
        assert!(l.text.contains(&l.call), "{} sent {}", l.call, l.text);
    }
}

#[test]
fn the_record_shows_a_message_in_one_line() {
    let mut c = Contest::new(clean(), 3);
    c.run(RunMode::Pileup);
    c.entry.call.text = "W1ABC".into();
    c.me.st.his_call = "W1ABC".into();
    c.me_send(Msg::HisCall);
    c.me_send(Msg::Nr);
    run_secs(&mut c, None, 0.2);
    assert!(c.sent.lines().last().unwrap().on_air);
    run_secs(&mut c, None, 8.0);
    let me = sent_by(&c, Sender::Me);
    assert_eq!(me.len(), 1, "{me:?}");
    assert!(me[0].starts_with("W1ABC 5NN"), "{me:?}");
    assert!(!c.sent.lines().next().unwrap().on_air);
}

#[test]
fn the_record_marks_a_stopped_message() {
    let mut c = Contest::new(clean(), 3);
    c.run(RunMode::Pileup);
    c.me_send(Msg::Cq);
    run_secs(&mut c, None, 0.2);
    c.abort_send();
    c.me_send(Msg::Tu);
    run_secs(&mut c, None, 3.0);
    let mine: Vec<_> = c.sent.lines().filter(|l| l.who == Sender::Me).collect();
    assert_eq!(mine.len(), 2);
    assert!(mine[0].cut && !mine[1].cut);
    assert_eq!(mine[1].text, "TU");
}

#[test]
fn the_record_follows_a_corrected_call() {
    let mut c = Contest::new(clean(), 3);
    c.run(RunMode::Pileup);
    c.me.st.his_call = "W1ABC".into();
    c.me_send(Msg::HisCall);
    assert!(c.update_call_in_message("W1ABCD"));
    run_secs(&mut c, None, 4.0);
    assert_eq!(sent_by(&c, Sender::Me), ["W1ABCD"]);
}

#[test]
fn a_new_run_starts_an_empty_record() {
    let mut c = Contest::new(clean(), 3);
    c.run(RunMode::Pileup);
    c.me_send(Msg::Cq);
    run_secs(&mut c, None, 1.0);
    assert!(c.sent.lines().next().is_some());
    c.run(RunMode::Stop);
    assert!(c.sent.lines().all(|l| !l.on_air));
    c.run(RunMode::Pileup);
    assert!(c.sent.lines().next().is_none());
}

#[test]
fn a_head_start_raises_the_callers_numbers() {
    let numbers = |head_start| {
        let mut c = Contest::new(Settings { head_start, ..clean() }, 4);
        c.run(RunMode::Pileup);
        let mut bot = Bot::new(4);
        run_secs(&mut c, Some(&mut bot), 300.0);
        c.log.qsos.iter().map(|q| q.true_nr).collect::<Vec<_>>()
    };
    // as in the original everybody starts with you: five minutes give at most 1 + 5 * 3
    let fresh = numbers(0);
    assert!(fresh.len() >= 5 && fresh.iter().all(|n| *n <= 16), "{fresh:?}");
    // six hours on, up to 1 + 365 * 3, and hardly a number as low as yours
    let late = numbers(6);
    assert!(late.len() >= 5 && late.iter().all(|n| *n <= 1096), "{late:?}");
    assert!(late.iter().filter(|n| **n > 100).count() * 2 >= late.len(), "{late:?}");
}

#[test]
fn a_competition_starts_with_everybody() {
    let mut c = Contest::new(Settings { head_start: 12, caller_slow: 90, caller_fast: 120, waves: true, ..clean() }, 1);
    c.run(RunMode::Wpx);
    assert_eq!((c.settings.head_start, c.settings.caller_slow, c.settings.caller_fast), (0, 50, 100));
    assert!(!c.settings.waves && c.tide() == 1.0);
    assert_eq!(c.code.as_ref().map(|k| (k.head_start, k.caller_slow, k.caller_fast)), Some((0, 50, 100)));
    c.run(RunMode::Stop);
    assert_eq!((c.settings.head_start, c.settings.caller_slow, c.settings.caller_fast), (12, 90, 120));
    assert!(c.settings.waves);
}

#[test]
fn closing_during_a_competition_keeps_your_own_settings() {
    let mine = Settings { head_start: 12, qrn: false, activity: 7, ..clean() };
    let mut c = Contest::new(mine.clone(), 1);
    c.run(RunMode::Wpx);
    assert!(c.settings.qrn && c.settings.head_start == 0);
    let kept = c.settings_to_keep();
    assert_eq!((kept.head_start, kept.qrn, kept.activity), (12, false, 7));
    let code = c.code.clone().unwrap();
    c.run(RunMode::Stop);
    let mut c = Contest::new(Settings { wpm: 18, ..mine }, 2);
    c.run_code(&code);
    assert_eq!(c.settings_to_keep().wpm, 18, "a code fixes the speed, yours comes back");
}

#[test]
fn a_misheard_call_knows_whom_it_was_meant_for() {
    let mut c = Contest::new(clean(), 6);
    c.run(RunMode::Pileup);
    let call = get_caller(&mut c);
    // the last letter wrong, as when a C is heard for a G
    let mut wrong = call.clone();
    let last = wrong.pop().unwrap();
    wrong.push(if last == 'A' { 'B' } else { 'A' });
    type_text(&mut c, &wrong);
    c.key(Key::Enter);
    settle(&mut c);
    let nr = dx_of(&c, &call).st.nr;
    type_text(&mut c, &nr.to_string());
    c.key(Key::Enter);
    settle(&mut c);
    let q = &c.log.qsos[0];
    assert_eq!(q.err, QsoErr::Nil);
    assert_eq!(q.meant.as_ref().map(|m| m.call.as_str()), Some(call.as_str()), "{q:?}");
    let a = mr_sim::analysis::analyse(&c.log.qsos);
    assert_eq!(a.misses[0].truth, call);
    assert_eq!(a.confusions[0].sent, Some(last));
}

#[test]
fn the_tide_moves_the_crowd() {
    let mut c = Contest::new(Settings { waves: true, ..clean() }, 3);
    c.run(RunMode::Pileup);
    let mut bot = Bot::new(3);
    let mut tides = Vec::new();
    for _ in 0..20 {
        run_secs(&mut c, Some(&mut bot), 60.0);
        tides.push(c.tide());
    }
    let (lo, hi) = tides.iter().fold((f64::MAX, f64::MIN), |(a, b), v| (a.min(*v), b.max(*v)));
    assert!(hi - lo > 0.5, "{tides:?}");
    assert!(c.log.qsos.len() >= 20, "the bot still works the run");
    // without it the crowd stays as in the original
    let mut c = Contest::new(clean(), 3);
    c.run(RunMode::Pileup);
    run_secs(&mut c, None, 120.0);
    assert_eq!(c.tide(), 1.0);
}
