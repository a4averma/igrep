use crate::frequency_table::bigram_weight;
use crate::types::{IndexConfig, NgramHash};
use std::collections::VecDeque;

pub fn build_all(input: &[u8], weight_fn: &dyn Fn(u8, u8) -> u32, consumer: &mut dyn FnMut(&[u8])) {
    if input.len() < 2 {
        return;
    }
    if input.len() == 2 {
        consumer(input);
        return;
    }

    #[derive(Clone, Copy)]
    struct WeightAndPos {
        weight: u32,
        pos: usize,
    }

    let mut stack: Vec<WeightAndPos> = Vec::with_capacity(input.len() - 1);

    for i in 0..=(input.len() - 2) {
        let current = WeightAndPos {
            weight: weight_fn(input[i], input[i + 1]),
            pos: i,
        };

        while let Some(top) = stack.last().copied() {
            if current.weight <= top.weight {
                break;
            }

            consumer(&input[top.pos..(i + 2)]);

            while stack.len() > 1 && stack[stack.len() - 1].weight == stack[stack.len() - 2].weight
            {
                stack.pop();
            }
            stack.pop();
        }

        if let Some(top) = stack.last().copied() {
            consumer(&input[top.pos..(i + 2)]);
        }

        stack.push(current);
    }
}

#[inline]
pub fn hash_ngram(ngram: &[u8]) -> NgramHash {
    crc32fast::hash(ngram)
}

pub fn build_all_ngrams(input: &[u8], config: &IndexConfig) -> Vec<Vec<u8>> {
    let mut ngrams = Vec::new();
    build_all(input, &bigram_weight, &mut |ngram| {
        if ngram.len() <= config.max_ngram_length {
            ngrams.push(ngram.to_vec());
        }
    });
    ngrams
}

pub fn build_covering(
    input: &[u8],
    weight_fn: &dyn Fn(u8, u8) -> u32,
    max_ngram_length: usize,
    consumer: &mut dyn FnMut(&[u8]),
) {
    if input.len() < 2 {
        return;
    }
    if input.len() == 2 {
        if max_ngram_length >= 2 {
            consumer(input);
        }
        return;
    }

    #[derive(Clone, Copy)]
    struct WeightAndPos {
        weight: u32,
        pos: usize,
    }

    let mut deque: VecDeque<WeightAndPos> = VecDeque::with_capacity(input.len() - 1);

    for i in 0..=(input.len() - 2) {
        let current = WeightAndPos {
            weight: weight_fn(input[i], input[i + 1]),
            pos: i,
        };

        if deque.len() > 1 && i - deque[0].pos + 3 >= max_ngram_length {
            consumer(&input[deque[0].pos..(deque[1].pos + 2)]);
            deque.pop_front();
        }

        while let Some(back) = deque.back().copied() {
            if current.weight <= back.weight {
                break;
            }

            if let Some(front) = deque.front().copied() {
                if front.weight == back.weight {
                    consumer(&input[back.pos..(i + 2)]);

                    while deque.len() > 1 {
                        let last_position = deque.back().unwrap().pos + 2;
                        deque.pop_back();
                        let new_back = deque.back().unwrap();
                        consumer(&input[new_back.pos..last_position]);
                    }
                }
            }

            deque.pop_back();
        }

        deque.push_back(current);
    }

    while deque.len() > 1 {
        let last_position = deque.back().unwrap().pos + 2;
        deque.pop_back();
        let new_back = deque.back().unwrap();
        consumer(&input[new_back.pos..last_position]);
    }
}

