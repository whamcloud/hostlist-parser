// Copyright (c) 2019 DDN. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file.

mod structures;

use crate::structures::{flatten_ranges, Part, RangeOutput};
use combine::{
    attempt, between,
    char::{alpha_num, digit, spaces},
    choice,
    combinator::ignore,
    eof,
    error::{ParseError, StreamError},
    many1, not_followed_by, optional,
    parser::repeat::repeat_until,
    parser::EasyParser,
    sep_by1,
    stream::{Stream, StreamErrorFor},
    token, Parser,
};
use itertools::Itertools;
use std::collections::BTreeSet;

fn comma<I>() -> impl Parser<I, Output = char>
where
    I: Stream<Token = char>,
    I::Error: ParseError<I::Token, I::Range, I::Position>,
{
    token(',')
}

fn open_bracket<I>() -> impl Parser<I, Output = char>
where
    I: Stream<Token = char>,
    I::Error: ParseError<I::Token, I::Range, I::Position>,
{
    token('[')
}

fn close_bracket<I>() -> impl Parser<I, Output = char>
where
    I: Stream<Token = char>,
    I::Error: ParseError<I::Token, I::Range, I::Position>,
{
    token(']')
}

fn dash<I>() -> impl Parser<I, Output = char>
where
    I: Stream<Token = char>,
    I::Error: ParseError<I::Token, I::Range, I::Position>,
{
    token('-')
}

fn optional_spaces<I>() -> impl Parser<I, Output = Option<()>>
where
    I: Stream<Token = char>,
    I::Error: ParseError<I::Token, I::Range, I::Position>,
{
    optional(spaces())
}

fn host_elements<I>() -> impl Parser<I, Output = String>
where
    I: Stream<Token = char>,
    I::Error: ParseError<I::Token, I::Range, I::Position>,
{
    many1(alpha_num().or(dash()).or(token('.')))
}

fn digits<I>() -> impl Parser<I, Output = String>
where
    I: Stream<Token = char>,
    I::Error: ParseError<I::Token, I::Range, I::Position>,
{
    many1(digit())
}

fn parsed_digits<I>() -> impl Parser<I, Output = u64>
where
    I: Stream<Token = char>,
    I::Error: ParseError<I::Token, I::Range, I::Position>,
{
    digits().and_then(|x| x.parse::<u64>().map_err(StreamErrorFor::<I>::other))
}

fn leading_zeros<I>() -> impl Parser<I, Output = (usize, u64)>
where
    I: Stream<Token = char>,
    I::Error: ParseError<I::Token, I::Range, I::Position>,
{
    digits().and_then(|x| {
        let mut digits = x.chars().take_while(|x| x == &'0').count();

        if x.len() == digits {
            digits -= 1;
        }

        x.parse::<u64>()
            .map(|num| (digits, num))
            .map_err(StreamErrorFor::<I>::other)
    })
}

fn range_digits<I>() -> impl Parser<I, Output = RangeOutput>
where
    I: Stream<Token = char>,
    I::Error: ParseError<I::Token, I::Range, I::Position>,
{
    attempt((
        leading_zeros(),
        optional_spaces().with(dash()),
        optional_spaces().with(leading_zeros()),
    ))
    .and_then(|((start_zeros, start), _, (end_zeros, end))| {
        let mut xs = vec![start, end];
        xs.sort();

        let (start, end, start_zeros, end_zeros) = if start > end {
            (end, start, end_zeros, start_zeros)
        } else {
            (start, end, start_zeros, end_zeros)
        };

        if end_zeros > start_zeros {
            Err(StreamErrorFor::<I>::unexpected_static_message(
                "larger end padding",
            ))
        } else {
            Ok(RangeOutput::Range(start_zeros, start, end))
        }
    })
}

fn disjoint_digits<I>() -> impl Parser<I, Output = RangeOutput>
where
    I: Stream<Token = char>,
    I::Error: ParseError<I::Token, I::Range, I::Position>,
{
    let not_name = not_followed_by(
        optional_spaces()
            .with(digits())
            .skip(optional_spaces())
            .skip(dash())
            .map(|_| ""),
    );

    sep_by1(
        optional_spaces()
            .with(parsed_digits())
            .skip(optional_spaces()),
        attempt(comma().skip(not_name)),
    )
    .map(RangeOutput::Disjoint)
}

fn range<I>() -> impl Parser<I, Output = Vec<RangeOutput>>
where
    I: Stream<Token = char>,
    I::Error: ParseError<I::Token, I::Range, I::Position>,
{
    between(
        open_bracket(),
        close_bracket(),
        sep_by1(range_digits().or(disjoint_digits()), comma()),
    )
}

