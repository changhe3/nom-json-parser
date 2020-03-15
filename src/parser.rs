use crate::repr::Json;
use crate::utils::{
    delimited_list, into, intoc, unescape, with_inputc, wrap_ws, ParserIteratorExt,
    HIGH_SURROGATES, LOW_SURROGATES,
};

use nom::branch::alt;
use nom::bytes::complete::*;
use nom::bytes::streaming::escaped;
use nom::character::complete::*;
use nom::combinator::mapc;
use nom::combinator::*;
use nom::error::{ErrorKind, ParseError};
use nom::lib::std::collections::BTreeMap;
use nom::number::complete::{double, recognize_float};
use nom::sequence::*;
use nom::sequence::{delimitedc, precededc};
use nom::Err::Failure;
use nom::{AsChar, IResult};
use std::borrow::Cow;

pub type ParserResult<'a, O, E> = IResult<&'a str, O, E>;
pub type JsonResult<'a, E> = ParserResult<'a, Json<'a>, E>;

/// Parse JSON from string
///
/// # Example
/// ```rust
/// use nom_json_parser::{parse, Json};
/// use nom::error::ErrorKind;
/// use maplit::btreemap;
/// type E<'a> = (&'a str, ErrorKind);
///
/// let json_str = r#"{
///     "name": "Frank",
///     "age": 18,
///     "phone": {
///         "work": "123-456-7890",
///         "home": "098-765-4321",
///         "fax": null
///     }
/// }"#;
/// let result = parse::<E>(json_str);
/// let json = btreemap!{
///     "name" => Json::from("Frank"),
///     "age" => 18.into(),
///     "phone" => btreemap!{
///         "work" => "123-456-7890".into(),
///         "home" => "098-765-4321".into(),
///         "fax" => Json::from(None)
///     }.into()
/// }.into();
/// assert_eq!(result, Ok(("", json)));
/// ```
pub fn parse_json_element<'a, E: Clone + ParseError<&'a str>>(input: &'a str) -> JsonResult<'a, E> {
    wrap_ws(parse_json)(input)
}

fn parse_json<'a, E: Clone + ParseError<&'a str>>(input: &'a str) -> JsonResult<'a, E> {
    alt((
        parse_null,
        parse_true,
        parse_false,
        parse_number,
        parse_string,
        parse_array,
        parse_object,
    ))(input)
}

fn parse_null<'a, E: ParseError<&'a str>>(input: &'a str) -> JsonResult<'a, E> {
    mapc(input, tag("null"), |_| Json::from(None))
}

fn parse_true<'a, E: ParseError<&'a str>>(input: &'a str) -> JsonResult<'a, E> {
    mapc(input, tag("true"), |_| Json::from(true))
}

fn parse_false<'a, E: ParseError<&'a str>>(input: &'a str) -> JsonResult<'a, E> {
    mapc(input, tag("false"), |_| Json::from(false))
}

fn parse_number<'a, E: ParseError<&'a str>>(input: &'a str) -> JsonResult<'a, E> {
    let (input, (num_str, num)) = with_inputc(input, double)?;
    let json = if num_str.contains(['.', 'e'].as_ref()) {
        num.into()
    } else {
        num_str.parse::<i64>().map(Into::into).unwrap_or(num.into())
    };
    Ok((input, json))
}

fn hex_u16<'a, E: ParseError<&'a str>>(input: &'a str) -> ParserResult<'a, u16, E> {
    let (input, hex_digits) = take_while_m_n(1, 4, |c: char| c.is_hex_digit())(input)?;
    let res: u32 = hex_digits
        .chars()
        .rev()
        .enumerate()
        .map(|(sig, digit)| digit.to_digit(16).unwrap() << (sig * 4) as u32)
        .sum();
    Ok((input, res as u16))
}

fn parse_escape_seq<'a, E: ParseError<&'a str>>(input: &'a str) -> ParserResult<'a, (), E> {
    let (input, esc) = one_of(r#""\/bfnrtu"#)(input)?;
    if esc == 'u' {
        let (input, cp_str) = take(4u8)(input)?;
        let (_, cp) = all_consuming(hex_u16)(cp_str)?;
        if HIGH_SURROGATES.contains(&cp) {
            let (input, low_cp_str) = precededc(input, tag(r#"\u"#), take(4u8))?;
            let (_, low_cp) = all_consuming(hex_u16)(low_cp_str)?;
            if !LOW_SURROGATES.contains(&low_cp) {
                Err(Failure(E::from_error_kind(low_cp_str, ErrorKind::HexDigit)))
            } else {
                Ok((input, ()))
            }
        } else if LOW_SURROGATES.contains(&cp) {
            Err(Failure(E::from_error_kind(cp_str, ErrorKind::HexDigit)))
        } else {
            Ok((input, ()))
        }
    } else {
        Ok((input, ()))
    }
}

fn unquote<'a, E: ParseError<&'a str>>(input: &'a str) -> ParserResult<'a, &'a str, E> {
    precededc(
        input,
        char('"'),
        cut(terminated(
            escaped(none_of(r#"\""#), '\\', parse_escape_seq),
            char('"'),
        )),
    )
}

fn parse_string_raw<'a, E: ParseError<&'a str>>(
    input: &'a str,
) -> ParserResult<'a, Cow<'a, str>, E> {
    mapc(input, unquote, unescape)
}

fn parse_string<'a, E: ParseError<&'a str>>(input: &'a str) -> JsonResult<'a, E> {
    intoc(input, parse_string_raw)
}

fn parse_array<'a, E: Clone + ParseError<&'a str>>(input: &'a str) -> JsonResult<'a, E> {
    delimitedc(
        input,
        terminated(char('['), multispace0),
        |input| {
            delimited_list(input, parse_json_element, char(','))
                .process(|it| it.collect::<Vec<_>>().into())
        },
        preceded(multispace0, char(']')),
    )
}

fn parse_object<'a, E: Clone + ParseError<&'a str>>(input: &'a str) -> JsonResult<'a, E> {
    delimitedc(
        input,
        terminated(char('{'), multispace0),
        |input| {
            delimited_list(
                input,
                tuple((
                    wrap_ws(parse_string_raw),
                    preceded(char(':'), parse_json_element),
                )),
                char(','),
            )
            .process(|it| it.collect::<BTreeMap<_, _>>().into())
        },
        terminated(multispace0, char('}')),
    )
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::repr::JsonValue;
    use crate::utils::escape;
    use assert_matches::assert_matches;
    use itertools::Itertools;
    use prop::collection::{btree_map, vec};
    use proptest::num;
    use proptest::prelude::*;
    use proptest::string::string_regex;

    type E<'a> = (&'a str, ErrorKind);

    fn arb_json_number() -> num::f64::Any {
        num::f64::POSITIVE | num::f64::NEGATIVE | num::f64::ZERO
    }

    fn arb_json_string() -> impl Strategy<Value = String> {
        string_regex(r#"(?:[^\pC\\"]|\\[\\/"bfnrt])*"#).unwrap()
    }

    fn arb_json() -> impl Strategy<Value = Json<'static>> {
        let leaf = prop_oneof![
            Just(None.into()),
            any::<bool>().prop_map(Into::into),
            any::<i64>().prop_map(Into::into),
            arb_json_number().prop_map(Into::into),
            arb_json_string().prop_map(|s| s.into())
        ];
        leaf.prop_recursive(8, 256, 10, |inner| {
            prop_oneof![
                vec(inner.clone(), 0..10).prop_map(Into::into),
                btree_map(arb_json_string(), inner.clone(), 0..10).prop_map(Into::into)
            ]
        })
    }

    #[test]
    fn test_parse_escape_seq() {
        assert_eq!(parse_escape_seq::<E>("t rest"), Ok((" rest", ())));
        assert_eq!(parse_escape_seq::<E>("\" rest"), Ok((" rest", ())));
        assert_matches!(parse_escape_seq::<E>("m rest"), Err(_));
        assert_eq!(parse_escape_seq::<E>("u0020 rest"), Ok((" rest", ())));
        assert_eq!(parse_escape_seq::<E>("uffff rest"), Ok((" rest", ())));
        assert_eq!(
            parse_escape_seq::<E>("ud800\\uDC00 rest"),
            Ok((" rest", ()))
        );
        assert_eq!(
            parse_escape_seq::<E>("ud800\\u1234 rest"),
            Err(Failure(E::from_error_kind("1234", ErrorKind::HexDigit)))
        );
        assert_eq!(
            parse_escape_seq::<E>("udd00 rest"),
            Err(Failure(E::from_error_kind("dd00", ErrorKind::HexDigit)))
        );
        assert_matches!(parse_escape_seq::<E>("ufff rest"), Err(_));
        assert_matches!(parse_escape_seq::<E>("ud800\\udc0 rest"), Err(_));
    }

    #[test]
    fn test_unquote() {
        assert_eq!(unquote::<E>(r#""""#), Ok(("", "")));
        assert_eq!(
            unquote::<E>(r#""a normal string" rest"#),
            Ok((" rest", "a normal string"))
        );
        assert_eq!(
            unquote::<E>(r#""a\tstring\twith\ttabs" rest"#),
            Ok((" rest", r#"a\tstring\twith\ttabs"#))
        );
        assert_eq!(
            unquote::<E>(r#""a\nstring\nwith\nnewlines" rest"#),
            Ok((" rest", r#"a\nstring\nwith\nnewlines"#))
        );
        assert_eq!(
            unquote::<E>(r#""a\r\nstring\r\nwith\r\nnewlines\r\non\r\nwindows" rest"#),
            Ok((
                " rest",
                r#"a\r\nstring\r\nwith\r\nnewlines\r\non\r\nwindows"#
            ))
        );
        assert_eq!(
            unquote::<E>(r#""\u4e00\u4e2a\u542b\u6709UTF-16\u7684\u5b57\u7b26\u4e32" rest"#),
            // "一个含有UTF-16的字符串"
            Ok((
                " rest",
                r#"\u4e00\u4e2a\u542b\u6709UTF-16\u7684\u5b57\u7b26\u4e32"#
            ))
        );
        assert_eq!(
            unquote::<E>(r#""\uD834\uDD1E\u006d\u0075\u0073\uDD1E\u0069\u0063\uD834""#),
            Err(Failure(E::from_error_kind("DD1E", ErrorKind::HexDigit)))
        );
        assert_eq!(
            unquote::<E>(r#""\uD834\uE000\u006d\u0075\u0073\uDD1E\u0069\u0063\uD834""#),
            Err(Failure(E::from_error_kind("E000", ErrorKind::HexDigit)))
        );
    }

    #[test]
    fn test_parse_string() {
        assert_eq!(
            parse_string::<E>(r#""a normal string" rest"#),
            Ok((" rest", "a normal string".into()))
        );
        assert_eq!(
            parse_string::<E>(r#""a\r\nstring\r\nwith\r\nnewlines\r\non\r\nwindows" rest"#),
            Ok((
                " rest",
                "a\r\nstring\r\nwith\r\nnewlines\r\non\r\nwindows".into()
            ))
        );
        assert_eq!(
            parse_string::<E>(r#""\u4e00\u4e2a\u542b\u6709UTF-16\u7684\u5b57\u7b26\u4e32" rest"#),
            // "一个含有UTF-16的字符串"
            Ok((" rest", "一个含有UTF-16的字符串".into()))
        );
        assert_eq!(
            parse_string::<E>(r#""\uD834\uDD1E\u006d\u0075\u0073\uDD1E\u0069\u0063\uD834""#),
            Err(Failure(E::from_error_kind("DD1E", ErrorKind::HexDigit)))
        );
        assert_eq!(
            parse_string::<E>(r#""\uD834\uE000\u006d\u0075\u0073\uDD1E\u0069\u0063\uD834""#),
            Err(Failure(E::from_error_kind("E000", ErrorKind::HexDigit)))
        );
    }

    #[test]
    fn test_parse_string_empty() {
        assert_eq!(parse_string::<E>(r#""""#), Ok(("", "".into())));
    }

    #[test]
    fn test_parse_string_invalid_escape() {
        assert_matches!(parse_string::<E>(r#"hello\a\world"#), Err(_));
        assert_matches!(parse_string::<E>(r#"utf16: \uff"#), Err(_));
    }

    proptest! {
        #[test]
        fn test_parse_string_random(s in "\\PC*") {
            let _ = parse_string::<E>(&s);
        }

        #[test]
        fn test_parse_string_no_escape(s in r#""[^\pC\\"]*""#) {
            let res = parse_string::<E>(&s);
            prop_assert_eq!(res, Ok(("", Json::from(&s[1..s.len() - 1]))));
        }

        #[test]
        fn test_parse_string_regular_escape(s in r#""(?:[^\pC\\"/]|\\[\\/"bfnrt])*"[^"]*"#) {
            let split = &s.rfind('"').unwrap() + 1;
            let unquoted = &s[1..split - 1];
            if let Ok((rest, Json(Some(JsonValue::String(res))))) = parse_string::<E>(&s) {
                prop_assert_eq!(rest, &s[split..]);
                prop_assert_eq!(&escape(&res).replace('/', "\\/"), unquoted);
            } else {
                prop_assert!(false);
            }
        }

        #[test]
        fn test_parse_string_utf16(orig in r#"[^\pC\\"]*"#, rest in r#"[^"]*"#) {
            let s = format!(r#""{}"{}"#, orig.encode_utf16().format_with("", |cp, f| f(&format_args!("\\u{:04X}", cp))), rest);
            if let Ok((remaining, Json(Some(JsonValue::String(res))))) = parse_string::<E>(&s) {
                prop_assert_eq!(remaining, rest);
                prop_assert_eq!(res, orig);
            } else {
                prop_assert!(false);
            }
        }

        #[test]
        fn test_parse_int(i: i64, rest in r#" \PC*"#) {
            if let Ok((remaining, Json(Some(JsonValue::Int(res))))) = parse_number::<E>(&format!("{}{}", i, rest)) {
                prop_assert_eq!(remaining, rest);
                prop_assert_eq!(res, i);
            } else {
                prop_assert!(false);
            }
        }

        #[test]
        fn test_parse_float(f in arb_json_number(), rest in r#" \PC*"#) {
            match parse_number::<E>(&format!("{}{}", f, rest)) {
                Ok((remaining, Json(Some(JsonValue::Int(res))))) => {
                    prop_assert_eq!(remaining, rest);
                    prop_assert_eq!(res as f64, f);
                },
                Ok((remaining, Json(Some(JsonValue::Float(res))))) => {
                    prop_assert_eq!(remaining, rest);
                    prop_assert_eq!(res, f);
                },
                e @ _ => {
                    eprintln!("{:?}", e);
                    prop_assert!(false);
                }
            };
        }

        #[test]
        fn test_parse_json_random(s in r#"\PC*"#) {
            let _ = parse_json_element::<E>(&s);
        }

        #[test]
        fn test_parse_json(json in arb_json()) {
            let input = format!("{:#}", json);
            prop_assert_eq!(parse_json_element::<E>(&input), Ok(("", json)));
        }
    }
}
