//! A run as a short text to pass on. It holds the seed and every condition that shapes the
//! run, so two operators who enter the same code work the same stations in the same order
//! on the same band. Personal things stay out: own call, sidetone level, language, keys.

use crate::callgen::WORLD_MIX;
use crate::contest::RunMode;
use crate::settings::{
    ACTIVITY_MAX, BW_MAX, BW_MIN, CALLER_FAST_DEFAULT, CALLER_FAST_MAX, CALLER_SLOW_DEFAULT, CALLER_SLOW_MIN, DURATION_MAX, HEAD_START_MAX, PITCH_MAX, PITCH_MIN, STEP_HZ,
    Settings, WPM_MAX, WPM_MIN,
};

/// Raise this with every change that makes the same seed sound or behave differently,
/// old codes are then refused instead of giving a different run. Raise it as well when the
/// layout of the code changes, and list the old number in `STILL_READ` if its codes still
/// give the same run.
pub const SIM_VERSION: u32 = 4;
/// Older versions whose codes still give the same run here. 3 is 4 without the tide, the
/// same run as one without; 2 is 3 without the callers' speeds, the same run as 50 to
/// 100 %; 1 is 2 without the head start, the same run as 0 h. Empty this list whenever the simulation itself changes: then no older code
/// gives the same run any more.
const STILL_READ: &[u32] = &[1, 2, 3];
/// Bits of the head start in hours.
const HEAD_BITS: u32 = 6;

const PREFIX: &str = "MR";
/// Crockford's alphabet: no I, L, O or U, nothing that reads like something else.
const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
const GROUP: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodeError {
    /// Not a code at all.
    Format,
    /// A character is wrong.
    Checksum,
    /// Made by a version with a different simulation.
    Version(u32),
    /// Passes the checksum, but a value is out of range.
    Range,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunCode {
    pub mode: RunMode,
    pub seed: u32,
    pub wpm: u32,
    pub pitch: u32,
    pub bandwidth: u32,
    pub activity: u32,
    pub duration: u32,
    pub qsk: bool,
    pub qrn: bool,
    pub qrm: bool,
    pub qsb: bool,
    pub flutter: bool,
    pub lids: bool,
    pub mix: [u8; 6],
    /// Hours the contest has been going.
    pub head_start: u32,
    /// The slowest and the fastest callers in percent of `wpm`.
    pub caller_slow: u32,
    pub caller_fast: u32,
    /// Callers come in waves.
    pub waves: bool,
}

#[derive(Default)]
struct Bits(Vec<bool>);

impl Bits {
    fn put(&mut self, value: u32, n: u32) {
        for i in (0..n).rev() {
            self.0.push(value >> i & 1 != 0);
        }
    }

    fn checksum(&self, version: u32) -> u32 {
        // FNV-1a over the bits, folded to 16
        let h = self.0.iter().fold(0x811C_9DC5u32 ^ version, |h, b| (h ^ (*b as u32 + 1)).wrapping_mul(0x0100_0193));
        (h >> 16) ^ (h & 0xFFFF)
    }
}

struct Reader<'a> {
    bits: &'a [bool],
    pos: usize,
}

impl Reader<'_> {
    fn get(&mut self, n: usize) -> Result<u32, CodeError> {
        let part = self.bits.get(self.pos..self.pos + n).ok_or(CodeError::Format)?;
        self.pos += n;
        Ok(part.iter().fold(0, |v, b| v << 1 | *b as u32))
    }

    fn flag(&mut self) -> Result<bool, CodeError> {
        Ok(self.get(1)? != 0)
    }
}

fn mode_no(m: RunMode) -> u32 {
    match m {
        RunMode::Pileup | RunMode::Stop => 0,
        RunMode::Single => 1,
        RunMode::Wpx => 2,
        RunMode::Hst => 3,
    }
}

fn default_mix() -> [u8; 6] {
    WORLD_MIX.map(|v| v as u8)
}

