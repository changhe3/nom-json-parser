use crate::repr::Json;
use crate::utils::{f64_to_i64, unescape, HIGH_SURROGATES, LOW_SURROGATES};

use nom::branch::alt;
use nom::bytes::complete::*;
use nom::character::complete::*;
use nom::combinator::*;
use nom::error::{ErrorKind, ParseError};
use nom::number::complete::double;
use nom::sequence::*;
use nom::Err::{Error, Failure};
use nom::IResult;

pub type ParserResult<'a, O, E> = IResult<&'a str, O, E>;
pub type JsonResult<'a, E> = ParserResult<'a, Json<'a>, E>;

pub fn parse<'a, E: ParseError<&'a str>>(input: &'a str) -> JsonResult<'a, E> {
    completec(input, delimited(multispace0, parse_json, multispace0))
}

fn parse_json<'a, E: ParseError<&'a str>>(input: &'a str) -> JsonResult<'a, E> {
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
    let (input, _) = tag("null")(input)?;
    Ok((input, Json::from(None)))
}

fn parse_true<'a, E: ParseError<&'a str>>(input: &'a str) -> JsonResult<'a, E> {
    let (input, _) = tag("true")(input)?;
    Ok((input, Json::from(true)))
}

fn parse_false<'a, E: ParseError<&'a str>>(input: &'a str) -> JsonResult<'a, E> {
    let (input, _) = tag("false")(input)?;
    Ok((input, Json::from(false)))
}

fn parse_number<'a, E: ParseError<&'a str>>(input: &'a str) -> JsonResult<'a, E> {
    let (input, num) = double(input)?;
    let json = if let Some(int) = f64_to_i64(num) {
        int.into()
    } else {
        num.into()
    };
    Ok((input, json))
}

fn hex_u16<'a, E: ParseError<&'a str>>(input: &'a str) -> ParserResult<'a, u16, E> {
    let (input, hex_digits) = take_while_m_n(1, 4, |c: char| c.is_ascii_hexdigit())(input)?;
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
    delimitedc(
        input,
        char('"'),
        escaped(none_of(r#"\""#), '\\', parse_escape_seq),
        char('"'),
    )
}

fn parse_string<'a, E: ParseError<&'a str>>(input: &'a str) -> JsonResult<'a, E> {
    mapc(input, unquote, |string| unescape(string).into())
}

fn parse_array<'a, E: ParseError<&'a str>>(input: &'a str) -> JsonResult<'a, E> {
    todo!()
}

fn parse_object<'a, E: ParseError<&'a str>>(input: &'a str) -> JsonResult<'a, E> {
    todo!()
}

#[cfg(test)]
mod test {
    use super::*;
    use assert_matches::assert_matches;
    use std::borrow::Cow::*;
    use proptest::prelude::*;

    type E<'a> = (&'a str, ErrorKind);

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
        assert_eq!(parse_string::<E>(r#""a normal string" rest"#), Ok((" rest", "a normal string".into())));
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
            Ok((
                " rest",
                "一个含有UTF-16的字符串".into()
            ))
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

}
