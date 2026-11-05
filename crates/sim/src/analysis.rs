//! What went wrong in a log: every contact that is not clean, what was logged against what
//! the other side sent, and which characters get mixed up.

use crate::log::{Qso, QsoErr};

/// The station a wrong call was most likely meant for, as it was on the air.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Meant {
    pub call: String,
    pub rst: u32,
    pub nr: u32,
}

/// Edit distance of two calls, in characters.
pub fn distance(a: &str, b: &str) -> usize {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut prev = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let here = row[j + 1];
            row[j + 1] = (prev + usize::from(ca != cb)).min(row[j] + 1).min(here + 1);
            prev = here;
        }
    }
    row[b.len()]
}

/// Close enough to be the same call misheard: two characters at most, and less than half
/// of it.
pub fn misheard(logged: &str, call: &str) -> bool {
    let d = distance(logged, call);
    d > 0 && d <= 2 && 2 * d < call.chars().count()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MissKind {
    Call,
    Rst,
    Nr,
    /// A call no station had, and none came close.
    Unknown,
}

/// A contact that is not clean.
#[derive(Clone, Debug, PartialEq)]
pub struct Miss {
    pub t: f64,
    pub kind: MissKind,
    pub logged: String,
    /// What the other side sent, empty when nobody knows.
    pub truth: String,
}

/// A character sent and what was written for it. `None` sent is a character too many,
/// `None` written one left out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Confusion {
    pub sent: Option<char>,
    pub written: Option<char>,
    pub n: u32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Analysis {
    pub qsos: usize,
    pub good: usize,
    pub dupes: usize,
    pub misses: Vec<Miss>,
    /// Most frequent first.
    pub confusions: Vec<Confusion>,
}

/// The steps that turn `sent` into `written`, character by character.
fn align(sent: &str, written: &str) -> Vec<(Option<char>, Option<char>)> {
    let (a, b): (Vec<char>, Vec<char>) = (sent.chars().collect(), written.chars().collect());
    let mut d = vec![vec![0usize; b.len() + 1]; a.len() + 1];
    for (i, row) in d.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, v) in d[0].iter_mut().enumerate() {
        *v = j;
    }
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            d[i][j] = (d[i - 1][j - 1] + usize::from(a[i - 1] != b[j - 1])).min(d[i - 1][j] + 1).min(d[i][j - 1] + 1);
        }
    }
    let (mut i, mut j, mut out) = (a.len(), b.len(), Vec::new());
    while i > 0 || j > 0 {
        // a wrong character before a missing or an extra one
        if i > 0 && j > 0 && d[i][j] == d[i - 1][j - 1] + usize::from(a[i - 1] != b[j - 1]) {
            if a[i - 1] != b[j - 1] {
                out.push((Some(a[i - 1]), Some(b[j - 1])));
            }
            i -= 1;
            j -= 1;
        } else if i > 0 && d[i][j] == d[i - 1][j] + 1 {
            out.push((Some(a[i - 1]), None));
            i -= 1;
        } else {
            out.push((None, Some(b[j - 1])));
            j -= 1;
        }
    }
    out.reverse();
    out
}