impl RunCode {
    /// The run these settings give in a mode, with whole percentages for the regions.
    pub fn from_settings(mode: RunMode, s: &Settings, seed: u32) -> Self {
        let mut mix = s.region_mix.map(|v| v.round().clamp(0.0, 100.0) as u8);
        if mix == [0; 6] {
            mix[0] = 100;
        }
        RunCode {
            mode,
            seed,
            wpm: s.wpm,
            pitch: s.pitch,
            bandwidth: s.bandwidth,
            activity: s.activity,
            duration: s.duration,
            qsk: s.qsk,
            qrn: s.qrn,
            qrm: s.qrm,
            qsb: s.qsb,
            flutter: s.flutter,
            lids: s.lids,
            mix,
            head_start: s.head_start.min(HEAD_START_MAX),
            caller_slow: s.caller_slow.clamp(CALLER_SLOW_MIN, 100),
            caller_fast: s.caller_fast.clamp(100, CALLER_FAST_MAX),
            waves: s.waves,
        }
    }

    /// Puts the conditions of the run into the settings.
    pub fn apply(&self, s: &mut Settings) {
        s.wpm = self.wpm;
        s.pitch = self.pitch;
        s.bandwidth = self.bandwidth;
        s.activity = self.activity;
        s.duration = self.duration;
        s.qsk = self.qsk;
        s.qrn = self.qrn;
        s.qrm = self.qrm;
        s.qsb = self.qsb;
        s.flutter = self.flutter;
        s.lids = self.lids;
        s.region_mix = self.mix.map(f64::from);
        s.head_start = self.head_start;
        (s.caller_slow, s.caller_fast) = (self.caller_slow, self.caller_fast);
        s.waves = self.waves;
    }

    fn payload(&self) -> Bits {
        let mut b = Bits::default();
        b.put(SIM_VERSION, 6);
        b.put(mode_no(self.mode), 2);
        b.put(self.seed, 32);
        b.put(self.wpm, 7);
        b.put((self.pitch - PITCH_MIN) / STEP_HZ, 4);
        b.put((self.bandwidth - BW_MIN) / STEP_HZ, 4);
        b.put(self.activity, 4);
        b.put(self.duration, 8);
        for f in [self.qsk, self.qrn, self.qrm, self.qsb, self.flutter, self.lids] {
            b.put(f as u32, 1);
        }
        let own_mix = self.mix != default_mix();
        b.put(own_mix as u32, 1);
        if own_mix {
            for m in self.mix {
                b.put(m as u32, 7);
            }
        }
        // one bit for the usual start with everybody, the hours only when there are some
        b.put((self.head_start > 0) as u32, 1);
        if self.head_start > 0 {
            b.put(self.head_start, HEAD_BITS);
        }
        // the same for the callers' speeds
        let own_speeds = (self.caller_slow, self.caller_fast) != (CALLER_SLOW_DEFAULT, CALLER_FAST_DEFAULT);
        b.put(own_speeds as u32, 1);
        if own_speeds {
            b.put(self.caller_slow, 7);
            b.put(self.caller_fast, 8);
        }
        b.put(self.waves as u32, 1);
        b
    }

    pub fn encode(&self) -> String {
        let mut b = self.payload();
        let sum = b.checksum(SIM_VERSION);
        b.put(sum, 16);
        let mut text = String::new();
        for chunk in b.0.chunks(5) {
            let mut v = chunk.iter().fold(0usize, |v, b| v << 1 | *b as usize);
            v <<= 5 - chunk.len();
            text.push(ALPHABET[v] as char);
        }
        let groups: Vec<&str> = text.as_bytes().chunks(GROUP).map(|g| std::str::from_utf8(g).unwrap_or("")).collect();
        format!("{PREFIX}-{}", groups.join("-"))
    }

