use crate::utils::{escape, PadAdapter};

use derive_more::From as DmFrom;
use itertools::Itertools;
use shrinkwraprs::Shrinkwrap;
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt::Display;
use std::fmt::{Error, Formatter, Write};
use std::iter::FromIterator;

#[derive(Shrinkwrap, PartialEq, Clone, DmFrom, Debug)]
pub struct Json<'a>(Option<JsonValue<'a>>);

#[derive(PartialEq, DmFrom, Clone, Debug)]
pub enum JsonValue<'a> {
    Int(i64),
    Float(f64),
    String(Cow<'a, str>),
    Bool(bool),
    #[from(ignore)]
    Object(BTreeMap<Cow<'a, str>, Json<'a>>),
    #[from(ignore)]
    Array(Vec<Json<'a>>),
}

impl<'a> From<&'a str> for JsonValue<'a> {
    fn from(arg: &'a str) -> Self {
        JsonValue::String(arg.into())
    }
}

impl<'a: 'b, 'b> From<&'b [Json<'a>]> for JsonValue<'a> {
    fn from(arg: &'b [Json<'a>]) -> Self {
        JsonValue::Array(arg.to_vec())
    }
}

impl<'a, V: Into<Json<'a>>> FromIterator<V> for JsonValue<'a> {
    fn from_iter<T: IntoIterator<Item = V>>(iter: T) -> Self {
        JsonValue::Array(iter.into_iter().map(V::into).collect())
    }
}

impl<'a, K: Into<Cow<'a, str>>, V: Into<Json<'a>>> FromIterator<(K, V)> for JsonValue<'a> {
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        JsonValue::Object(
            iter.into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        )
    }
}

impl Display for Json<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        match &self.0 {
            Some(json_value) => {
                json_value.fmt(f)?;
            }
            None => {
                f.write_str("null")?;
            }
        };
        Ok(())
    }
}

impl<'a, T: Into<JsonValue<'a>>> From<T> for Json<'a> {
    fn from(v: T) -> Self {
        Json(v.into().into())
    }
}

impl<'a, K: Into<Cow<'a, str>>, V: Into<Json<'a>>> From<BTreeMap<K, V>> for JsonValue<'a> {
    fn from(v: BTreeMap<K, V>) -> Self {
        Self::from_iter(v)
    }
}

impl<'a, T: Into<Json<'a>>> From<Vec<T>> for JsonValue<'a> {
    fn from(v: Vec<T>) -> Self {
        Self::from_iter(v)
    }
}

impl Display for JsonValue<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        match self {
            JsonValue::Int(int) => {
                f.write_str(&int.to_string())?;
            }
            JsonValue::Float(float) => {
                f.write_str(&float.to_string())?;
            }
            JsonValue::String(string) => {
                f.write_str("\"")?;
                f.write_str(&escape(string))?;
                f.write_str("\"")?;
            }
            JsonValue::Bool(boolean) => {
                f.write_str(&boolean.to_string())?;
            }
            JsonValue::Object(obj) => {
                if f.alternate() {
                    let mut f: PadAdapter = f.into();
                    f.write_str("{\n")?;
                    let formatter = obj.iter().format_with(",\n", |(k, v), f| {
                        f(&format_args!("\"{:#}\": {:#}", escape(k), v))
                    });
                    f.write_fmt(format_args!("{}", formatter))?;
                    f.into_inner().write_str("\n}")?;
                } else {
                    f.write_str("{")?;
                    let formatter = obj.iter().format_with(", ", |(k, v), f| {
                        f(&format_args!("\"{}\": {}", escape(k), v))
                    });
                    f.write_fmt(format_args!("{}", formatter))?;
                    f.write_str("}")?;
                };
            }
            JsonValue::Array(arr) => {
                if f.alternate() {
                    let mut f: PadAdapter = f.into();
                    f.write_str("[\n")?;
                    let formatter = arr
                        .iter()
                        .format_with(",\n", |elem, f| f(&format_args!("{:#}", elem)));
                    f.write_fmt(format_args!("{}", formatter))?;
                    f.into_inner().write_str("\n]")?;
                } else {
                    f.write_str("[")?;
                    f.write_fmt(format_args!("{}", arr.iter().format(", ")))?;
                    f.write_str("]")?;
                };
            }
        };
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use maplit::btreemap;

    #[test]
    fn manual_tests() {
        let json: Json = btreemap! {
            "a" => 1,
            "b" => 2,
        }
        .into();
        println!("{}", json);
        println!("{:#}", json);
        let nested_json: Json = btreemap! {
            "age" => 20.into(),
            "name" => "Alice".into(),
            "phone_nums" => Json::from(btreemap! {
                "home\t" => "123-456-7890".into(),
                "work" => btreemap! {
                    "office1" => "111-111-1111",
                    "office2" => "222-222-2222"
                }.into(),
                "fax" => Json::from(None)
            }),
            "friends" => vec!["Brown", "Catherine", "Dell"].into()
        }
        .into();
        println!("{}", nested_json);
        println!("{:#}", nested_json);
    }

    #[test]
    fn test_escape() {
        assert_eq!(&escape(r#""Hello\World""#), r#"\"Hello\\World\""#);
        assert_eq!(
            &escape("a\tb\tc\td\u{0000}\u{001a}"),
            r#"a\tb\tc\td\u0000\u001a"#
        );
    }
}
