use regex::{Replacer, Regex, Captures};
use std::borrow::Cow;
use std::fmt::{Display, Debug, Formatter, Error, Write};
use lazy_static::lazy_static;


pub(crate) trait RegexExt {
    fn cow_replace<'t, R: Replacer>(&self, text: Cow<'t, str>, rep: R) -> Cow<'t, str>;
    fn cow_replace_all<'t, R: Replacer>(&self, text: Cow<'t, str>, rep: R) -> Cow<'t, str>;
    fn cow_replacen<'t, R: Replacer>(&self, text: Cow<'t, str>, limit: usize, rep: R) -> Cow<'t, str>;
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

    fn cow_replacen<'t, R: Replacer>(&self, text: Cow<'t, str>, limit: usize, rep: R) -> Cow<'t, str> {
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
            "\u{0008}" => r#"\b"#.into(),
            "\u{000c}" => r#"\f"#.into(),
            "\u{000a}" => r#"\n"#.into(),
            "\u{000d}" => r#"\r"#.into(),
            "\u{0009}" => r#"\t"#.into(),
            other => format!("\\u{:04x}", other.bytes().next().unwrap()).into(),
        }
    });
    string
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
