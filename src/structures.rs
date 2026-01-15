// Copyright (c) 2022 DDN. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file.

pub trait Cardinality {
    fn cardinality(&self) -> u64;
}

#[derive(Debug, Clone)]
pub(crate) enum RangeOutput {
    Range(usize, bool, u64, u64),
    RangeReversed(usize, bool, u64, u64),
    Disjoint(Vec<(usize, u64)>),
    HexRange(usize, bool, u64, u64),
    HexRangeReversed(usize, bool, u64, u64),
    HexDisjoint(Vec<(usize, u64)>),
}

impl Cardinality for RangeOutput {
    fn cardinality(&self) -> u64 {
        match self {
            RangeOutput::Range(_, _, start, end) | RangeOutput::HexRange(_, _, start, end) => {
                end - start
            }
            RangeOutput::RangeReversed(_, _, start, end)
            | RangeOutput::HexRangeReversed(_, _, start, end) => start - end,
            RangeOutput::Disjoint(items) | RangeOutput::HexDisjoint(items) => items.len() as u64,
        }
    }
}

impl RangeOutput {
    pub(crate) fn iter(&self) -> RangeOutputIter {
        match self {
            RangeOutput::Range(prefix, same_prefix_len, start, end) => {
                RangeOutputIter::External(*prefix, *same_prefix_len, Box::new(*start..=*end))
            }
            RangeOutput::RangeReversed(prefix, same_prefix_len, end, start) => {
                RangeOutputIter::External(
                    *prefix,
                    *same_prefix_len,
                    Box::new((*end..=*start).rev()),
                )
            }
            RangeOutput::Disjoint(xs) => {
                RangeOutputIter::Internal(Box::new(xs.clone().into_iter()))
            }
            RangeOutput::HexRange(prefix, same_prefix_len, start, end) => {
                RangeOutputIter::ExternalHex(*prefix, *same_prefix_len, Box::new(*start..=*end))
            }
            RangeOutput::HexRangeReversed(prefix, same_prefix_len, end, start) => {
                RangeOutputIter::ExternalHex(
                    *prefix,
                    *same_prefix_len,
                    Box::new((*end..=*start).rev()),
                )
            }
            RangeOutput::HexDisjoint(xs) => {
                RangeOutputIter::InternalHex(Box::new(xs.clone().into_iter()))
            }
        }
    }
}

pub(crate) enum RangeOutputIter {
    External(usize, bool, Box<dyn Iterator<Item = u64>>),
    Internal(Box<dyn Iterator<Item = (usize, u64)>>),
    ExternalHex(usize, bool, Box<dyn Iterator<Item = u64>>),
    InternalHex(Box<dyn Iterator<Item = (usize, u64)>>),
}

impl Iterator for RangeOutputIter {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            RangeOutputIter::External(prefix, same_prefix_len, xs) => xs
                .next()
                .map(|x| format_num_prefix(x, *prefix, *same_prefix_len)),
            RangeOutputIter::Internal(xs) => xs
                .next()
                .map(|(prefix, x)| format_num_prefix(x, prefix, true)),
            RangeOutputIter::ExternalHex(prefix, same_prefix_len, xs) => xs
                .next()
                .map(|x| format_hex_prefix(x, *prefix, *same_prefix_len)),
            RangeOutputIter::InternalHex(xs) => xs
                .next()
                .map(|(prefix, x)| format_hex_prefix(x, prefix, true)),
        }
    }
}

pub(crate) fn format_num_prefix(num: u64, prefix: usize, same_prefix_len: bool) -> String {
    let width = if same_prefix_len {
        prefix + num.to_string().len()
    } else {
        prefix + 1
    };

    format!("{num:0>width$}")
}

pub(crate) fn format_hex_prefix(num: u64, prefix: usize, same_prefix_len: bool) -> String {
    let s = format!("{:x}", num);
    let width = if same_prefix_len {
        prefix + s.len()
    } else {
        prefix + 1
    };

    if width <= s.len() {
        s
    } else {
        let mut out = String::with_capacity(width);
        out.extend(std::iter::repeat_n('0', width - s.len()));
        out.push_str(&s);
        out
    }
}

#[derive(Debug, Clone)]
pub(crate) enum Part {
    String(String),
    Range(Vec<RangeOutput>),
}

impl Cardinality for Part {
    fn cardinality(&self) -> u64 {
        match self {
            Part::String(_) => 1,
            Part::Range(range_outputs) => {
                range_outputs.iter().map(Cardinality::cardinality).product()
            }
        }
    }
}

impl Cardinality for Vec<Part> {
    fn cardinality(&self) -> u64 {
        self.iter().map(Cardinality::cardinality).product()
    }
}

impl Part {
    pub(crate) fn get_ranges(&self) -> Option<&Vec<RangeOutput>> {
        match self {
            Part::Range(xs) => Some(xs),
            _ => None,
        }
    }
}

pub(crate) fn flatten_ranges(xs: &[RangeOutput]) -> Vec<String> {
    xs.iter().flat_map(|x| x.iter()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_debug_snapshot;

    #[test]
    fn test_range_output_range_iter() {
        assert_debug_snapshot!(
            RangeOutput::Range(3, false, 1, 10)
                .iter()
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_range_output_disjoint_iter() {
        assert_debug_snapshot!(
            RangeOutput::Disjoint(vec![(0, 1), (1, 10)])
                .iter()
                .collect::<Vec<_>>()
        );
    }
}
