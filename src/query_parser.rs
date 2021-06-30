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
    bytes::complete::{escaped_transform, tag, take_while1, take_while_m_n},
    character::complete::{char, digit1, none_of, one_of},
    combinator::{all_consuming, map, map_res},
    error::Error,
    multi::many0,
    sequence::{delimited, preceded, tuple},
    Err, IResult,
};

fn hex_unicode_scalar(len: usize, s: &str) -> IResult<&str, char> {
	map_res(take_while_m_n(len, len, |c: char| c.is_ascii_hexdigit()), |s: &str| {
		char::try_from(u32::from_str_radix(s, 16).unwrap())
	})(s)
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
    let string_body = alt((
        escaped_transform(none_of("\\\""), '\\', basic_string_escape),
        // TODO report a nom bug in escaped_transform: it rejects empty sequence.
        //   https://github.com/Geal/nom/issues/953#issuecomment-525557597
        //   https://docs.rs/nom/7.1.1/src/nom/bytes/complete.rs.html#570-577
        map(tag(""), String::from),
    ));
    delimited(char('"'), string_body, char('"'))(s)
}

fn bare_string(s: &str) -> IResult<&str, &str> {
	take_while1(|c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_')(s)
}

fn key_string(s: &str) -> IResult<&str, String> {
	alt((basic_string, map(bare_string, String::from)))(s)
}

fn array_index(s: &str) -> IResult<&str, usize> {
    map_res(digit1, |i: &str| i.parse())(s)
}

fn tpath_segment_name(s: &str) -> IResult<&str, TpathSegment> {
	map(key_string, TpathSegment::Name)(s)
}

#[rustfmt::skip]
fn tpath_segment_num(s: &str) -> IResult<&str, TpathSegment> {
	map(delimited(char('['), array_index, char(']')), TpathSegment::Num)(s)
}

#[rustfmt::skip]
fn tpath_segment_rest(s: &str) -> IResult<&str, TpathSegment> {
	alt((preceded(char('.'), tpath_segment_name), tpath_segment_num))(s)
}

#[rustfmt::skip]
fn tpath(s: &str) -> IResult<&str, Vec<TpathSegment>> {
    alt((
        map(all_consuming(char('.')), |_| vec![]),
        // Must start with a name, because TOML root is always a table.
        map(tuple((tpath_segment_name, many0(tpath_segment_rest))),
            |(hd, mut tl)| { tl.insert(0, hd); tl }),
    ))(s)
}

pub fn parse_query(s: &str) -> Result<Query, Err<Error<&str>>> {
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
        ("a", Ok(vec![name("a")])),
        ("a.b", Ok(vec![name("a"), name("b")])),
        ("\"a.b\"", Ok(vec![name("a.b")])),
        ("\"\"", Ok(vec![name("")])),
        ("a.\"\".b", Ok(vec![name("a"), name(""), name("b")])),
        ("..", Err(())),
        ("a[1]", Ok(vec![name("a"), Num(1)])),
        ("a[b]", Err(())),
        ("a[1].b", Ok(vec![name("a"), Num(1), name("b")])),
        ("a.b[1]", Ok(vec![name("a"), name("b"), Num(1)])),
    ] {
        let actual = parse_query(s);
        // This could use some slicker check that prints the actual on failure.
        // Also nice would be to proceed to try the other test cases.
        match expected {
            Ok(q) => assert!(q == actual.unwrap().0),
            Err(_) => assert!(actual.is_err()),
        }
    }
}