fn hostlist<I>() -> impl Parser<I, Output = Vec<Part>>
where
    I: Stream<Token = char>,
    I::Error: ParseError<I::Token, I::Range, I::Position>,
{
    repeat_until(
        choice([
            range().map(Part::Range).left(),
            optional_spaces()
                .with(host_elements())
                .map(Part::String)
                .right(),
        ]),
        attempt(optional_spaces().skip(ignore(comma()).or(eof()))),
    )
    .and_then(|xs: Vec<_>| {
        if xs.is_empty() {
            Err(StreamErrorFor::<I>::unexpected_static_message(
                "no host found",
            ))
        } else {
            Ok(xs)
        }
    })
}

fn hostlists<I>() -> impl Parser<I, Output = Vec<Vec<Part>>>
where
    I: Stream<Token = char>,
    I::Error: ParseError<I::Token, I::Range, I::Position>,
{
    sep_by1(hostlist(), optional_spaces().with(comma()))
}

pub fn parse(
    input: &str,
) -> Result<BTreeSet<String>, combine::stream::easy::Errors<char, &str, usize>> {
    let (hosts, _) = hostlists()
        .easy_parse(input)
        .map_err(|err| err.map_position(|p| p.translate_position(input)))?;

    let mut xs = BTreeSet::new();

    for parts in hosts {
        let x_prod: Vec<_> = parts
            .iter()
            .filter_map(Part::get_ranges)
            .map(|xs| flatten_ranges(xs))
            .multi_cartesian_product()
            .collect();

        // No ranges means no interpolation
        if x_prod.is_empty() {
            if parts.is_empty() {}

            let mut s = String::new();

            for p in parts.clone() {
                if let Part::String(x) = p {
                    s.push_str(&x)
                }
            }

            xs.insert(s);
        } else {
            for ys in x_prod {
                let mut it = ys.iter();

                let mut s = String::new();

                for p in parts.clone() {
                    match p {
                        Part::String(x) => s.push_str(&x),
                        Part::Range(_) => s.push_str(it.next().unwrap()),
                    }
                }

                xs.insert(s);
            }
        }
    }

    Ok(xs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use combine::parser::EasyParser;
    use insta::assert_debug_snapshot;

    #[test]
    fn test_leading_zeros() {
        assert_debug_snapshot!(leading_zeros().easy_parse("001"));
        assert_debug_snapshot!(leading_zeros().easy_parse("0001"));
        assert_debug_snapshot!(leading_zeros().easy_parse("01"));
        assert_debug_snapshot!(leading_zeros().easy_parse("00"));
        assert_debug_snapshot!(leading_zeros().easy_parse("0"));
    }

    #[test]
    fn test_range_digits() {
        assert_debug_snapshot!(range_digits().easy_parse("001-003"));
        assert_debug_snapshot!(range_digits().easy_parse("001 -  003"));
        assert_debug_snapshot!(range_digits().easy_parse("1-100"));
        assert_debug_snapshot!(range_digits().easy_parse("100-0"));
    }

    #[test]
    fn test_disjoint_digits() {
        assert_debug_snapshot!(disjoint_digits().easy_parse("1,2,3,4,5]"));
        assert_debug_snapshot!(disjoint_digits().easy_parse("1,2,3-5"));
    }

    #[test]
    fn test_range() {
        assert_debug_snapshot!(range().easy_parse("[1,2,3,4,5]"));
        assert_debug_snapshot!(range().easy_parse("[1,2,3-5]"));
        assert_debug_snapshot!(range().easy_parse("[1,2,3-5,6,7,8-10]"));
        assert_debug_snapshot!(range().easy_parse("[01-10]"));
    }

    #[test]
    fn test_hostlist() {
        assert_debug_snapshot!(hostlist().easy_parse("oss1.local"));
        assert_debug_snapshot!(hostlist().easy_parse("oss[1,2].local"));
        assert_debug_snapshot!(hostlist().easy_parse(
            "hostname[2,6,7].iml.com,hostname[10,11-12,2-3,5].iml.com,hostname[15-17].iml.com"
        ));
    }

    #[test]
    fn test_hostlists() {
        assert_debug_snapshot!(hostlists().easy_parse("oss1.local"));
        assert_debug_snapshot!(hostlists().easy_parse("oss[1,2].local"));
        assert_debug_snapshot!(hostlists().easy_parse(
            "hostname[2,6,7].iml.com,hostname[10,11-12,2-3,5].iml.com,hostname[15-17].iml.com"
        ));
        assert_debug_snapshot!(hostlists().easy_parse(
            "hostname[2,6,7].iml.com, hostname[10,11-12,2-3,5].iml.com, hostname[15-17].iml.com"
        ));
    }

    #[test]
    fn test_parse() {
        assert_debug_snapshot!(parse("oss[1,2].local"));

        assert_debug_snapshot!(parse("oss1.local"));

        assert_debug_snapshot!(parse("hostname[10,11-12,002-003,5].iml.com"));

        assert_debug_snapshot!(parse(
            "hostname[2,6,7].iml.com,hostname[10,11-12,2-3,5].iml.com,hostname[15-17].iml.com"
        ));

        assert_debug_snapshot!(parse(
            "hostname[2,6,7].iml.com,  hostname[10,11-12,2-3,5].iml.com, hostname[15-17].iml.com"
        ));

        assert_debug_snapshot!("single item without ranges", parse("hostname1.iml.com"));

        assert_debug_snapshot!(
            "two items without ranges",
            parse("hostname1.iml.com, hostname2.iml.com")
        );

        assert_debug_snapshot!(
            "single item with single range",
            parse("hostname[6].iml.com")
        );

        assert_debug_snapshot!(
            "single item with single range and nothing after the range",
            parse("hostname[6]")
        );

        assert_debug_snapshot!(
            "single item with single digit prefixed range",
            parse("hostname[09-11]")
        );
        assert_debug_snapshot!(
            "single item with double digit prefixed range",
            parse("hostname[009-011]")
        );

        assert_debug_snapshot!("single item in reverse order", parse("hostname[7-5]"));

        assert_debug_snapshot!(
            "multiple items combined into regular and reverse order",
            parse("hostname[7-5], hostname[8,9], hostname[3,2,1]")
        );

        assert_debug_snapshot!("long range with prefix", parse("hostname[001-999]"));

        assert_debug_snapshot!(
            "single item with two ranges",
            parse("hostname[6,7]-[9-11].iml.com")
        );

        assert_debug_snapshot!(
            "single item with range containing mixture of comma and dash",
            parse("hostname[7,9-11].iml.com")
        );

        assert_debug_snapshot!(
            "Single range per hostname with dup",
            parse(
                "hostname[2,6,7].iml.com,hostname[10,11-12,2-4,5].iml.com, hostname[15-17].iml.com"
            )
        );

        assert_debug_snapshot!("Multiple ranges per hostname in which the difference is 1", parse("hostname[1,2-3].iml[2,3].com,hostname[3,4,5].iml[2,3].com,hostname[5-6,7].iml[2,3].com"));

        assert_debug_snapshot!(
            "Multiple ranges per hostname in which the difference is 1 two formats",
            parse("hostname[1,2-3].iml[2,3].com,hostname[1,2,3].iml[2,4].com")
        );

        assert_debug_snapshot!(
            "Multiple ranges per hostname in which the difference is gt 1",
            parse("hostname[1,2-3].iml[2,3].com,hostname[4,5].iml[3,4].com")
        );

        assert_debug_snapshot!(
            "no prefix to prefix should throw an error",
            parse("hostname[9-0011]").unwrap_err()
        );

        assert_debug_snapshot!(
            "Overlapping ranges",
            parse("hostname[1,2-3].iml[2,3].com,hostname[3,4,5].iml[3,4].com")
        );

        assert_debug_snapshot!(
            "Duplicate without using a range",
            parse("hostname4.iml.com,hostname4.iml.com")
        );

        assert_debug_snapshot!(
            "Single item with single range and additional characters after range",
            parse("hostname[0-3]-eth0.iml.com")
        );

        assert_debug_snapshot!(
            "Single item with three ranges separated by character",
            parse("hostname[1,2]-[3-4]-[5,6].iml.com")
        );

        assert_debug_snapshot!(
            "Single item with two ranges and no separation between the ranges",
            parse("hostname[1,2][3,4].iml.com")
        );

        assert_debug_snapshot!(
            "Single item with prefix range starting at 0",
            parse("test[000-002].localdomain")
        );

        assert_debug_snapshot!(
            "Combined Ranges",
            parse(
                "hostname[2,6,7].iml.com,hostname[10,11-12,2-3,5].iml.com, hostname[15-17].iml.com"
            )
        );

        assert_debug_snapshot!(
            "Padding with a single and double digit number",
            parse("hostname[06-10]")
        );

        assert_debug_snapshot!(
            "Invalid character in range (snowman)",
            parse("test[00☃-002].localdomain")
        );

        assert_debug_snapshot!("Empty expression", parse(""));

        assert_debug_snapshot!(
            "No separation between comma's",
            parse("hostname[1,,2].iml.com")
        );

        assert_debug_snapshot!(
            "No separation between dashes",
            parse("hostname[1--2].iml.com")
        );

        assert_debug_snapshot!(
            "No separation between dash and comma",
            parse("hostname[1-,2].iml.com")
        );

        assert_debug_snapshot!(
            "No separation between comma and dash",
            parse("hostname[1,-2].iml.com")
        );

        assert_debug_snapshot!("Missing closing brace", parse("hostname[1"));

        assert_debug_snapshot!("Ending an expression with a comma", parse("hostname[1],"));

        assert_debug_snapshot!(
            "Beginning and ending prefixes that don't match with two single digit numbers",
            parse("hostname[01-009]")
        );

        assert_debug_snapshot!(
            "Having a closing brace before an opening brace",
            parse("hostname]00[asdf")
        );
    }
}
