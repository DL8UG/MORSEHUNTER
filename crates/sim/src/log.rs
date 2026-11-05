//! The contest log and what can be read from it: errors, prefixes, score, rate.

use std::collections::BTreeSet;

use crate::morse;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QsoErr {
    Ok,
    Nil,
    Dup,
    Rst,
    Nr,
}

impl QsoErr {
    pub fn label(self) -> &'static str {
        match self {
            QsoErr::Ok => "",
            QsoErr::Nil => "NIL",
            QsoErr::Dup => "DUP",
            QsoErr::Rst => "RST",
            QsoErr::Nr => "NR",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Qso {
    /// Seconds into the run.
    pub t: f64,
    pub call: String,
    pub true_call: String,
    pub rst: u32,
    pub true_rst: u32,
    pub nr: u32,
    pub true_nr: u32,
    pub sent_rst: u32,
    pub sent_nr: u32,
    pub pfx: String,
    pub dupe: bool,
    pub err: QsoErr,
    /// My speed when the contact went into the log.
    pub wpm: u32,
    /// Questions I asked in this contact: `?`, `NR?`, `AGN`.
    pub asks: u32,
    /// The factor for the band conditions when the contact went into the log.
    pub conditions: f64,
    /// For a call no station had: the one it was most likely meant for. Only for the
    /// analysis, the score goes by the other side's log alone.
    pub meant: Option<crate::analysis::Meant>,
}

impl Qso {
    /// What the other station's log says decides.
    pub fn check(&mut self) {
        self.err = if self.true_call.is_empty() {
            QsoErr::Nil
        } else if self.dupe {
            QsoErr::Dup
        } else if self.true_rst != self.rst {
            QsoErr::Rst
        } else if self.true_nr != self.nr {
            QsoErr::Nr
        } else {
            QsoErr::Ok
        };
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub points: u64,
    pub mults: u64,
    pub score: u64,
}

/// The WPX prefix of a call: letters up to the last digit, at least two characters.
pub fn extract_prefix(call: &str) -> String {
    let mut c = call.to_string();
    for suffix in ["/QRP", "/MM", "/M", "/P"] {
        if let Some(s) = c.strip_suffix(suffix) {
            c = s.to_string();
            break;
        }
    }
    let c = c.replace("//", "/");
    if c.len() < 2 {
        return String::new();
    }

    let mut digit = None;
    let mut r = match c.find('/') {
        None => c.clone(),
        Some(0) => c[1..].to_string(),
        Some(p) if p == c.len() - 1 => c[..p].to_string(),
        Some(p) => {
            let (a, b) = (&c[..p], &c[p + 1..]);
            let is_digit = |s: &str| s.len() == 1 && s.as_bytes()[0].is_ascii_digit();
            if is_digit(a) {
                digit = Some(a.as_bytes()[0]);
                b.to_string()
            } else if is_digit(b) {
                digit = Some(b.as_bytes()[0]);
                a.to_string()
            } else if a.len() <= b.len() {
                a.to_string()
            } else {
                b.to_string()
            }
        }
    };
    if r.is_empty() || r.contains('/') {
        return String::new();
    }

    while r.len() > 2 && !r.as_bytes()[r.len() - 1].is_ascii_digit() {
        r.pop();
    }
    if !r.as_bytes()[r.len() - 1].is_ascii_digit() {
        r.push('0');
    }
    if let Some(d) = digit {
        r.pop();
        r.push(d as char);
    }
    r.truncate(5);
    r
}

/// HST points of a call: its length in Morse units.
pub fn call_to_score(call: &str) -> u64 {
    let mut score = -1i64;
    for c in morse::encode(call).chars() {
        score += match c {
            '.' | ' ' => 2,
            '-' => 4,
            _ => 0,
        };
    }
    score.max(0) as u64
}

#[derive(Clone, Debug, Default)]
pub struct Log {
    pub qsos: Vec<Qso>,
}

const BIN_S: f64 = 300.0;
pub const HISTO_BINS: usize = 48;

impl Log {
    fn stats<'a>(list: impl Iterator<Item = &'a Qso>) -> Stats {
        let mut pfx = BTreeSet::new();
        let mut points = 0;
        for q in list {
            points += 1;
            pfx.insert(q.pfx.as_str());
        }
        let mults = pfx.len() as u64;
        Stats { points, mults, score: points * mults }
    }

    pub fn raw(&self) -> Stats {
        Self::stats(self.qsos.iter())
    }

    pub fn verified(&self) -> Stats {
        Self::stats(self.qsos.iter().filter(|q| q.err == QsoErr::Ok))
    }

    /// HST score over all contacts and over the clean ones.
    pub fn hst(&self) -> (u64, u64) {
        let (mut raw, mut clean) = (0, 0);
        for q in &self.qsos {
            let s = call_to_score(&q.call);
            raw += s;
            if q.err == QsoErr::Ok {
                clean += s;
            }
        }
        (raw, clean)
    }

    /// QSOs per hour over the last five minutes.
    pub fn rate(&self, now: f64) -> u32 {
        if now <= 0.0 {
            return 0;
        }
        let d = now.min(BIN_S);
        let n = self.qsos.iter().rev().take_while(|q| q.t > now - d).count();
        (n as f64 * 3600.0 / d).round() as u32
    }

    /// QSOs in each five-minute block.
    pub fn histogram(&self) -> [u32; HISTO_BINS] {
        let mut h = [0; HISTO_BINS];
        for q in &self.qsos {
            h[((q.t / BIN_S) as usize).min(HISTO_BINS - 1)] += 1;
        }
        h
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(call: &str, err: QsoErr, t: f64) -> Qso {
        Qso {
            t,
            call: call.into(),
            true_call: call.into(),
            rst: 599,
            true_rst: 599,
            nr: 1,
            true_nr: 1,
            sent_rst: 599,
            sent_nr: 1,
            pfx: extract_prefix(call),
            dupe: false,
            err,
            wpm: 25,
            asks: 0,
            conditions: 1.0,
            meant: None,
        }
    }

    #[test]
    fn prefixes() {
        for (c, p) in [
            ("W1ABC", "W1"),
            ("DL8UG", "DL8"),
            ("5Z4AB", "5Z4"),
            ("HB9XY", "HB9"),
            ("S51AB", "S51"),
            ("W1ABC/P", "W1"),
            ("W1ABC/QRP", "W1"),
            ("W1ABC/MM", "W1"),
            ("DL/W1ABC", "DL0"),
            ("W1ABC/4", "W4"),
            ("4/W1ABC", "W4"),
            ("VP2E/W1ABC", "VP2"),
            ("RAEM", "RA0"),
            ("N", ""),
        ] {
            assert_eq!(extract_prefix(c), p, "{c}");
        }
    }

    #[test]
    fn error_classes() {
        let mut a = q("W1ABC", QsoErr::Ok, 0.0);
        a.check();
        assert_eq!(a.err, QsoErr::Ok);
        let mut b = Qso { true_call: String::new(), ..a.clone() };
        b.check();
        assert_eq!(b.err, QsoErr::Nil);
        let mut c = Qso { dupe: true, true_rst: 559, ..a.clone() };
        c.check();
        assert_eq!(c.err, QsoErr::Dup);
        let mut d = Qso { true_rst: 559, true_nr: 7, ..a.clone() };
        d.check();
        assert_eq!(d.err, QsoErr::Rst);
        let mut e = Qso { true_nr: 7, ..a };
        e.check();
        assert_eq!(e.err, QsoErr::Nr);
    }

    #[test]
    fn score_counts_prefixes_once() {
        let log = Log { qsos: vec![q("W1ABC", QsoErr::Ok, 1.0), q("W1XYZ", QsoErr::Ok, 2.0), q("DL8UG", QsoErr::Nil, 3.0)] };
        assert_eq!(log.raw(), Stats { points: 3, mults: 2, score: 6 });
        assert_eq!(log.verified(), Stats { points: 2, mults: 1, score: 2 });
    }

    #[test]
    fn call_points() {
        // E: 1 element of 2 units, minus the last gap
        assert_eq!(call_to_score("E"), 1);
        // A: dit 2, dah 4 -> 5
        assert_eq!(call_to_score("A"), 5);
    }

    #[test]
    fn rate_and_histogram() {
        let log = Log { qsos: vec![q("W1A", QsoErr::Ok, 10.0), q("W1B", QsoErr::Ok, 400.0), q("W1C", QsoErr::Ok, 500.0)] };
        assert_eq!(log.rate(600.0), 24);
        assert_eq!(log.rate(0.0), 0);
        let h = log.histogram();
        assert_eq!((h[0], h[1]), (1, 2));
    }
}
