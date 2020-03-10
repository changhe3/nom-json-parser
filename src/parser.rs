use crate::repr::{Json, JsonValue};
use crate::utils::f64_to_i64;
use ascii::ToAsciiChar;
use nom::branch::alt;
use nom::bytes::complete::*;
use nom::character::complete::*;
use nom::combinator::*;
use nom::error::{ErrorKind, ParseError};
use nom::number::complete::{double, hex_u32};
use nom::sequence::*;
use nom::Err::{Error, Failure};
use nom::IResult;
use std::ops::Range;

const HIGH_SURROGATES: Range<u16> = 0xd800..0xdc00;
const LOW_SURROGATES: Range<u16> = 0xdc00..0xe000;

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
    let mut v = 0u32;
    let mut iter = input.chars();
    for _ in 0..4 {
        if let Some(chr) = iter.next() {
            if let Some(digit) = chr.to_digit(16) {
                v <<= 4;
                v += digit;
                continue;
            }
        }
        break;
    }
    Ok((iter.as_str(), v as u16))
}

fn parse_escape_seq<'a, E: ParseError<&'a str>>(input: &'a str) -> ParserResult<'a, (), E> {
    let (input, esc) = one_of(r#""\/bfnrtu"#)(input)?;
    if esc == 'u' {
        let (input, cp_str) = take(4u8)(input)?;
        let (_, cp) = completec(cp_str, hex_u16)?;
        if HIGH_SURROGATES.contains(&cp) {
            let (input, low_cp_str) = precededc(input, tag(r#"\u"#), take(4u8))?;
            let (_, low_cp) = completec(low_cp_str, hex_u16)?;
            if !LOW_SURROGATES.contains(&low_cp) {
                return Err(Failure(E::from_error_kind(low_cp_str, ErrorKind::HexDigit)));
            }
        } else if LOW_SURROGATES.contains(&cp) {
            return Err(Failure(E::from_error_kind(cp_str, ErrorKind::HexDigit)));
        }
    }
    Ok((input, ()))
}

fn parse_string<'a, E: ParseError<&'a str>>(input: &'a str) -> JsonResult<'a, E> {
    let (input, escaped_str) = delimitedc(
        input,
        char('"'),
        escaped(none_of(r#"\""#), '\\', parse_escape_seq),
        char('"'),
    )?;
    todo!()
}

fn parse_array<'a, E: ParseError<&'a str>>(input: &'a str) -> JsonResult<'a, E> {
    todo!()
}

fn parse_object<'a, E: ParseError<&'a str>>(input: &'a str) -> JsonResult<'a, E> {
    todo!()
}
