use std::convert::TryFrom;

/// Query language is simple: a query is a "TOML path", or tpath.
pub struct Query(pub Vec<TpathSegment>);

#[derive(Debug, PartialEq, Eq)]
pub enum TpathSegment {
    Name(String),
    Num(usize),
}

use nom::{
    branch::alt,
    bytes::complete::{escaped_transform, take_while1, take_while_m_n},
    character::complete::{char, digit1, none_of, one_of},
    combinator::{all_consuming, map, map_res},
    error::ErrorKind,
    multi::many1,
    sequence::{delimited, preceded},
    Err, IResult,
};

fn hex_unicode_scalar(len: usize, s: &str) -> IResult<&str, char> {
    map_res(
        take_while_m_n(len, len, |c: char| c.is_ascii_hexdigit()),
        |s: &str| char::try_from(u32::from_str_radix(s, 16).unwrap()),
    )(s)
}

fn basic_string_escape(s: &str) -> IResult<&str, char> {
    alt((
        one_of("\\\""),
        map(char('b'), |_| '\x08'),
        map(char('t'), |_| '\t'),
        map(char('n'), |_| '\n'),
        map(char('f'), |_| '\x0c'),
        map(char('r'), |_| '\r'),
        preceded(char('u'), |s| hex_unicode_scalar(4, s)),
        preceded(char('U'), |s| hex_unicode_scalar(8, s)),
    ))(s)
}

fn basic_string(s: &str) -> IResult<&str, String> {
    let string_body = escaped_transform(none_of("\\\""), '\\', basic_string_escape);
    delimited(char('"'), string_body, char('"'))(s)
}

fn bare_string(s: &str) -> IResult<&str, &str> {
    take_while1(|c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_')(s)
}

fn array_index(s: &str) -> IResult<&str, usize> {
    map_res(digit1, |i: &str| usize::from_str_radix(i, 10))(s)
}

fn tpath_segment_name(s: &str) -> IResult<&str, String> {
    preceded(
        char('.'),
        alt((basic_string, map(bare_string, String::from))),
    )(s)
}

fn tpath_segment_num(s: &str) -> IResult<&str, usize> {
    delimited(char('['), array_index, char(']'))(s)
}

fn tpath_segment(s: &str) -> IResult<&str, TpathSegment> {
    alt((
        map(tpath_segment_name, TpathSegment::Name),
        map(tpath_segment_num, TpathSegment::Num),
    ))(s)
}

fn tpath(s: &str) -> IResult<&str, Vec<TpathSegment>> {
    alt((
        map(all_consuming(char('.')), |_| vec![]),
        many1(tpath_segment),
    ))(s)
}

pub fn parse_query(s: &str) -> Result<Query, Err<(&str, ErrorKind)>> {
    all_consuming(tpath)(s).map(|(trailing, res)| {
        assert!(trailing.is_empty());
        Query(res)
    })
}

#[test]
fn test_parse_query() {
    use TpathSegment::{Name, Num};
    let name = |n: &str| Name(n.to_string());
    for (s, expected) in vec![
        (".", Ok(vec![])),
        (".a", Ok(vec![name("a")])),
        (".\"a.b\"", Ok(vec![name("a.b")])),
        ("..", Err(())),
        (".a[1]", Ok(vec![name("a"), Num(1)])),
        (".a[b]", Err(())),
    ] {
        let actual = parse_query(s);
        // This could use some slicker check that prints the actual on failure.
        // Also nice would be to proceed to try the other test cases.
        match expected {
            Ok(q) => assert!(q == actual.unwrap().0),
            Err(_) => assert!(actual.is_err())
        }
    }
}
