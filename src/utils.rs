use aho_corasick::AhoCorasick;
use arrayvec::ArrayVec;
use debug_unreachable::debug_unreachable;
use lazy_static::lazy_static;
use std::borrow::Cow;
use std::fmt::{Error, Formatter, Write};
use std::ops::Range;

pub(crate) const HIGH_SURROGATES: Range<u16> = 0xd800..0xdc00;
pub(crate) const LOW_SURROGATES: Range<u16> = 0xdc00..0xe000;

pub(crate) fn escape(input: &str) -> Cow<str> {
    lazy_static! {
        static ref PATTERNS: &'static [&'static str] = &[
            "\"", "\\", "\x00", "\x01", "\x02", "\x03", "\x04", "\x05", "\x06", "\x07", "\x08",
            "\x09", "\x0a", "\x0b", "\x0c", "\x0d", "\x0e", "\x0f", "\x10", "\x11", "\x12", "\x13",
            "\x14", "\x15", "\x16", "\x17", "\x18", "\x19", "\x1a", "\x1b", "\x1c", "\x1d", "\x1e",
            "\x1f"
        ];
        static ref REPLACEMENTS: &'static [&'static str] = &[
            r#"\""#,
            r#"\\"#,
            r#"\u0000"#,
            r#"\u0001"#,
            r#"\u0002"#,
            r#"\u0003"#,
            r#"\u0004"#,
            r#"\u0005"#,
            r#"\u0006"#,
            r#"\u0007"#,
            r#"\b"#,
            r#"\t"#,
            r#"\n"#,
            r#"\u000b"#,
            r#"\f"#,
            r#"\r"#,
            r#"\u000e"#,
            r#"\u000f"#,
            r#"\u0010"#,
            r#"\u0011"#,
            r#"\u0012"#,
            r#"\u0013"#,
            r#"\u0014"#,
            r#"\u0015"#,
            r#"\u0016"#,
            r#"\u0017"#,
            r#"\u0018"#,
            r#"\u0019"#,
            r#"\u001a"#,
            r#"\u001b"#,
            r#"\u001c"#,
            r#"\u001d"#,
            r#"\u001e"#,
            r#"\u001f"#
        ];
        static ref AC: AhoCorasick = AhoCorasick::new_auto_configured(&PATTERNS);
    }

    let mut res = Cow::default();
    let mut last_start = 0usize;
    for mat in AC.find_iter(input) {
        res += &input[last_start..mat.start()];
        last_start = mat.end();
        res += REPLACEMENTS[mat.pattern()];
    }
    res += &input[last_start..];
    res
}

pub(crate) fn unescape(input: &str) -> Cow<str> {
    lazy_static! {
        static ref PATTERNS: &'static [&'static str] =
            &[r#"\""#, r"\\", r"\/", r"\b", r"\f", r"\n", r"\r", r"\t", r"\u"];
        static ref REPLACEMENTS: &'static [&'static str] =
            &["\"", "\\", "/", "\x08", "\x0c", "\x0a", "\x0d", "\x09"];
        static ref AC: AhoCorasick = AhoCorasick::new_auto_configured(&PATTERNS);
    }

    let mut res = Cow::default();
    let mut last_start = 0usize;
    let mut vec: ArrayVec<[u16; 2]> = ArrayVec::new();
    for mat in AC.find_iter(input) {
        res += &input[last_start..mat.start()];
        last_start = mat.end();

        let pat_idx = mat.pattern();
        if pat_idx < REPLACEMENTS.len() {
            res += Cow::from(REPLACEMENTS[pat_idx]);
        } else if PATTERNS[pat_idx] == r"\u" {
            last_start += 4;
            let hex_digits = &input[mat.end()..mat.end() + 4];
            let cp = u16::from_str_radix(hex_digits, 16).unwrap();
            vec.push(cp);

            if !HIGH_SURROGATES.contains(&cp) {
                res += Cow::from(String::from_utf16(vec.as_ref()).unwrap());
                vec.clear();
            }
        } else {
            unsafe {
                debug_unreachable!();
            }
        }
    }
    res += &input[last_start..];
    res
}

pub(crate) struct PadAdapter<'a, 'b: 'a> {
    fmt: &'a mut Formatter<'b>,
    on_newline: bool,
}

impl<'a, 'b: 'a> From<&'a mut Formatter<'b>> for PadAdapter<'a, 'b> {
    fn from(fmt: &'a mut Formatter<'b>) -> Self {
        PadAdapter {
            fmt,
            on_newline: false,
        }
    }
}

impl<'a, 'b: 'a> Write for PadAdapter<'a, 'b> {
    fn write_str(&mut self, mut s: &str) -> Result<(), Error> {
        while !s.is_empty() {
            if self.on_newline {
                self.fmt.write_str("    ")?;
            }
            let split = match s.find('\n') {
                Some(pos) => {
                    self.on_newline = true;
                    pos + 1
                }
                None => {
                    self.on_newline = false;
                    s.len()
                }
            };
            self.fmt.write_str(&s[..split])?;
            s = &s[split..];
        }
        Ok(())
    }
}

impl<'a, 'b: 'a> PadAdapter<'a, 'b> {
    pub fn into_inner(self) -> &'a mut Formatter<'b> {
        self.fmt
    }
}

pub(crate) fn f64_to_i64(f: f64) -> Option<i64> {
    let truncated = f.trunc() as i64;
    if f - truncated as f64 == 0.0 {
        Some(truncated)
    } else {
        None
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use itertools::Itertools;
    use proptest::prelude::*;
    use std::panic::catch_unwind;

    #[test]
    fn placeholder() {}

    proptest! {
        #[test]
        fn test_unescape_random(s in r#"[^\pC\\]*"#) {
            let res = unescape(&s);
            prop_assert_eq!(&res, &s);
        }

        #[test]
        fn test_unescape_random_utf16(s in r#"[^\pC\\]*"#) {
            let encoded = format!("{}", s.encode_utf16().format_with("", |cp: u16, f| f(&format_args!("\\u{:04X}", cp))));
            if let Ok(res) = catch_unwind(|| {
                unescape(&encoded)
            }) {
                prop_assert_eq!(&res, &s);
            } else {
                prop_assert!(false);
            }
        }
    }
}
