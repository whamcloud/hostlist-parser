// Copyright (c) 2022 DDN. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file.

mod structures;

use crate::structures::{flatten_ranges, Part, RangeOutput};
use combine::{
    attempt, between, choice, eof,
    error::{ParseError, StreamError},
    many1, not_followed_by, optional,
    parser::{
        char::{alpha_num, digit, spaces},
        combinator::ignore,
        repeat::repeat_until,
        EasyParser,
    },
    sep_by1,
    stream::{Stream, StreamErrorFor},
    token, Parser,
};
use itertools::Itertools as _;

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
        let mut xs = [start, end];
        xs.sort_unstable();

        let same_prefix_len = start_zeros == end_zeros;

        let (range, start_zeros, end_zeros) = if start > end {
            (
                RangeOutput::RangeReversed(end_zeros, same_prefix_len, end, start),
                end_zeros,
                start_zeros,
            )
        } else {
            (
                RangeOutput::Range(start_zeros, same_prefix_len, start, end),
                start_zeros,
                end_zeros,
            )
        };

        if end_zeros > start_zeros {
            Err(StreamErrorFor::<I>::unexpected_static_message(
                "larger end padding",
            ))
        } else {
            Ok(range)
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
            .with(leading_zeros())
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

pub fn parse(input: &str) -> Result<Vec<String>, combine::stream::easy::Errors<char, &str, usize>> {
    let (hosts, _) = hostlists()
        .easy_parse(input)
        .map_err(|err| err.map_position(|p| p.translate_position(input)))?;

    let mut xs = vec![];

    for parts in hosts {
        let x_prod: Vec<_> = parts
            .iter()
            .filter_map(Part::get_ranges)
            .map(|xs| flatten_ranges(xs))
            .multi_cartesian_product()
            .collect();

        // No ranges means no interpolation
        if x_prod.is_empty() {
            let mut s = String::new();

            for p in parts.clone() {
                if let Part::String(x) = p {
                    s.push_str(&x)
                }
            }

            xs.push(s);
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

                xs.push(s);
            }
        }
    }

    Ok(xs.into_iter().unique().collect())
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
        assert_debug_snapshot!(leading_zeros().easy_parse("042"));
        assert_debug_snapshot!(leading_zeros().easy_parse("042"));
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
        assert_debug_snapshot!(disjoint_digits().easy_parse("1,2,006,0007,3-5"));
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
            "Beginning and ending prefixes don't match two single digit numbers",
            parse("hostname[01-009]")
        );

        assert_debug_snapshot!(
            "Having a closing brace before an opening brace",
            parse("hostname]00[asdf")
        );
    }

    #[test]
    fn test_parse_large_expression() {
        let xs = parse("atla-pio-03-o[048-051],atla-pio-05-o[052-055],atla-pio-07-o[056-059],atla-pio-09-o[060-063],atla-pio-11-o[064-067],atla-pio-13-o[068-071],atla-pip-03-o[072-075],atla-pip-05-o[076-079],atla-pip-07-o[080-083],atla-pip-11-o[088-091],atla-pip-13-o[092-095],atla-pip-09-o[085,087],atla-piq-03-o[096-099],atla-piq-05-o[100-103],atla-piq-07-o[104-107],atla-piq-09-o[108-111],atla-piq-11-o[112-115],atla-piq-13-o[116-119],atla-pir-03-o[120-123],atla-pir-05-o[124-127],atla-pir-07-o[128-131],atla-pir-09-o[132-135],atla-pir-11-o[136-139],atla-pir-13-o[140-143],atla-pis-03-m[000-003],atla-pis-05-o[000-003],atla-pis-07-o[004-007],atla-pis-09-o[008-011],atla-pis-11-o[012-015],atla-pis-13-o[016-019],atla-pis-15-o[020-023],atla-pit-03-m[004-007],atla-pit-05-o[024-027],atla-pit-07-o[028-031],atla-pit-09-o[032-035],atla-pit-11-o[036-039],atla-pit-15-o[044-047],atla-pit-13-o[040,042]").unwrap();

        assert_debug_snapshot!("Large expression", xs);
    }

    #[test]

    fn test_parse_osts() {
        assert_debug_snapshot!("Leading 0s", parse("OST01[00,01]"));
    }
}