    /// Reads a code as it arrives from a mail or a chat: any case, with or without dashes
    /// and spaces, O for 0 and I or L for 1.
    pub fn decode(text: &str) -> Result<Self, CodeError> {
        let clean: String = text.chars().filter(|c| !matches!(c, '-' | ' ' | '\t' | '\n' | '\r')).collect::<String>().to_uppercase();
        let body = clean.strip_prefix(PREFIX).ok_or(CodeError::Format)?;
        let mut bits = Vec::with_capacity(body.len() * 5);
        for c in body.chars() {
            let c = match c {
                'O' => '0',
                'I' | 'L' => '1',
                c => c,
            };
            let v = ALPHABET.iter().position(|a| *a as char == c).ok_or(CodeError::Format)?;
            bits.extend((0..5).rev().map(|i| v >> i & 1 != 0));
        }
        let mut r = Reader { bits: &bits, pos: 0 };
        let version = r.get(6)?;
        // the layout after this depends on the version: name another version as such, not
        // as a typing error or no code at all
        if version != SIM_VERSION && !STILL_READ.contains(&version) {
            return Err(CodeError::Version(version));
        }
        let mode = match r.get(2)? {
            0 => RunMode::Pileup,
            1 => RunMode::Single,
            2 => RunMode::Wpx,
            _ => RunMode::Hst,
        };
        let seed = r.get(32)?;
        let wpm = r.get(7)?;
        let pitch = PITCH_MIN + r.get(4)? * STEP_HZ;
        let bandwidth = BW_MIN + r.get(4)? * STEP_HZ;
        let activity = r.get(4)?;
        let duration = r.get(8)?;
        let (qsk, qrn, qrm) = (r.flag()?, r.flag()?, r.flag()?);
        let (qsb, flutter, lids) = (r.flag()?, r.flag()?, r.flag()?);
        let mut mix = default_mix();
        if r.flag()? {
            for m in &mut mix {
                *m = r.get(7)? as u8;
            }
        }
        let head_start = if version >= 2 && r.flag()? { r.get(HEAD_BITS as usize)? } else { 0 };
        let (caller_slow, caller_fast) = if version >= 3 && r.flag()? { (r.get(7)?, r.get(8)?) } else { (CALLER_SLOW_DEFAULT, CALLER_FAST_DEFAULT) };
        let waves = version >= 4 && r.flag()?;
        let payload = Bits(bits[..r.pos].to_vec());
        let sum = r.get(16)?;
        if bits.len() - r.pos >= 5 || bits[r.pos..].iter().any(|b| *b) {
            return Err(CodeError::Format);
        }
        if payload.checksum(version) != sum {
            return Err(CodeError::Checksum);
        }
        let ok = (WPM_MIN..=WPM_MAX).contains(&wpm)
            && (PITCH_MIN..=PITCH_MAX).contains(&pitch)
            && (BW_MIN..=BW_MAX).contains(&bandwidth)
            && (1..=ACTIVITY_MAX).contains(&activity)
            && (1..=DURATION_MAX).contains(&duration)
            && mix.iter().all(|m| *m <= 100)
            && mix != [0; 6]
            && head_start <= HEAD_START_MAX
            && (CALLER_SLOW_MIN..=100).contains(&caller_slow)
            && (100..=CALLER_FAST_MAX).contains(&caller_fast)
            // competitions start with everybody and keep the callers of the original, a code
            // may not say otherwise
            && (!mode.competition() || (head_start == 0 && (caller_slow, caller_fast) == (CALLER_SLOW_DEFAULT, CALLER_FAST_DEFAULT) && !waves));
        if !ok {
            return Err(CodeError::Range);
        }
        Ok(RunCode { mode, seed, wpm, pitch, bandwidth, activity, duration, qsk, qrn, qrm, qsb, flutter, lids, mix, head_start, caller_slow, caller_fast, waves })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::Rng;

    fn sample(rng: &mut Rng) -> RunCode {
        let flag = |r: &mut Rng| r.chance(0.5);
        let own = rng.chance(0.5);
        let mut c = RunCode {
            mode: *rng.pick(&[RunMode::Pileup, RunMode::Single, RunMode::Wpx, RunMode::Hst]),
            seed: rng.next_u64() as u32,
            wpm: WPM_MIN + rng.below((WPM_MAX - WPM_MIN + 1) as usize) as u32,
            pitch: PITCH_MIN + STEP_HZ * rng.below(13) as u32,
            bandwidth: BW_MIN + STEP_HZ * rng.below(11) as u32,
            activity: 1 + rng.below(9) as u32,
            duration: 1 + rng.below(240) as u32,
            qsk: flag(rng),
            qrn: flag(rng),
            qrm: flag(rng),
            qsb: flag(rng),
            flutter: flag(rng),
            lids: flag(rng),
            mix: if own { [1, 2, 3, 4, 5, 100].map(|m: u8| m - rng.below(1) as u8) } else { default_mix() },
            head_start: if rng.chance(0.5) { 0 } else { rng.below(HEAD_START_MAX as usize + 1) as u32 },
            caller_slow: CALLER_SLOW_DEFAULT,
            caller_fast: CALLER_FAST_DEFAULT,
            waves: rng.chance(0.5),
        };
        if rng.chance(0.5) {
            c.caller_slow = CALLER_SLOW_MIN + rng.below((100 - CALLER_SLOW_MIN + 1) as usize) as u32;
            c.caller_fast = 100 + rng.below((CALLER_FAST_MAX - 100 + 1) as usize) as u32;
        }
        if c.mode.competition() {
            c.head_start = 0;
            (c.caller_slow, c.caller_fast) = (CALLER_SLOW_DEFAULT, CALLER_FAST_DEFAULT);
            c.waves = false;
        }
        c
    }

    #[test]
    fn every_value_comes_back() {
        let mut rng = Rng::new(1);
        for _ in 0..5000 {
            let c = sample(&mut rng);
            assert_eq!(RunCode::decode(&c.encode()), Ok(c));
        }
    }

    #[test]
    fn code_is_short_with_the_usual_mix() {
        let c = RunCode::from_settings(RunMode::Wpx, &Settings::default(), u32::MAX);
        let t = c.encode();
        assert!(t.len() <= 26, "{t}");
        assert!(t.starts_with("MR-"), "{t}");
    }

    #[test]
    fn sloppy_copies_still_read() {
        let c = RunCode::from_settings(RunMode::Pileup, &Settings::default(), 12345);
        let t = c.encode();
        let loose = format!("  {} \n", t.to_lowercase().replace('-', " "));
        assert_eq!(RunCode::decode(&loose).as_ref(), Ok(&c));
        assert_eq!(RunCode::decode(&t.replace('-', "")).as_ref(), Ok(&c));
        assert_eq!(RunCode::decode(&t.replace('0', "O").replace('1', "l")).as_ref(), Ok(&c));
    }

    #[test]
    fn every_typing_error_is_caught() {
        let c = RunCode::from_settings(RunMode::Wpx, &Settings::default(), 987654321);
        let t = c.encode();
        for (i, old) in t.char_indices().skip(3).filter(|(_, c)| *c != '-') {
            for new in ALPHABET.iter().map(|b| *b as char).filter(|n| *n != old) {
                let mut bad = t.clone();
                bad.replace_range(i..i + 1, &new.to_string());
                // a change in the version field reads as another version, also refused
                assert!(RunCode::decode(&bad).is_err(), "{bad}");
            }
        }
    }

    #[test]
    fn swapped_neighbours_are_caught() {
        let c = RunCode::from_settings(RunMode::Hst, &Settings::default(), 555);
        let t: Vec<char> = c.encode().replace('-', "").chars().collect();
        for i in 2..t.len() - 1 {
            if t[i] != t[i + 1] {
                let mut bad = t.clone();
                bad.swap(i, i + 1);
                assert!(RunCode::decode(&bad.iter().collect::<String>()).is_err());
            }
        }
    }

    #[test]
    fn nonsense_is_refused() {
        for t in ["", "MR", "hello", "MR-", "MR-ABCD", "XX-0000-0000-0000-0000", "MR-UUUU-UUUU"] {
            assert!(RunCode::decode(t).is_err(), "{t}");
        }
    }

    #[test]
    fn cut_off_code_is_refused() {
        let t = RunCode::from_settings(RunMode::Pileup, &Settings::default(), 7).encode();
        assert!(RunCode::decode(&t[..t.len() - 3]).is_err());
    }

    #[test]
    fn other_version_is_named() {
        let c = RunCode::from_settings(RunMode::Pileup, &Settings::default(), 7);
        let t = c.encode().replace('-', "");
        // the version sits in the first six bits, right behind the prefix
        let v = ALPHABET.iter().position(|a| *a as char == t.as_bytes()[2] as char).unwrap();
        let other = ALPHABET[v ^ 0b00100] as char;
        let mut bad = t.clone();
        bad.replace_range(2..3, &other.to_string());
        assert!(matches!(RunCode::decode(&bad), Err(CodeError::Version(_))));
    }

    #[test]
    fn codes_of_version_1_still_read() {
        // made by 0.1.1, before the head start
        let c = RunCode::decode("MR-0G38-9ERV-7K85-W7WP-G3").unwrap();
        assert_eq!((c.mode, c.wpm, c.head_start), (RunMode::Pileup, 30, 0));
        // the first character holds the top five of the six version bits
        let first = ALPHABET[(SIM_VERSION >> 1) as usize] as char;
        assert!(c.encode().starts_with(&format!("MR-{first}")), "comes back as a code of this version");
    }

    #[test]
    fn a_head_start_costs_one_bit_when_there_is_none() {
        let s = Settings::default();
        let none = RunCode::from_settings(RunMode::Pileup, &s, 9).encode();
        let some = RunCode::from_settings(RunMode::Pileup, &Settings { head_start: 48, ..s }, 9).encode();
        assert!(none.len() <= 26 && some.len() <= 28, "{none} {some}");
    }

    #[test]
    fn codes_of_version_2_still_read() {
        // made before the callers' speeds, with a head start of 6 h
        let c = RunCode::decode("MR-1038-9ERV-7K85-W7X3-8VX8").unwrap();
        assert_eq!((c.head_start, c.caller_slow, c.caller_fast), (6, CALLER_SLOW_DEFAULT, CALLER_FAST_DEFAULT));
    }

    /// Made by 0.1.2 with callers at 90 to 110 % and a head start of 2 h.
    const V3_CODE: &str = "MR-1HZA-Y5QN-7K85-W7X1-DMVP-XK0";

    #[test]
    fn codes_of_version_3_still_read() {
        // made before the tide, with callers at 90 to 110 %
        let s = Settings { caller_slow: 90, caller_fast: 110, ..Settings::default() };
        let c = RunCode::decode(V3_CODE).unwrap();
        assert_eq!((c.caller_slow, c.caller_fast, c.waves, c.head_start), (s.caller_slow, s.caller_fast, false, 2));
    }

    #[test]
    fn a_competition_code_with_the_tide_is_refused() {
        let mut c = RunCode::from_settings(RunMode::Pileup, &Settings { waves: true, ..Settings::default() }, 5);
        assert_eq!(RunCode::decode(&c.encode()), Ok(c.clone()));
        c.mode = RunMode::Wpx;
        assert_eq!(RunCode::decode(&c.encode()), Err(CodeError::Range));
    }

    #[test]
    fn a_competition_code_with_own_caller_speeds_is_refused() {
        let s = Settings { caller_slow: 90, caller_fast: 110, ..Settings::default() };
        let mut c = RunCode::from_settings(RunMode::Pileup, &s, 5);
        assert_eq!(RunCode::decode(&c.encode()), Ok(c.clone()));
        c.mode = RunMode::Hst;
        assert_eq!(RunCode::decode(&c.encode()), Err(CodeError::Range));
    }

    #[test]
    fn a_competition_code_with_a_head_start_is_refused() {
        let mut c = RunCode::from_settings(RunMode::Pileup, &Settings { head_start: 6, ..Settings::default() }, 5);
        assert_eq!(RunCode::decode(&c.encode()), Ok(c.clone()));
        c.mode = RunMode::Wpx;
        assert_eq!(RunCode::decode(&c.encode()), Err(CodeError::Range));
    }

    #[test]
    fn a_head_start_out_of_range_is_capped_not_wrapped() {
        let c = RunCode::from_settings(RunMode::Pileup, &Settings { head_start: 64, ..Settings::default() }, 5);
        assert_eq!(RunCode::decode(&c.encode()).map(|c| c.head_start), Ok(HEAD_START_MAX));
    }

    #[test]
    fn no_regions_at_all_still_gives_callers() {
        let s = Settings { region_mix: [0.0; 6], ..Settings::default() };
        let c = RunCode::from_settings(RunMode::Pileup, &s, 1);
        assert_eq!(RunCode::decode(&c.encode()), Ok(c));
    }
}
