//! A plain operator for tests and for listening without a key: it copies calls from the
//! stations themselves and works them with the Enter key, like the keyboard drill in the
//! original.

use crate::contest::{Contest, RunMode};
use crate::dx::OpState;
use crate::entry::{Field, Key};
use crate::rng::Rng;
use crate::station::State;
use crate::{RATE, BLOCK};

pub struct Bot {
    rng: Rng,
    /// Chance to get a character of a call wrong.
    pub miscopy: f64,
    idle_blocks: u32,
}

impl Bot {
    pub fn new(seed: u64) -> Self {
        Bot { rng: Rng::new(seed), miscopy: 0.0, idle_blocks: 0 }
    }

    fn type_text(&mut self, c: &mut Contest, text: &str) {
        for ch in text.chars() {
            let ch = if self.rng.chance(self.miscopy) { 'X' } else { ch };
            c.key(Key::Char(ch));
        }
    }

    fn field(&self, c: &mut Contest, f: Field) {
        while c.entry.focus != f {
            c.key(Key::Tab);
        }
    }

    /// Looks at the band and acts; call once per block.
    pub fn step(&mut self, c: &mut Contest) {
        if c.mode == RunMode::Stop || c.me.st.state == State::Sending {
            return;
        }
        let dx: Vec<(String, OpState, State, u32, u32)> = c
            .stations
            .iter()
            .filter_map(|s| s.dx().map(|d| (s.st.my_call.clone(), d.op.state, s.st.state, s.st.nr, s.st.rst)))
            .collect();
        // never talk over a station
        if dx.iter().any(|d| d.2 == State::Sending) {
            self.idle_blocks = 0;
            return;
        }
        self.idle_blocks += 1;
        if (self.idle_blocks as usize) < RATE as usize / BLOCK / 5 {
            return;
        }
        self.idle_blocks = 0;

        if c.entry.call.text.is_empty() {
            match dx.iter().find(|d| d.1 == OpState::NeedQso) {
                Some(d) => {
                    self.type_text(c, &d.0.clone());
                    c.key(Key::Enter);
                }
                None => c.key(Key::Enter),
            }
            return;
        }
        let mine = dx.iter().find(|d| d.0 == c.entry.call.text);
        match mine {
            Some(d) if matches!(d.1, OpState::NeedEnd) => {
                let (nr, rst) = (d.3.to_string(), d.4.to_string());
                self.field(c, Field::Rst);
                for _ in 0..3 {
                    c.key(Key::Backspace);
                }
                c.key(Key::End);
                for _ in 0..3 {
                    c.key(Key::Backspace);
                }
                self.type_text(c, &rst);
                self.field(c, Field::Nr);
                self.type_text(c, &nr);
                c.key(Key::Enter);
            }
            Some(d) if matches!(d.1, OpState::NeedNr | OpState::NeedCallNr | OpState::NeedCall) => c.key(Key::Enter),
            _ => c.key(Key::Wipe),
        }
    }
}
