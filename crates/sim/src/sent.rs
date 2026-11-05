//! What went out on the air, for the debug window. This only watches: it draws no random
//! number and touches no audio, so a run is the same with the window open or closed.

use std::collections::VecDeque;

/// Enough for a long stretch of scrolling back, old lines fall off.
const KEEP: usize = 500;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sender {
    Me,
    Caller,
    Qrm,
}

/// One transmission, it grows while a message is still being added to.
#[derive(Clone, Debug)]
pub struct Sent {
    id: u64,
    /// Seconds into the run.
    pub t: f64,
    pub who: Sender,
    pub call: String,
    pub text: String,
    /// Keyed by hand: the text is what the decoder read.
    pub hand: bool,
    pub on_air: bool,
    /// Stopped before the end, not all of the text went out.
    pub cut: bool,
}

#[derive(Default)]
pub struct SentLog {
    lines: VecDeque<Sent>,
    next_id: u64,
}

impl SentLog {
    pub fn lines(&self) -> impl Iterator<Item = &Sent> {
        self.lines.iter()
    }

    pub fn clear(&mut self) {
        self.lines.clear();
    }

    fn find(&mut self, id: u64) -> Option<&mut Sent> {
        self.lines.iter_mut().rev().find(|l| l.id == id)
    }

    /// A station has this on the air now. `line` remembers its entry between blocks.
    pub(crate) fn show(&mut self, line: &mut Option<u64>, t: f64, who: Sender, call: &str, text: &str, hand: bool) {
        if let Some(l) = line.and_then(|id| self.find(id)) {
            if l.text != text {
                l.text = text.to_string();
            }
            l.on_air = true;
            return;
        }
        let id = self.next_id;
        self.next_id += 1;
        *line = Some(id);
        self.lines.push_back(Sent { id, t, who, call: call.to_string(), text: text.to_string(), hand, on_air: true, cut: false });
        if self.lines.len() > KEEP {
            self.lines.pop_front();
        }
    }

    /// The transmission ended, the next one gets its own line.
    pub(crate) fn close(&mut self, line: &mut Option<u64>, cut: bool) {
        if let Some(l) = line.take().and_then(|id| self.find(id)) {
            l.on_air = false;
            l.cut |= cut;
        }
    }

    /// Nothing is on the air until `show` says otherwise.
    pub(crate) fn quiet(&mut self) {
        for l in &mut self.lines {
            l.on_air = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(log: &SentLog) -> Vec<&str> {
        log.lines().map(|l| l.text.as_str()).collect()
    }

    #[test]
    fn a_growing_message_stays_on_one_line() {
        let (mut log, mut line) = (SentLog::default(), None);
        log.show(&mut line, 1.0, Sender::Caller, "W1AW", "W1AW", false);
        log.show(&mut line, 2.0, Sender::Caller, "W1AW", "W1AW W1AW", false);
        assert_eq!(text(&log), ["W1AW W1AW"]);
        assert_eq!(log.lines().next().unwrap().t, 1.0);
    }

    #[test]
    fn a_closed_line_stays_closed() {
        let (mut log, mut line) = (SentLog::default(), None);
        log.show(&mut line, 1.0, Sender::Me, "DL8UG", "CQ", false);
        log.close(&mut line, true);
        log.show(&mut line, 2.0, Sender::Me, "DL8UG", "TU", false);
        assert_eq!(text(&log), ["CQ", "TU"]);
        let first = log.lines().next().unwrap();
        assert!(first.cut && !first.on_air);
    }

    #[test]
    fn old_lines_fall_off() {
        let mut log = SentLog::default();
        for i in 0..KEEP + 10 {
            let mut line = None;
            log.show(&mut line, i as f64, Sender::Caller, "K1", "K1", false);
        }
        assert_eq!(log.lines().count(), KEEP);
        assert_eq!(log.lines().next().unwrap().t, 10.0);
    }

    #[test]
    fn a_line_that_fell_off_starts_a_new_one() {
        let (mut log, mut line) = (SentLog::default(), None);
        log.show(&mut line, 0.0, Sender::Caller, "K1", "K1", false);
        log.clear();
        log.show(&mut line, 1.0, Sender::Caller, "K1", "K1 K1", false);
        assert_eq!(text(&log), ["K1 K1"]);
    }
}