pub fn analyse(qsos: &[Qso]) -> Analysis {
    let mut a = Analysis { qsos: qsos.len(), ..Analysis::default() };
    for q in qsos {
        let miss = match q.err {
            QsoErr::Ok => {
                a.good += 1;
                None
            }
            QsoErr::Dup => {
                a.dupes += 1;
                None
            }
            QsoErr::Nil => Some(match &q.meant {
                Some(m) => Miss { t: q.t, kind: MissKind::Call, logged: q.call.clone(), truth: m.call.clone() },
                None => Miss { t: q.t, kind: MissKind::Unknown, logged: q.call.clone(), truth: String::new() },
            }),
            QsoErr::Rst => Some(Miss { t: q.t, kind: MissKind::Rst, logged: q.rst.to_string(), truth: q.true_rst.to_string() }),
            QsoErr::Nr => Some(Miss { t: q.t, kind: MissKind::Nr, logged: q.nr.to_string(), truth: q.true_nr.to_string() }),
        };
        if let Some(m) = miss {
            if !m.truth.is_empty() {
                for (sent, written) in align(&m.truth, &m.logged) {
                    match a.confusions.iter_mut().find(|c| c.sent == sent && c.written == written) {
                        Some(c) => c.n += 1,
                        None => a.confusions.push(Confusion { sent, written, n: 1 }),
                    }
                }
            }
            a.misses.push(m);
        }
    }
    // most frequent first, then in the order of the alphabet
    a.confusions.sort_by(|x, y| y.n.cmp(&x.n).then(x.sent.cmp(&y.sent)).then(x.written.cmp(&y.written)));
    a
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(call: &str, err: QsoErr, meant: Option<&str>) -> Qso {
        Qso {
            t: 1.0,
            call: call.into(),
            true_call: if err == QsoErr::Nil { String::new() } else { call.into() },
            rst: 599,
            true_rst: if err == QsoErr::Rst { 579 } else { 599 },
            nr: 128,
            true_nr: if err == QsoErr::Nr { 123 } else { 128 },
            sent_rst: 599,
            sent_nr: 1,
            pfx: String::new(),
            dupe: err == QsoErr::Dup,
            err,
            wpm: 25,
            asks: 0,
            conditions: 1.0,
            meant: meant.map(|c| Meant { call: c.into(), rst: 599, nr: 1 }),
        }
    }

    #[test]
    fn distances() {
        assert_eq!(distance("DL1ABC", "DL1ABC"), 0);
        assert_eq!(distance("DL1ABC", "DL1ABG"), 1);
        assert_eq!(distance("DL1ABC", "DL1AB"), 1);
        assert_eq!(distance("K5ZD", "K5ZDD"), 1);
        assert!(misheard("DL1ABG", "DL1ABC"));
        assert!(!misheard("W1AW", "JA1XYZ"));
        assert!(!misheard("K1", "K2"), "a short call changed in half is no mishearing");
    }

    #[test]
    fn wrong_missing_and_extra_characters() {
        assert_eq!(align("DL1ABC", "DL1ABG"), vec![(Some('C'), Some('G'))]);
        assert_eq!(align("DL1ABC", "DL1AC"), vec![(Some('B'), None)]);
        assert_eq!(align("K5ZD", "K5ZDD"), vec![(None, Some('D'))]);
        assert_eq!(align("123", "123"), vec![]);
    }

    #[test]
    fn a_log_with_every_kind_of_miss() {
        let log = vec![
            q("DL1ABC", QsoErr::Ok, None),
            q("DL1ABG", QsoErr::Nil, Some("DL1ABC")),
            q("OK1HG", QsoErr::Nil, Some("OK1HC")),
            q("XX9XX", QsoErr::Nil, None),
            q("W1AW", QsoErr::Rst, None),
            q("K5ZD", QsoErr::Nr, None),
            q("DL1ABC", QsoErr::Dup, None),
        ];
        let a = analyse(&log);
        assert_eq!((a.qsos, a.good, a.dupes, a.misses.len()), (7, 1, 1, 5));
        let kinds: Vec<MissKind> = a.misses.iter().map(|m| m.kind).collect();
        assert_eq!(kinds, [MissKind::Call, MissKind::Call, MissKind::Unknown, MissKind::Rst, MissKind::Nr]);
        assert_eq!((a.misses[3].logged.as_str(), a.misses[3].truth.as_str()), ("599", "579"));
        assert_eq!((a.misses[4].logged.as_str(), a.misses[4].truth.as_str()), ("128", "123"));
        // C for G twice comes first
        assert_eq!(a.confusions[0], Confusion { sent: Some('C'), written: Some('G'), n: 2 });
        assert_eq!(a.confusions.iter().map(|c| c.n).sum::<u32>(), 4);
    }
}
