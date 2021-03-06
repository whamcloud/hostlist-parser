// Copyright (c) 2019 DDN. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file.

#[derive(Debug, Clone)]
pub enum RangeOutput {
    Range(usize, u64, u64),
    Disjoint(Vec<u64>),
}

impl RangeOutput {
    pub fn iter(&self) -> RangeOutputIter {
        match self {
            RangeOutput::Range(prefix, start, end) => RangeOutputIter {
                prefix: *prefix,
                iterator: Box::new(*start..=*end),
            },
            RangeOutput::Disjoint(xs) => RangeOutputIter {
                prefix: 0,
                iterator: Box::new(xs.clone().into_iter()),
            },
        }
    }
}

pub struct RangeOutputIter {
    prefix: usize,
    iterator: Box<dyn Iterator<Item = u64>>,
}

impl Iterator for RangeOutputIter {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        self.iterator
            .next()
            .map(|x| format_num_prefix(x, self.prefix))
    }
}

pub fn format_num_prefix(num: u64, prefix: usize) -> String {
    format!("{:0>width$}", num, width = prefix + 1)
}

#[derive(Debug, Clone)]
pub enum Part {
    String(String),
    Range(Vec<RangeOutput>),
}

impl Part {
    pub fn get_ranges(&self) -> Option<&Vec<RangeOutput>> {
        match self {
            Part::Range(xs) => Some(xs),
            _ => None,
        }
    }
}

pub fn flatten_ranges(xs: &[RangeOutput]) -> Vec<String> {
    xs.iter().map(|x| x.iter()).flatten().collect()
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
        assert_debug_snapshot!(RangeOutput::Disjoint(vec![1, 10])
            .iter()
            .collect::<Vec<_>>());
    }
}
