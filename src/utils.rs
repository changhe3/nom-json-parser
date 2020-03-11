use aho_corasick::AhoCorasick;
use lazy_static::lazy_static;
use regex::{Captures, Regex, Replacer};
use std::borrow::Cow;
use std::fmt::{Debug, Display, Error, Formatter, Write};
use std::ops::Range;

pub(crate) const HIGH_SURROGATES: Range<u16> = 0xd800..0xdc00;
pub(crate) const LOW_SURROGATES: Range<u16> = 0xdc00..0xe000;

pub(crate) trait RegexExt {
    fn cow_replace<'t, R: Replacer>(&self, text: Cow<'t, str>, rep: R) -> Cow<'t, str>;
    fn cow_replace_all<'t, R: Replacer>(&self, text: Cow<'t, str>, rep: R) -> Cow<'t, str>;
    fn cow_replacen<'t, R: Replacer>(
        &self,
        text: Cow<'t, str>,
        limit: usize,
        rep: R,
    ) -> Cow<'t, str>;
}

impl RegexExt for Regex {
    fn cow_replace<'t, R: Replacer>(&self, text: Cow<'t, str>, rep: R) -> Cow<'t, str> {
        match text {
            Cow::Borrowed(borrowed) => self.replace(borrowed, rep),
            Cow::Owned(owned) => self.replace(&owned, rep).into_owned().into(),
        }
    }

    fn cow_replace_all<'t, R: Replacer>(&self, text: Cow<'t, str>, rep: R) -> Cow<'t, str> {
        match text {
            Cow::Borrowed(borrowed) => self.replace_all(borrowed, rep),
            Cow::Owned(owned) => self.replace_all(&owned, rep).into_owned().into(),
        }
    }

    fn cow_replacen<'t, R: Replacer>(
        &self,
        text: Cow<'t, str>,
        limit: usize,
        rep: R,
    ) -> Cow<'t, str> {
        match text {
            Cow::Borrowed(borrowed) => self.replacen(borrowed, limit, rep),
            Cow::Owned(owned) => self.replacen(&owned, limit, rep).into_owned().into(),
        }
    }
}

pub(crate) struct DisplayAsDeBug<T>(pub T);

impl<T: Display> Debug for DisplayAsDeBug<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        self.0.fmt(f)?;
        Ok(())
    }
}

pub(crate) fn escape(string: &str) -> Cow<str> {
    lazy_static! {
        static ref RE_ESCAPE: Regex = Regex::new(r#"[\\"\u0000-\u001f]"#).unwrap();
    }
    let string = RE_ESCAPE.replace_all(string, |caps: &Captures| -> Cow<str> {
        match &caps[0] {
            "\\" => r#"\\"#.into(),
            "\"" => r#"\""#.into(),
            "\x08" => r#"\b"#.into(),
            "\x0c" => r#"\f"#.into(),
            "\x0a" => r#"\n"#.into(),
            "\x0d" => r#"\r"#.into(),
            "\x09" => r#"\t"#.into(),
            other => format!("\\u{:04x}", other.bytes().next().unwrap()).into(),
        }
    });
    string
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
    for mat in AC.find_iter(input) {
        res += &input[last_start..mat.start()];
        last_start = mat.end();

        let pat_idx = mat.pattern();
        res += if pat_idx < REPLACEMENTS.len() {
            Cow::from(REPLACEMENTS[pat_idx])
        } else {
            last_start += 4;
            let hex_digits = &input[mat.end()..mat.end() + 4];
            let cp = u16::from_str_radix(hex_digits, 16).unwrap();
            if HIGH_SURROGATES.contains(&cp) {
                last_start += 6;
                let hex_digits = &input[mat.end() + 6..mat.end() + 10];
                let low_cp = u16::from_str_radix(hex_digits, 16).unwrap();
                String::from_utf16(&[cp, low_cp]).unwrap()
            } else {
                String::from_utf16(&[cp]).unwrap()
            }
            .into()
        };
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
