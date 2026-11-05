//! Morse table. Text uses plain characters; a prosign keyed as one character is
//! written in angle brackets, e.g. `<BK>`.

const LETTERS: [&str; 26] = [
    ".-", "-...", "-.-.", "-..", ".", "..-.", "--.", "....", "..", ".---", "-.-", ".-..", "--",
    "-.", "---", ".--.", "--.-", ".-.", "...", "-", "..-", "...-", ".--", "-..-", "-.--", "--..",
];
const DIGITS: [&str; 10] = [
    "-----", ".----", "..---", "...--", "....-", ".....", "-....", "--...", "---..", "----.",
];
const OTHER: [(char, &str); 5] = [
    ('/', "-..-."),
    ('?', "..--.."),
    ('.', ".-.-.-"),
    (',', "--..--"),
    ('=', "-...-"),
];
const PROSIGNS: [(&str, &str); 6] = [
    ("BK", "-...-.-"),
    ("KN", "-.--."),
    ("AR", ".-.-."),
    ("SK", "...-.-"),
    ("BT", "-...-"),
    ("HH", "........"),
];

pub fn of_char(c: char) -> Option<&'static str> {
    match c {
        'A'..='Z' => Some(LETTERS[c as usize - 'A' as usize]),
        '0'..='9' => Some(DIGITS[c as usize - '0' as usize]),
        _ => OTHER.iter().find(|(o, _)| *o == c).map(|(_, m)| *m),
    }
}

pub fn of_prosign(name: &str) -> Option<&'static str> {
    PROSIGNS.iter().find(|(n, _)| *n == name).map(|(_, m)| *m)
}

/// Text for a keyed pattern: a character, or `<XX>` for a prosign.
pub fn decode(pattern: &str) -> Option<String> {
    if let Some(c) = ('A'..='Z')
        .chain('0'..='9')
        .chain(OTHER.iter().map(|(o, _)| *o).filter(|c| *c != '='))
        .find(|c| of_char(*c) == Some(pattern))
    {
        return Some(c.to_string());
    }
    PROSIGNS
        .iter()
        .find(|(_, m)| *m == pattern)
        .map(|(n, _)| format!("<{n}>"))
}

/// The element patterns of a text, one entry per keyed character, `None` for a word gap.
pub fn patterns(text: &str) -> Vec<Option<&'static str>> {
    let mut out = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c.is_whitespace() {
            if out.last().is_some_and(|l: &Option<&str>| l.is_some()) {
                out.push(None);
            }
        } else if c == '<' {
            let name: String = chars.by_ref().take_while(|c| *c != '>').collect();
            if let Some(m) = of_prosign(&name) {
                out.push(Some(m));
            }
        } else if let Some(m) = of_char(c.to_ascii_uppercase()) {
            out.push(Some(m));
        }
    }
    while matches!(out.last(), Some(None)) {
        out.pop();
    }
    out
}

/// Text as a key sequence: `.` and `-` take their length plus one unit, a space two more
/// units, `~` closes the message. Characters without a code are dropped.
pub fn encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len() * 6);
    for c in text.chars() {
        match c {
            ' ' | '_' => out.push(' '),
            _ => {
                if let Some(m) = of_char(c) {
                    out.push_str(m);
                    out.push(' ');
                }
            }
        }
    }
    if !out.is_empty() {
        out.pop();
        out.push('~');
    }
    out
}

const RISE_S: f64 = 0.005;

/// Rise of a Blackman-Harris window, `len` samples from 0 to 1.
fn ramp(len: usize) -> Vec<f32> {
    use std::f64::consts::PI;
    let kernel = |x: f64| 0.35875 - 0.48829 * (2.0 * PI * x).cos() + 0.14128 * (4.0 * PI * x).cos() - 0.01168 * (6.0 * PI * x).cos();
    let mut sum = 0.0;
    let mut r: Vec<f64> = (0..len)
        .map(|i| {
            sum += kernel(i as f64 / len as f64);
            sum
        })
        .collect();
    r.iter_mut().for_each(|v| *v /= sum);
    r.into_iter().map(|v| v as f32).collect()
}

