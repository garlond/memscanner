use nom::{
    branch::alt,
    bytes::complete::{tag, take},
    character::complete::digit1,
    combinator::{map_res, opt, recognize, value},
    multi::many1,
    sequence::pair,
    IResult,
};

use super::{Match, Op};

#[rustfmt::skip]
fn match_signed_integer(input: &str) -> IResult<&str, &str> {
    recognize(pair(
        opt(
            alt((
                 tag("-"),
                 tag("+")
            ))
        ),
        digit1
    ))(input)
}

fn parse_i32(input: &str) -> IResult<&str, i32> {
    map_res(match_signed_integer, |s: &str| s.parse::<i32>())(input)
}

fn literal_from_hex(input: &str) -> Result<Match, std::num::ParseIntError> {
    let val = u8::from_str_radix(input, 16)?;
    Ok(Match::Literal(val))
}

fn parse_literal(input: &str) -> IResult<&str, Match> {
    map_res(take(2usize), literal_from_hex)(input)
}

fn parse_any(input: &str) -> IResult<&str, Match> {
    value(Match::Any, tag("**"))(input)
}

fn parse_position(input: &str) -> IResult<&str, Match> {
    value(Match::Position, tag("^^"))(input)
}

fn parse_match(input: &str) -> IResult<&str, Match> {
    alt((parse_any, parse_position, parse_literal))(input)
}

fn parse_lea(input: &str) -> IResult<&str, Op> {
    let (input, _) = tag("asm(")(input)?;
    let (input, pattern) = many1(parse_match)(input)?;
    let (input, _) = tag(")")(input)?;

    Ok((input, Op::Asm(pattern)))
}

fn parse_ptr(input: &str) -> IResult<&str, Op> {
    let (input, _) = tag("ptr(")(input)?;
    let (input, offset) = parse_i32(input)?;
    let (input, _) = tag(")")(input)?;

    Ok((input, Op::Ptr(offset)))
}

pub(super) fn parse_op(input: &str) -> IResult<&str, Op> {
    alt((parse_lea, parse_ptr))(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    use failure::Error;

    #[test]
    fn match_from_hext_test() -> Result<(), Error> {
        let m = literal_from_hex("00")?;
        assert_eq!(m, Match::Literal(0x00));

        let m = literal_from_hex("5a")?;
        assert_eq!(m, Match::Literal(0x5a));

        let m = literal_from_hex("a5")?;
        assert_eq!(m, Match::Literal(0xa5));

        let m = literal_from_hex("ff")?;
        assert_eq!(m, Match::Literal(0xff));

        Ok(())
    }

    #[test]
    fn parse_any_test() -> Result<(), Error> {
        assert_eq!(parse_any("**"), Ok(("", Match::Any)));
        assert_eq!(
            parse_any("ab"),
            Err(nom::Err::Error(("ab", nom::error::ErrorKind::Tag)))
        );

        Ok(())
    }

    #[test]
    fn parse_position_test() -> Result<(), Error> {
        assert_eq!(parse_position("^^"), Ok(("", Match::Position)));
        assert_eq!(
            parse_position("**"),
            Err(nom::Err::Error(("**", nom::error::ErrorKind::Tag)))
        );

        Ok(())
    }

    #[test]
    fn parse_match_test() -> Result<(), Error> {
        assert_eq!(parse_match("ab"), Ok(("", Match::Literal(0xab))));
        assert_eq!(parse_match("**"), Ok(("", Match::Any)));
        assert_eq!(parse_match("^^"), Ok(("", Match::Position)));
        Ok(())
    }

    #[test]
    fn parse_i32_test() -> Result<(), Error> {
        let ints = vec![i32::min_value(), i32::max_value(), 0];
        for i in ints {
            assert_eq!(parse_i32(&format!("{}", i)), Ok(("", i)));
        }

        assert_eq!(
            parse_i32("a1"),
            Err(nom::Err::Error(("a1", nom::error::ErrorKind::Digit)))
        );
        assert_eq!(parse_i32("1a"), Ok(("a", 1)));

        Ok(())
    }

    #[test]
    fn parse_ptr_test() -> Result<(), Error> {
        assert_eq!(parse_ptr("ptr(-1)"), Ok(("", Op::Ptr(-1))));
        assert_eq!(parse_ptr("ptr(8)"), Ok(("", Op::Ptr(8))));
        Ok(())
    }

    #[test]
    fn parse_pattern_test() -> Result<(), Error> {
        assert_eq!(
            parse_op("asm(01234567********^^^^^^^^89abcdef)"),
            Ok((
                "",
                Op::Asm(vec![
                    Match::Literal(0x01),
                    Match::Literal(0x23),
                    Match::Literal(0x45),
                    Match::Literal(0x67),
                    Match::Any,
                    Match::Any,
                    Match::Any,
                    Match::Any,
                    Match::Position,
                    Match::Position,
                    Match::Position,
                    Match::Position,
                    Match::Literal(0x89),
                    Match::Literal(0xab),
                    Match::Literal(0xcd),
                    Match::Literal(0xef),
                ])
            ))
        );
        Ok(())
    }
}
