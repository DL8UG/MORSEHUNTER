//! Skill points: a score for every kind of run, made of speed, clean contacts, questions
//! asked and band conditions. It is not part of the original, which only scores WPX.
//!
//! A good contact is worth `BASE` at `REF_WPM`, in proportion to the speed. Each question
//! in it (`?`, `NR?`, `AGN`) takes `ASK_COST` of that, down to `ASK_FLOOR`. A busted contact
//! costs half its worth, a dupe nothing. Every band effect on adds `COND_STEP`. Runs of
//! different length compare by points per hour.

use crate::log::{Qso, QsoErr};
use crate::settings::Settings;

pub const BASE: f64 = 10.0;
pub const REF_WPM: f64 = 25.0;
pub const ASK_COST: f64 = 0.2;
pub const ASK_FLOOR: f64 = 0.4;
pub const BUST_COST: f64 = 0.5;
pub const COND_STEP: f64 = 0.1;

/// The factor for the band conditions as they are now.
pub fn conditions(s: &Settings) -> f64 {
    let on = [s.qrn, s.qrm, s.qsb, s.flutter, s.lids].iter().filter(|b| **b).count();
    1.0 + COND_STEP * on as f64 + head_start_bonus(s.head_start)
}

/// Longer numbers are harder to copy: three digits from 1 h on, four from 6 h on.
pub fn head_start_bonus(hours: u32) -> f64 {
    match hours {
        0 => 0.0,
        1..6 => COND_STEP,
        _ => 2.0 * COND_STEP,
    }
}

/// What one contact is worth, negative for a busted one.
pub fn value(q: &Qso) -> f64 {
    let worth = BASE * q.wpm as f64 / REF_WPM * q.conditions;
    match q.err {
        QsoErr::Ok => worth * (1.0 - ASK_COST * q.asks as f64).max(ASK_FLOOR),
        QsoErr::Dup => 0.0,
        QsoErr::Nil | QsoErr::Rst | QsoErr::Nr => -BUST_COST * worth,
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Skill {
    /// The sum over the log, never below zero.
    pub points: u64,
    /// The sum scaled to an hour.
    pub per_hour: u64,
    pub good: u32,
    pub busted: u32,
    pub asks: u32,
    /// Mean speed of the good contacts.
    pub wpm: u32,
}

/// The skill of a log worked in `secs` seconds.
pub fn skill(qsos: &[Qso], secs: f64) -> Skill {
    let sum: f64 = qsos.iter().map(value).sum();
    let good: Vec<&Qso> = qsos.iter().filter(|q| q.err == QsoErr::Ok).collect();
    let points = sum.max(0.0);
    let per_hour = if secs > 0.0 { points * 3600.0 / secs } else { 0.0 };
    let wpm = if good.is_empty() { 0 } else { (good.iter().map(|q| q.wpm as f64).sum::<f64>() / good.len() as f64).round() as u32 };
    Skill {
        points: points.round() as u64,
        per_hour: per_hour.round() as u64,
        good: good.len() as u32,
        busted: qsos.iter().filter(|q| matches!(q.err, QsoErr::Nil | QsoErr::Rst | QsoErr::Nr)).count() as u32,
        asks: qsos.iter().map(|q| q.asks).sum(),
        wpm,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(err: QsoErr, wpm: u32, asks: u32, conditions: f64) -> Qso {
        Qso {
            t: 0.0,
            call: "DL1ABC".into(),
            true_call: "DL1ABC".into(),
            rst: 599,
            true_rst: 599,
            nr: 1,
            true_nr: 1,
            sent_rst: 599,
            sent_nr: 1,
            pfx: "DL1".into(),
            dupe: false,
            err,
            wpm,
            asks,
            conditions,
            meant: None,
        }
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn a_clean_contact_follows_the_speed() {
        assert!(close(value(&q(QsoErr::Ok, 25, 0, 1.0)), 10.0));
        assert!(close(value(&q(QsoErr::Ok, 35, 0, 1.0)), 14.0));
        assert!(close(value(&q(QsoErr::Ok, 25, 0, 1.5)), 15.0));
    }

    #[test]
    fn questions_cost_but_never_all() {
        assert!(close(value(&q(QsoErr::Ok, 25, 1, 1.0)), 8.0));
        assert!(close(value(&q(QsoErr::Ok, 25, 2, 1.0)), 6.0));
        assert!(close(value(&q(QsoErr::Ok, 25, 9, 1.0)), 4.0));
    }

    #[test]
    fn busts_cost_and_dupes_count_nothing() {
        for e in [QsoErr::Nil, QsoErr::Rst, QsoErr::Nr] {
            assert!(close(value(&q(e, 25, 0, 1.0)), -5.0));
        }
        assert!(close(value(&q(QsoErr::Dup, 25, 0, 1.0)), 0.0));
    }

    #[test]
    fn a_head_start_adds_for_the_longer_numbers() {
        let none = Settings { qrn: false, qrm: false, qsb: false, flutter: false, lids: false, ..Settings::default() };
        assert!(close(conditions(&none), 1.0));
        assert!(close(conditions(&Settings { head_start: 1, ..none.clone() }), 1.1));
        assert!(close(conditions(&Settings { head_start: 5, ..none.clone() }), 1.1));
        assert!(close(conditions(&Settings { head_start: 6, ..none.clone() }), 1.2));
        assert!(close(conditions(&Settings { head_start: 48, ..none }), 1.2));
    }

    #[test]
    fn conditions_add_a_tenth_each() {
        let s = Settings { qrn: true, qrm: true, qsb: false, flutter: false, lids: true, ..Settings::default() };
        assert!(close(conditions(&s), 1.3));
    }

    #[test]
    fn the_sum_scales_to_an_hour_and_stays_positive() {
        let log = vec![q(QsoErr::Ok, 25, 0, 1.0), q(QsoErr::Ok, 25, 1, 1.0), q(QsoErr::Nr, 25, 0, 1.0)];
        let s = skill(&log, 600.0);
        assert_eq!((s.points, s.per_hour, s.good, s.busted, s.asks, s.wpm), (13, 78, 2, 1, 1, 25));
        assert_eq!(skill(&[q(QsoErr::Nil, 25, 0, 1.0)], 600.0).points, 0);
    }
}