/// The amplitude of a key sequence from `encode`, 0 to 1 at `rate`, padded with silence
/// to a whole number of blocks. Above 80 wpm the edges shrink to fit a dit.
pub fn envelope(morse: &str, wpm: u32, rate: u32, block: usize) -> Vec<f32> {
    let unit = (0.1 * rate as f64 * 12.0 / wpm.max(1) as f64).round() as usize;
    let ramp_len = ((2.7 * RISE_S * rate as f64).round() as usize).min(unit);
    let on = ramp(ramp_len);
    let units: usize = morse
        .chars()
        .map(|c| match c {
            '.' | ' ' => 2,
            '-' => 4,
            '~' => 1,
            _ => 0,
        })
        .sum();
    if units == 0 {
        return Vec::new();
    }
    let true_len = units * unit + ramp_len;
    let mut out = vec![0f32; true_len.div_ceil(block) * block];
    let mut p = 0;
    let element = |out: &mut [f32], p: &mut usize, dur: usize| {
        out[*p..*p + ramp_len].copy_from_slice(&on);
        *p += ramp_len;
        let flat = dur * unit - ramp_len;
        out[*p..*p + flat].fill(1.0);
        *p += flat;
        for i in 0..ramp_len {
            out[*p + i] = on[ramp_len - 1 - i];
        }
        *p += ramp_len + unit - ramp_len;
    };
    for c in morse.chars() {
        match c {
            '.' => element(&mut out, &mut p, 1),
            '-' => element(&mut out, &mut p, 3),
            ' ' => p += 2 * unit - ramp_len,
            '~' => p += unit - ramp_len,
            _ => {}
        }
    }
    out
}

/// Edit distance between two strings.
pub fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0; b.len() + 1];
    for i in 1..=a.len() {
        cur[0] = i;
        for j in 1..=b.len() {
            cur[j] = (prev[j - 1] + usize::from(a[i - 1] != b[j - 1]))
                .min(prev[j] + 1)
                .min(cur[j - 1] + 1);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_inverts_table() {
        for c in ('A'..='Z').chain('0'..='9') {
            assert_eq!(decode(of_char(c).unwrap()).unwrap(), c.to_string());
        }
        assert_eq!(decode("-...-.-").unwrap(), "<BK>");
        assert_eq!(decode("-...-").unwrap(), "<BT>");
        assert_eq!(decode("........").unwrap(), "<HH>");
        assert_eq!(decode("------"), None);
    }

    #[test]
    fn patterns_keep_word_gaps_and_prosigns() {
        let p = patterns(" K <BK>  E ");
        assert_eq!(p, vec![Some("-.-"), None, Some("-...-.-"), None, Some(".")]);
    }

    #[test]
    fn encode_marks_the_end() {
        assert_eq!(encode("TU"), "- ..-~");
        assert_eq!(encode("E E"), ".  .~");
        assert_eq!(encode(""), "");
        assert_eq!(encode(" "), "~");
    }

    #[test]
    fn envelope_timing() {
        // 12 wpm at 11025: a unit is 1103 samples
        let e = envelope(&encode("E"), 12, 11025, 256);
        assert_eq!(e.len() % 256, 0);
        let high = e.iter().filter(|v| **v > 0.5).count() as i64;
        assert!((high - 1103).abs() < 10, "{high}");
        assert!(e.iter().all(|v| (0.0..=1.0).contains(v)));
        assert_eq!(e[1103 + 200], 0.0);
    }

    #[test]
    fn dah_is_three_dits() {
        let count = |t: &str| envelope(&encode(t), 25, 11025, 256).iter().filter(|v| **v > 0.5).count() as f64;
        let ratio = count("T") / count("E");
        assert!((ratio - 3.0).abs() < 0.05, "{ratio}");
    }

    #[test]
    fn fast_speeds_still_key() {
        for wpm in [80, 81, 120] {
            let e = envelope(&encode("PARIS"), wpm, 11025, 256);
            assert!(e.iter().filter(|v| **v > 0.5).count() > 100);
            assert!(e.contains(&0.0));
        }
    }

    #[test]
    fn edit_distance() {
        assert_eq!(levenshtein("W1ABC", "W1ABC"), 0);
        assert_eq!(levenshtein("W1ABC", "W1AB"), 1);
        assert_eq!(levenshtein("W1ABC", "K1ABD"), 2);
    }
}
