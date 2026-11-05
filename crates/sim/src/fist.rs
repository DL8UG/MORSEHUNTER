//! How the operator keys: every mark and every pause the decoder sees, measured against the
//! set speed. With a keyer only the pauses between letters and words are the hand's own,
//! with a straight key or a bug the marks and the pauses inside a letter as well.

/// Count, sum and sum of squares, enough for mean and spread.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Stat {
    pub n: u32,
    sum: f64,
    sum2: f64,
}

impl Stat {
    fn add(&mut self, v: f64) {
        self.n += 1;
        self.sum += v;
        self.sum2 += v * v;
    }

    pub fn mean(&self) -> Option<f64> {
        (self.n > 0).then(|| self.sum / self.n as f64)
    }

    /// Spread as a share of the mean: 0.1 is a steady hand, 0.3 a restless one.
    pub fn spread(&self) -> Option<f64> {
        let m = self.mean()?;
        if self.n < 3 || m <= 0.0 {
            return None;
        }
        let var = (self.sum2 / self.n as f64 - m * m).max(0.0);
        Some(var.sqrt() / m)
    }
}

/// Longer silences than this many dits are pauses in thought, not word gaps.
const WORD_GAP_MAX: f64 = 20.0;

/// Marks and pauses in dits of the set speed.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Fist {
    /// The hand makes the marks itself: straight key or bug.
    pub by_hand: bool,
    pub dits: Stat,
    pub dahs: Stat,
    /// Between the elements of one character.
    pub element_gaps: Stat,
    pub letter_gaps: Stat,
    pub word_gaps: Stat,
    pub chars: u32,
    /// Characters that are no Morse character.
    pub unknown: u32,
    /// Error signs, eight dits or more.
    pub errors: u32,
}

impl Fist {
    pub(crate) fn mark(&mut self, dits: f64, dah: bool) {
        if dah {
            self.dahs.add(dits);
        } else {
            self.dits.add(dits);
        }
    }

    pub(crate) fn element_gap(&mut self, dits: f64) {
        self.element_gaps.add(dits);
    }

    /// The pause before a character, a word gap if the decoder took it for one.
    pub(crate) fn gap(&mut self, dits: f64, word: bool) {
        if word {
            if dits <= WORD_GAP_MAX {
                self.word_gaps.add(dits);
            }
        } else {
            self.letter_gaps.add(dits);
        }
    }

    pub(crate) fn char(&mut self, known: bool) {
        self.chars += 1;
        if !known {
            self.unknown += 1;
        }
    }

    pub(crate) fn error(&mut self) {
        self.errors += 1;
    }

    /// Dah over dit, 3 by the book.
    pub fn weight(&self) -> Option<f64> {
        Some(self.dahs.mean()? / self.dits.mean()?)
    }

    /// What stands out most, worst first. Each line is a fact with the book value next
    /// to it, so the window can say it in its own language.
    pub fn advice(&self) -> Vec<Advice> {
        let mut out = Vec::new();
        if let Some(g) = self.letter_gaps.mean() {
            if g < 2.4 {
                out.push(Advice::LettersTight(g));
            } else if g > 4.5 {
                out.push(Advice::LettersWide(g));
            }
        }
        if let (Some(w), Some(l)) = (self.word_gaps.mean(), self.letter_gaps.mean())
            && w < 1.6 * l
        {
            out.push(Advice::WordsLikeLetters(w, l));
        }
        if self.by_hand {
            if let Some(r) = self.weight()
                && !(2.5..=3.8).contains(&r)
            {
                out.push(Advice::Weight(r));
            }
            if let Some(s) = self.dits.spread().into_iter().chain(self.dahs.spread()).reduce(f64::max)
                && s > 0.25
            {
                out.push(Advice::Unsteady(s));
            }
        }
        if self.chars >= 10 && self.unknown * 10 > self.chars {
            out.push(Advice::Unreadable(self.unknown, self.chars));
        }
        out
    }
}

/// One finding about the keying.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Advice {
    /// Letter gaps in dits, book 3: below 2.4 letters run together.
    LettersTight(f64),
    LettersWide(f64),
    /// Word gap and letter gap in dits: the words do not stand apart.
    WordsLikeLetters(f64, f64),
    /// Dah over dit, book 3.
    Weight(f64),
    /// The larger spread of dits or dahs.
    Unsteady(f64),
    /// Unknown characters of all keyed.
    Unreadable(u32, u32),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mean_and_spread() {
        let mut s = Stat::default();
        assert_eq!(s.mean(), None);
        for v in [1.0, 1.0, 1.0, 1.0] {
            s.add(v);
        }
        assert_eq!(s.mean(), Some(1.0));
        assert!(s.spread().unwrap() < 1e-9);
        let mut s = Stat::default();
        for v in [2.0, 4.0, 2.0, 4.0] {
            s.add(v);
        }
        assert!((s.spread().unwrap() - 1.0 / 3.0).abs() < 1e-9);
    }

    #[test]
    fn textbook_keying_gives_no_advice() {
        let mut f = Fist { by_hand: true, ..Fist::default() };
        for _ in 0..20 {
            f.mark(1.0, false);
            f.mark(3.0, true);
            f.gap(3.0, false);
            f.char(true);
        }
        f.gap(7.0, true);
        assert_eq!(f.advice(), vec![]);
        assert!((f.weight().unwrap() - 3.0).abs() < 1e-9);
    }

    #[test]
    fn crowded_letters_and_light_dahs_are_named() {
        let mut f = Fist { by_hand: true, ..Fist::default() };
        for _ in 0..20 {
            f.mark(1.0, false);
            f.mark(2.0, true);
            f.gap(1.8, false);
            f.char(true);
        }
        let a = f.advice();
        assert!(matches!(a[0], Advice::LettersTight(g) if (g - 1.8).abs() < 1e-9), "{a:?}");
        assert!(a.iter().any(|x| matches!(x, Advice::Weight(r) if (r - 2.0).abs() < 1e-9)), "{a:?}");
    }

    #[test]
    fn long_pauses_are_no_word_gaps() {
        let mut f = Fist::default();
        f.gap(7.0, true);
        f.gap(60.0, true);
        assert_eq!(f.word_gaps.n, 1);
    }
}
