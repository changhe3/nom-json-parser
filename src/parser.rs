use crate::repr::{Json, JsonValue};
use nom::branch::alt;
use nom::bytes::complete::*;
use nom::character::complete::*;
use nom::combinator::*;
use nom::error::ParseError;
use nom::sequence::*;
use nom::IResult;

pub type ParserResult<'a, O, E> = IResult<&'a str, O, E>;
pub type JsonResult<'a, E> = ParserResult<'a, Json<'a>, E>;

pub fn parse<'a, E: ParseError<&'a str>>(input: &'a str) -> JsonResult<'a, E> {
    delimitedc(input, multispace0, parse_json, multispace0)
}

fn parse_json<'a, E: ParseError<&'a str>>(input: &'a str) -> JsonResult<'a, E> {
    alt((
        parse_null,
        parse_true,
        parse_false,
        parse_int,
        parse_float,
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

fn parse_int<'a, E: ParseError<&'a str>>(input: &'a str) -> JsonResult<'a, E> {
    todo!()
}

fn parse_float<'a, E: ParseError<&'a str>>(input: &'a str) -> JsonResult<'a, E> {
    todo!()
}

fn parse_string<'a, E: ParseError<&'a str>>(input: &'a str) -> JsonResult<'a, E> {
    todo!()
}

fn parse_array<'a, E: ParseError<&'a str>>(input: &'a str) -> JsonResult<'a, E> {
    todo!()
}

fn parse_object<'a, E: ParseError<&'a str>>(input: &'a str) -> JsonResult<'a, E> {
    todo!()
}