pub fn build_covering_ngrams(input: &[u8], config: &IndexConfig) -> Vec<Vec<u8>> {
    let mut ngrams = Vec::new();
    build_covering(
        input,
        &bigram_weight,
        config.max_ngram_length,
        &mut |ngram| {
            ngrams.push(ngram.to_vec());
        },
    );
    ngrams
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn collect_ngrams(input: &[u8]) -> Vec<Vec<u8>> {
        let mut out = Vec::new();
        build_all(input, &bigram_weight, &mut |ngram| out.push(ngram.to_vec()));
        out
    }

    fn collect_covering_ngrams(input: &[u8], max_ngram_length: usize) -> Vec<Vec<u8>> {
        let mut out = Vec::new();
        build_covering(input, &bigram_weight, max_ngram_length, &mut |ngram| {
            out.push(ngram.to_vec())
        });
        out
    }

    fn is_subslice(haystack: &[u8], needle: &[u8]) -> bool {
        if needle.is_empty() || needle.len() > haystack.len() {
            return false;
        }
        haystack.windows(needle.len()).any(|w| w == needle)
    }

    #[test]
    fn empty_input_emits_no_ngrams() {
        let out = collect_ngrams(b"");
        assert!(out.is_empty());
    }

    #[test]
    fn one_char_input_emits_no_ngrams() {
        let out = collect_ngrams(b"a");
        assert!(out.is_empty());
    }

    #[test]
    fn two_chars_emit_exactly_one_bigram() {
        let out = collect_ngrams(b"ab");
        assert_eq!(out, vec![b"ab".to_vec()]);
    }

    #[test]
    fn covering_two_chars_emit_exactly_one_bigram() {
        let out = collect_covering_ngrams(b"ab", 16);
        assert_eq!(out, vec![b"ab".to_vec()]);
    }

    #[test]
    fn covering_short_inputs_produce_valid_covering_set() {
        let cases: &[&[u8]] = &[b"", b"a", b"ab", b"abc", b"abcd"];

        for input in cases {
            let out = collect_covering_ngrams(input, 16);
            if input.len() < 2 {
                assert!(out.is_empty());
                continue;
            }
            for ngram in out {
                assert!(ngram.len() >= 2);
                assert!(is_subslice(input, &ngram));
            }
        }
    }

    #[test]
    fn covering_count_is_never_more_than_build_all() {
        let input = b"abcdefghijk";
        let covering = collect_covering_ngrams(input, 16);
        let all = collect_ngrams(input);
        assert!(covering.len() <= all.len());
    }

    #[test]
    fn build_covering_ngrams_respects_max_ngram_length() {
        let input = b"abcdef";
        let mut config = IndexConfig::default();
        config.max_ngram_length = 3;

        let out = build_covering_ngrams(input, &config);
        assert!(!out.is_empty());
        assert!(out.iter().all(|n| n.len() <= 3));
    }

    #[test]
    fn abcdef_outputs_are_substrings_and_len_at_least_two() {
        let input = b"abcdef";
        let out = collect_ngrams(input);

        assert!(!out.is_empty());
        for ngram in out {
            assert!(ngram.len() >= 2);
            assert!(is_subslice(input, &ngram));
        }
    }

    #[test]
    fn hash_ngram_is_deterministic() {
        let ngram = b"deterministic-ngram";
        assert_eq!(hash_ngram(ngram), hash_ngram(ngram));
        assert_ne!(hash_ngram(b"abc"), hash_ngram(b"abd"));
    }

    #[test]
    fn build_all_ngrams_respects_max_ngram_length() {
        let input = b"abcdef";
        let mut config = IndexConfig::default();
        config.max_ngram_length = 3;

        let out = build_all_ngrams(input, &config);
        assert!(!out.is_empty());
        assert!(out.iter().all(|n| n.len() <= 3));
    }

    proptest! {
        #[test]
        fn prop_output_count_bound(input in proptest::collection::vec(any::<u8>(), 3..256)) {
            let out = collect_ngrams(&input);
            prop_assert!(out.len() <= 2 * (input.len() - 2));
        }

        #[test]
        fn prop_every_ngram_is_substring(input in proptest::collection::vec(any::<u8>(), 0..256)) {
            let out = collect_ngrams(&input);
            for ngram in out {
                prop_assert!(ngram.len() >= 2);
                prop_assert!(is_subslice(&input, &ngram));
            }
        }

        #[test]
        fn prop_covering_count_bound(input in proptest::collection::vec(any::<u8>(), 3..256)) {
            let out = collect_covering_ngrams(&input, 16);
            prop_assert!(out.len() <= input.len() - 2);
        }

        #[test]
        fn prop_every_covering_ngram_is_substring(input in proptest::collection::vec(any::<u8>(), 0..256)) {
            let out = collect_covering_ngrams(&input, 16);
            for ngram in out {
                prop_assert!(ngram.len() >= 2);
                prop_assert!(is_subslice(&input, &ngram));
            }
        }

        #[test]
        fn prop_covering_is_subset_of_all(input in proptest::collection::vec(any::<u8>(), 0..256)) {
            let covering = collect_covering_ngrams(&input, 16);
            let all = collect_ngrams(&input);
            for ngram in covering {
                prop_assert!(all.iter().any(|candidate| candidate == &ngram));
            }
        }
    }
}
