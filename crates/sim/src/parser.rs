//! What the operator's over says. It has to cope with what a tired fist produces:
//! words run together, stray characters, cut numbers.

/// A report in plain or cut numbers, as digits; `None` if the token is not one.
pub fn rst(token: &str) -> Option<String> {
    let b = token.as_bytes();
    let tail = |c: u8| match c {
        b'N' => Some('9'),
        b'1'..=b'9' => Some(c as char),
        _ => None,
    };
    match b {
        [a @ b'1'..=b'5', b @ b'1'..=b'9'] => Some(format!("{}{}", *a as char, *b as char)),
        [a @ (b'1'..=b'5' | b'E'), b, c] => {
            let a = if *a == b'E' { '5' } else { *a as char };
            let s = format!("{a}{}{}", tail(*b)?, tail(*c)?);
            Some(s)
        }
        _ => None,
    }
}

/// Letters and digits with a digit inside: W1ABC, DL8UG/P, 9A2XY, 5Z4AB.
pub fn looks_like_call(token: &str) -> bool {
    let base = token.split('/').max_by_key(|p| p.len()).unwrap_or("");
    let b = base.as_bytes();
    b.len() >= 3
        && b.len() <= 10
        && b.iter().all(u8::is_ascii_alphanumeric)
        && b.iter().any(u8::is_ascii_alphabetic)
        && b.last().is_some_and(u8::is_ascii_alphabetic)
        && b[..b.len() - 1].iter().any(u8::is_ascii_digit)
        && rst(token).is_none()
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Over {
    pub text: String,
    pub cq: bool,
    pub qrz: bool,
    pub qrl: bool,
    pub own_call: bool,
    /// Whole calls other than the own one.
    pub calls: Vec<String>,
    /// Fragments asked for: `W1?` or `?BC`.
    pub partials: Vec<String>,
    pub rst: Option<String>,
    /// Everything that reads as a number, reports included, in the order sent.
    pub numbers: Vec<String>,
    /// B4: worked before.
    pub before: bool,
    /// AGN, a lone ?, or a named question below.
    pub repeat: bool,
    pub ask_rst: bool,
    pub ask_qth: bool,
    pub ask_name: bool,
    pub ask_call: bool,
    pub closing: bool,
    /// Tokens nothing could be made of.
    pub garbled: usize,
    pub words: usize,
}

const FILLER: &[&str] = &[
    "DE", "K", "BK", "<BK>", "<KN>", "KN", "<AR>", "AR", "<SK>", "SK", "<BT>", "=", "R", "RR", "UR", "RST", "GM", "GA",
    "GE", "GN", "ES", "ALSO", "FB", "OM", "HI", "HW", "PSE", "NAME", "OP", "QTH", "POTA", "SOTA", "TNX", "NW", "TEST",
    "NR", "QSO",
];

/// A serial number in digits, cut numbers included. A letter alone or a word stays out.
pub fn number(token: &str) -> Option<String> {
    let cut = |c: char| match c {
        'T' | 'O' => Some('0'),
        'A' => Some('1'),
        'E' => Some('5'),
        'N' => Some('9'),
        '0'..='9' => Some(c),
        _ => None,
    };
    let has_digit = token.chars().any(|c| c.is_ascii_digit());
    let only_tn = token.len() >= 2 && token.chars().all(|c| c == 'T' || c == 'N');
    if token.len() > 5 || !(has_digit || only_tn) {
        return None;
    }
    token.chars().map(cut).collect()
}

/// `5NN001`: report and number without a gap.
fn report_and_number(token: &str) -> Option<String> {
    ["5NN", "599", "ENN"].iter().find_map(|r| {
        let rest = token.strip_prefix(r)?;
        (!rest.is_empty()).then(|| number(rest)).flatten()
    })
}

/// Words that may come in pieces, `T U` or `5 N N`, the way a person still reads them.
const SPLIT_WORDS: [&str; 22] = [
    "CQ", "DE", "UR", "TU", "73", "72", "BK", "KN", "RST", "RST?", "AGN", "AGN?", "QRZ", "QRZ?", "QRL", "QRL?", "POTA",
    "TNX", "NAME?", "TEST", "NR", "NR?",
];
const CLOSING: [&str; 13] = ["TU", "73", "72", "EE", "E", "GL", "TKS", "THX", "SK", "<SK>", "CUL", "GD", "GB"];

fn is_word(t: &str) -> bool {
    t == "CQ"
        || t == "?"
        || FILLER.contains(&t)
        || CLOSING.contains(&t)
        || SPLIT_WORDS.contains(&t)
        || rst(t).is_some()
}

/// Puts back together what a pause in the wrong place took apart: first the known words,
/// then runs of pieces that make a call or a question like `W 1?`.
fn rejoin(tokens: Vec<String>) -> Vec<String> {
    let mut words = Vec::with_capacity(tokens.len());
    let mut i = 0;
    while i < tokens.len() {
        let whole = (2..=4.min(tokens.len() - i)).rev().find_map(|n| {
            let joined = tokens[i..i + n].concat();
            (SPLIT_WORDS.contains(&joined.as_str()) || rst(&joined).is_some()).then_some((joined, n))
        });
        match whole {
            Some((joined, n)) => {
                words.push(joined);
                i += n;
            }
            None => {
                words.push(tokens[i].clone());
                i += 1;
            }
        }
    }

    let mut out: Vec<String> = Vec::with_capacity(words.len());
    let mut i = 0;
    while i < words.len() {
        let mut joined = words[i].clone();
        let mut j = i + 1;
        if !is_word(&joined) {
            while j < words.len() && !is_word(&words[j]) && !joined.ends_with('?') {
                // a whole call next to it is a call of its own
                if looks_like_call(&words[j]) && words[j].len() >= 4 {
                    break;
                }
                joined.push_str(&words[j]);
                j += 1;
            }
        }
        if j > i + 1 && (looks_like_call(&joined) || joined.ends_with('?') || number(&joined).is_some()) {
            out.push(joined);
            i = j;
        } else {
            out.push(words[i].clone());
            i += 1;
        }
    }
    out
}

/// Parses a finished over. Calls are upper case, `own` without any portable suffix.
pub fn parse(text: &str, own: &str) -> Over {
    let mut o = Over { text: text.trim().to_string(), ..Over::default() };
    let tokens: Vec<&str> = o.text.split_whitespace().map(|s| s.trim_matches('*')).filter(|s| !s.is_empty()).collect();
    let tokens = rejoin(tokens.iter().map(|s| s.to_string()).collect());
    o.words = tokens.len();
    let is_own = |t: &str| t.split('/').any(|p| p == own);

    for t in &tokens {
        let t = t.as_str();
        if let Some(n) = report_and_number(t) {
            o.rst.get_or_insert("599".to_string());
            o.numbers.extend(["599".to_string(), n]);
            continue;
        }
        if !CLOSING.contains(&t) && !is_own(t) && !looks_like_call(t)
            && let Some(n) = number(t) {
                o.numbers.push(n);
                if rst(t).is_none() {
                    continue;
                }
            }
        match t {
            "B4" => o.before = true,
            "CQ" => o.cq = true,
            "QRZ" | "QRZ?" => o.qrz = true,
            "QRL" | "QRL?" => o.qrl = true,
            "?" | "AGN" | "AGN?" | "PSE?" => o.repeat = true,
            "RST?" | "NR?" | "SIG?" | "UR?" | "RPT?" => o.ask_rst = true,
            "QTH?" | "ST?" | "STATE?" | "QTH" if t.ends_with('?') => o.ask_qth = true,
            "NAME?" | "OP?" => o.ask_name = true,
            "CALL?" | "CL?" => o.ask_call = true,
            _ if CLOSING.contains(&t) => o.closing = true,
            _ if rst(t).is_some() => {
                o.rst.get_or_insert(rst(t).unwrap());
            }
            _ if is_own(t) => o.own_call = true,
            _ if looks_like_call(t) => {
                o.calls.push(t.to_string());
                if t.len() >= 8 && !t.contains('/') {
                    run_together(t, own, &mut o);
                }
            }
            _ if t.ends_with('?') && t.len() > 1 => o.partials.push(t.trim_end_matches('?').to_string()),
            _ if t.starts_with('?') && t.len() > 1 => o.partials.push(t.trim_start_matches('?').to_string()),
            _ if FILLER.contains(&t) => {}
            _ => {
                // words run together: look inside before giving up
                let inner = run_together(t, own, &mut o);
                if !inner {
                    o.garbled += 1;
                }
            }
        }
    }
    o.repeat |= o.ask_rst || o.ask_qth || o.ask_name || o.ask_call;
    // the own call counts in whatever pieces it came
    o.own_call |= !own.is_empty() && o.text.replace(' ', "").contains(own);
    o
}

fn run_together(t: &str, own: &str, o: &mut Over) -> bool {
    let mut found = false;
    for r in ["5NN", "599", "579", "559", "449", "ENN", "55N", "57N"] {
        if t.contains(r) {
            o.rst.get_or_insert(rst(r).unwrap());
            found = true;
        }
    }
    if t.contains("TU") || t.contains("73") || t.ends_with("EE") {
        o.closing = true;
        found = true;
    }
    if t.contains(own) {
        o.own_call = true;
        found = true;
    }
    if t.starts_with("CQ") {
        o.cq = true;
        found = true;
    }
    found
}

impl Over {
    /// The serial number of a contest exchange: what follows the report, or a lone number.
    pub fn serial(&self) -> Option<&str> {
        match self.numbers.as_slice() {
            [] => None,
            [only] if self.rst.as_deref() == Some(only) && only.ends_with('9') && only.len() <= 3 && only.starts_with('5') => None,
            all => all.last().map(String::as_str),
        }
    }

    /// Does the over name this call, also when run into other words?
    pub fn names(&self, call: &str) -> bool {
        self.calls.iter().any(|c| c == call) || self.text.replace(' ', "").contains(call)
    }

    /// The nearest call in the over and its distance.
    pub fn nearest(&self, call: &str) -> Option<usize> {
        self.calls.iter().map(|c| crate::morse::levenshtein(c, call)).min()
    }

    pub fn fragment_of(&self, call: &str) -> bool {
        self.partials.iter().any(|p| !p.is_empty() && call.contains(p.as_str()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ME: &str = "DL8UG";

    #[test]
    fn reports() {
        for (t, want) in [("5NN", "599"), ("599", "599"), ("ENN", "599"), ("55N", "559"), ("449", "449"), ("559", "559"), ("59", "59")] {
            assert_eq!(rst(t).as_deref(), Some(want), "{t}");
        }
        for t in ["73", "72", "TU", "EE", "E", "5", "NNN", "W1A"] {
            assert_eq!(rst(t), None, "{t}");
        }
    }

    #[test]
    fn calls() {
        for t in ["W1ABC", "DL8UG/P", "9A2XY", "5Z4AB", "HB9CVQ", "M0AAA", "G4XYZ", "W1ABC/QRP"] {
            assert!(looks_like_call(t), "{t}");
        }
        for t in ["5NN", "POTA", "73", "US-1234", "E", "599", "QRZ"] {
            assert!(!looks_like_call(t), "{t}");
        }
    }

    #[test]
    fn cq() {
        let o = parse("CQ POTA CQ POTA DE DL8UG DL8UG/P K", ME);
        assert!(o.cq && o.own_call && o.calls.is_empty() && o.garbled == 0);
    }

    #[test]
    fn report_over() {
        let o = parse("W1ABC GM UR 5NN 5NN <BK>", ME);
        assert_eq!(o.calls, vec!["W1ABC"]);
        assert_eq!(o.rst.as_deref(), Some("599"));
        assert!(!o.closing && !o.repeat);
    }

    #[test]
    fn partial_and_questions() {
        assert_eq!(parse("W1?", ME).partials, vec!["W1"]);
        assert_eq!(parse("?BC", ME).partials, vec!["BC"]);
        assert!(parse("?", ME).repeat);
        assert!(parse("AGN?", ME).repeat);
        let o = parse("RST?", ME);
        assert!(o.ask_rst && o.repeat);
        assert!(parse("QTH?", ME).ask_qth);
    }

    #[test]
    fn closings() {
        assert!(parse("TU 73 E E", ME).closing);
        assert!(parse("<BK> TU 73 DE DL8UG K", ME).closing);
        assert!(parse("R TU EE", ME).closing);
    }

    #[test]
    fn run_together_words() {
        let o = parse("W1ABCUR5NN", ME);
        assert!(o.names("W1ABC"));
        assert_eq!(o.rst.as_deref(), Some("599"));
        assert!(parse("TU73", ME).closing);
    }

    #[test]
    fn own_call_in_pieces() {
        for t in ["CQ POTA DE DL 8 U G K", "CQ DE DL8 UG K", "CQ DE D L 8 U G DL 8UG K", "C Q P O T A D E DL 8 U G K"] {
            let o = parse(t, ME);
            assert!(o.cq && o.own_call, "{t}: {o:?}");
            assert!(o.calls.is_empty() || o.calls.iter().all(|c| c.contains("8")), "{t}: {o:?}");
        }
    }

    #[test]
    fn words_in_pieces() {
        let o = parse("W 1 A B C U R 5 N N 5 NN K", ME);
        assert!(o.names("W1ABC"));
        assert_eq!(o.calls, vec!["W1ABC"]);
        assert_eq!(o.rst.as_deref(), Some("599"));
        assert_eq!(o.garbled, 0);
        assert!(parse("T U 7 3 E E", ME).closing);
        assert!(parse("A G N ?", ME).repeat);
        assert!(parse("R S T ?", ME).ask_rst);
        assert!(parse("QRZ ?", ME).qrz);
    }

    #[test]
    fn wrong_call_in_pieces_is_still_close() {
        let o = parse("W 1 A B D 5NN", ME);
        assert_eq!(o.nearest("W1ABC"), Some(1));
    }

    #[test]
    fn partial_in_pieces() {
        assert_eq!(parse("W 1?", ME).partials, vec!["W1"]);
    }

    #[test]
    fn whole_calls_stay_apart() {
        assert_eq!(parse("W1ABC W1ABC", ME).calls, vec!["W1ABC", "W1ABC"]);
        assert_eq!(parse("N2AV W5TFQ", ME).calls, vec!["N2AV", "W5TFQ"]);
        assert_eq!(parse("W1ABC UR 5NN", ME).calls, vec!["W1ABC"]);
    }

    #[test]
    fn contest_numbers() {
        for (t, want) in [
            ("W1ABC 5NN 001", Some("001")),
            ("5NN TT1", Some("001")),
            ("5NN 1T", Some("10")),
            ("5NN001", Some("001")),
            ("599 123", Some("123")),
            ("5NN 0 0 7", Some("007")),
            ("NR 42", Some("42")),
            ("123", Some("123")),
            ("5NN", None),
            ("TU", None),
            ("TU DL8UG", None),
        ] {
            assert_eq!(parse(t, ME).serial(), want, "{t}");
        }
        assert!(parse("QSO B4", ME).before);
    }

    #[test]
    fn garbage_counts() {
        assert_eq!(parse("XQZ *", ME).garbled, 1);
    }

    #[test]
    fn nearest_call() {
        let o = parse("W1ABD 5NN", ME);
        assert_eq!(o.nearest("W1ABC"), Some(1));
        assert!(!o.names("W1ABC"));
    }
}
