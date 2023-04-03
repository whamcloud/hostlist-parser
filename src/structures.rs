// Copyright (c) 2022 DDN. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file.

#[derive(Debug, Clone)]
pub(crate) enum RangeOutput {
    Range(usize, u64, u64),
    RangeReversed(usize, u64, u64),
    Disjoint(Vec<(usize, u64)>),
}

impl RangeOutput {
    pub(crate) fn iter(&self) -> RangeOutputIter {
        match self {
            RangeOutput::Range(prefix, start, end) => {
                RangeOutputIter::External(*prefix, Box::new(*start..=*end))
            }
            RangeOutput::RangeReversed(prefix, end, start) => {
                RangeOutputIter::External(*prefix, Box::new((*end..=*start).rev()))
            }
            RangeOutput::Disjoint(xs) => {
                RangeOutputIter::Internal(Box::new(xs.clone().into_iter()))
            }
        }
    }
}

pub(crate) enum RangeOutputIter {
    External(usize, Box<dyn Iterator<Item = u64>>),
    Internal(Box<dyn Iterator<Item = (usize, u64)>>),
}

impl Iterator for RangeOutputIter {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            RangeOutputIter::External(prefix, xs) => {
                xs.next().map(|x| format_num_prefix(x, *prefix))
            }
            RangeOutputIter::Internal(xs) => {
                xs.next().map(|(prefix, x)| format_num_prefix(x, prefix))
            }
        }
    }
}

pub(crate) fn format_num_prefix(num: u64, prefix: usize) -> String {
    format!("{:0>width$}", num, width = prefix + 1)
}

#[derive(Debug, Clone)]
pub(crate) enum Part {
    String(String),
    Range(Vec<RangeOutput>),
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
        assert_debug_snapshot!(RangeOutput::Range(3, 1, 10).iter().collect::<Vec<_>>());
    }

    #[test]
    fn test_range_output_disjoint_iter() {
        assert_debug_snapshot!(RangeOutput::Disjoint(vec![(0, 1), (1, 10)])
            .iter()
            .collect::<Vec<_>>());
    }
}
