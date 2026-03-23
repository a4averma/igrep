use std::collections::HashMap;

use crate::types::NgramHash;

/// Slide a 3-byte window over input, collect all trigrams (including duplicates).
pub fn extract_trigrams(input: &[u8]) -> Vec<[u8; 3]> {
    if input.len() < 3 {
        return Vec::new();
    }
    input.windows(3).map(|w| [w[0], w[1], w[2]]).collect()
}

/// CRC32 hash of 3 bytes using crc32fast.
pub fn trigram_to_hash(trigram: &[u8; 3]) -> NgramHash {
    crc32fast::hash(trigram)
}

/// A simple single-byte hash used for next-char and position masks.
#[inline]
fn hash_byte(b: u8) -> u8 {
    // Use the low bits of a crc32 of the single byte for consistency
    (crc32fast::hash(&[b]) & 0xFF) as u8
}

/// A trigram with probabilistic position and next-character masks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrigramWithMasks {
    pub trigram: [u8; 3],
    pub next_mask: u8,
    pub loc_mask: u8,
}

/// Extract trigrams with probabilistic masks.
///
/// For each trigram at position `p`:
/// - `loc_mask |= 1 << (p % 8)` — records which positions (mod 8) the trigram appears at
/// - `next_mask |= 1 << (hash_byte(input[p+3]) % 8)` — if `p+3 < len`, records a hash of the
///   byte following the trigram
///
/// Trigrams are grouped by value; masks are merged with OR.
pub fn extract_trigrams_with_masks(input: &[u8]) -> Vec<TrigramWithMasks> {
    if input.len() < 3 {
        return Vec::new();
    }

    let mut map: HashMap<[u8; 3], (u8, u8)> = HashMap::new();

    for p in 0..=(input.len() - 3) {
        let tri = [input[p], input[p + 1], input[p + 2]];
        let entry = map.entry(tri).or_insert((0u8, 0u8));

        // loc_mask: record position mod 8
        entry.1 |= 1 << (p % 8);

        // next_mask: record hash of next byte if it exists
        if p + 3 < input.len() {
            entry.0 |= 1 << (hash_byte(input[p + 3]) % 8);
        }
    }

    map.into_iter()
        .map(|(trigram, (next_mask, loc_mask))| TrigramWithMasks {
            trigram,
            next_mask,
            loc_mask,
        })
        .collect()
}

/// Check whether two trigrams could be adjacent based on their position masks.
///
/// If trigram A is at position p and trigram B is at position p+1,
/// then bit (p%8) is set in mask_a and bit ((p+1)%8) is set in mask_b.
/// We check if shifting mask_a left by 1 (with wrap from bit 7 to bit 0)
/// overlaps with mask_b.
pub fn check_adjacency(mask_a: u8, mask_b: u8) -> bool {
    let shifted = (mask_a << 1) | (mask_a >> 7);
    (shifted & mask_b) != 0
}

/// Check whether a next-character mask is consistent with an expected character.
///
/// Returns true if the bit corresponding to `hash_byte(expected_char) % 8`
/// is set in `next_mask`.
pub fn check_next_char(next_mask: u8, expected_char: u8) -> bool {
    let bit = hash_byte(expected_char) % 8;
    (next_mask & (1 << bit)) != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_trigrams_the_cat_sat() {
        let input = b"the cat sat";
        let trigrams = extract_trigrams(input);
        // 11 bytes => 11 - 2 = 9 trigram positions
        assert_eq!(trigrams.len(), 9);
        assert_eq!(&trigrams[0], b"the");
        assert_eq!(&trigrams[1], b"he ");
        assert_eq!(&trigrams[2], b"e c");
        assert_eq!(&trigrams[3], b" ca");
        assert_eq!(&trigrams[4], b"cat");
        assert_eq!(&trigrams[5], b"at ");
        assert_eq!(&trigrams[6], b"t s");
        assert_eq!(&trigrams[7], b" sa");
        assert_eq!(&trigrams[8], b"sat");
    }

    #[test]
    fn test_extract_trigrams_with_duplicates() {
        let input = b"aba aba";
        let trigrams = extract_trigrams(input);
        // 7 bytes => 5 trigrams
        assert_eq!(trigrams.len(), 5);
        // "aba" appears at position 0 and position 4
        let aba_count = trigrams.iter().filter(|t| *t == b"aba").count();
        assert_eq!(aba_count, 2);
    }

    #[test]
    fn test_extract_trigrams_empty_input() {
        assert!(extract_trigrams(b"").is_empty());
    }

    #[test]
    fn test_extract_trigrams_short_input() {
        assert!(extract_trigrams(b"ab").is_empty());
        assert!(extract_trigrams(b"a").is_empty());
    }

    #[test]
    fn test_extract_trigrams_exact_three() {
        let trigrams = extract_trigrams(b"abc");
        assert_eq!(trigrams.len(), 1);
        assert_eq!(&trigrams[0], b"abc");
    }

    #[test]
    fn test_trigram_to_hash_deterministic() {
        let tri = [b't', b'h', b'e'];
        let h1 = trigram_to_hash(&tri);
        let h2 = trigram_to_hash(&tri);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_trigram_to_hash_different_inputs() {
        let h1 = trigram_to_hash(b"the");
        let h2 = trigram_to_hash(b"cat");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_extract_trigrams_with_masks_basic() {
        let input = b"the cat sat";
        let results = extract_trigrams_with_masks(input);
        // All 9 trigrams are unique in "the cat sat", so we get 9 entries
        assert_eq!(results.len(), 9);
    }

    #[test]
    fn test_extract_trigrams_with_masks_grouping() {
        // "aba aba" has "aba" at positions 0 and 4
        let input = b"aba aba";
        let results = extract_trigrams_with_masks(input);

        let aba = results.iter().find(|r| r.trigram == *b"aba").unwrap();
        // loc_mask: bit 0 (pos 0) and bit 4 (pos 4) => 0b00010001 = 0x11
        assert_eq!(aba.loc_mask, (1 << 0) | (1 << 4));
    }

    #[test]
    fn test_extract_trigrams_with_masks_empty() {
        assert!(extract_trigrams_with_masks(b"").is_empty());
        assert!(extract_trigrams_with_masks(b"ab").is_empty());
    }

    #[test]
    fn test_extract_trigrams_with_masks_loc_mask() {
        // "abcdef" => trigrams at positions 0,1,2,3
        // "abc" at pos 0 => loc_mask has bit 0
        let input = b"abcdef";
        let results = extract_trigrams_with_masks(input);

        let abc = results.iter().find(|r| r.trigram == *b"abc").unwrap();
        assert_eq!(abc.loc_mask, 1 << 0);

        let bcd = results.iter().find(|r| r.trigram == *b"bcd").unwrap();
        assert_eq!(bcd.loc_mask, 1 << 1);

        let cde = results.iter().find(|r| r.trigram == *b"cde").unwrap();
        assert_eq!(cde.loc_mask, 1 << 2);

        let def = results.iter().find(|r| r.trigram == *b"def").unwrap();
        assert_eq!(def.loc_mask, 1 << 3);
    }

    #[test]
    fn test_extract_trigrams_with_masks_next_mask() {
        // "abcd": trigram "abc" at pos 0, next byte is 'd'
        let input = b"abcd";
        let results = extract_trigrams_with_masks(input);

        let abc = results.iter().find(|r| r.trigram == *b"abc").unwrap();
        let expected_bit = hash_byte(b'd') % 8;
        assert_ne!(abc.next_mask, 0);
        assert!(abc.next_mask & (1 << expected_bit) != 0);

        // Last trigram "bcd" at pos 1 has no next byte => next_mask stays 0
        let bcd = results.iter().find(|r| r.trigram == *b"bcd").unwrap();
        assert_eq!(bcd.next_mask, 0);
    }

    #[test]
    fn test_adjacency_check_adjacent() {
        // "the" at pos 0 => loc_mask = 0b00000001
        // "he " at pos 1 => loc_mask = 0b00000010
        let mask_a = 1u8 << 0; // pos 0
        let mask_b = 1u8 << 1; // pos 1
        assert!(check_adjacency(mask_a, mask_b));
    }

    #[test]
    fn test_adjacency_check_not_adjacent() {
        // pos 0 and pos 3 are not adjacent
        let mask_a = 1u8 << 0;
        let mask_b = 1u8 << 3;
        assert!(!check_adjacency(mask_a, mask_b));
    }

    #[test]
    fn test_adjacency_check_wrap_around() {
        // pos 7 wraps to pos 0
        let mask_a = 1u8 << 7;
        let mask_b = 1u8 << 0;
        assert!(check_adjacency(mask_a, mask_b));
    }

    #[test]
    fn test_adjacency_check_multiple_positions() {
        // A at positions 0 and 4, B at positions 1 and 5
        let mask_a = (1u8 << 0) | (1u8 << 4);
        let mask_b = (1u8 << 1) | (1u8 << 5);
        assert!(check_adjacency(mask_a, mask_b));
    }

    #[test]
    fn test_adjacency_from_real_trigrams() {
        // Verify adjacency using actual extracted masks from "the cat sat"
        let input = b"the cat sat";
        let results = extract_trigrams_with_masks(input);

        let the = results.iter().find(|r| r.trigram == *b"the").unwrap();
        let he_space = results.iter().find(|r| r.trigram == *b"he ").unwrap();
        assert!(check_adjacency(the.loc_mask, he_space.loc_mask));
    }

    #[test]
    fn test_check_next_char_basic() {
        // "the " => trigram "the" at pos 0, next char is ' '
        let input = b"the ";
        let results = extract_trigrams_with_masks(input);

        let the = results.iter().find(|r| r.trigram == *b"the").unwrap();
        assert!(check_next_char(the.next_mask, b' '));
    }

    #[test]
    fn test_check_next_char_no_next() {
        // "the" => trigram "the" at pos 0, no next byte => next_mask = 0
        let input = b"the";
        let results = extract_trigrams_with_masks(input);

        let the = results.iter().find(|r| r.trigram == *b"the").unwrap();
        assert_eq!(the.next_mask, 0);
        // Any check against a zero mask should fail
        assert!(!check_next_char(the.next_mask, b' '));
        assert!(!check_next_char(the.next_mask, b'x'));
    }

    #[test]
    fn test_hash_byte_deterministic() {
        let h1 = hash_byte(b'a');
        let h2 = hash_byte(b'a');
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_adjacency_zero_masks() {
        assert!(!check_adjacency(0, 0));
        assert!(!check_adjacency(0, 0xFF));
        assert!(!check_adjacency(0xFF, 0));
    }

    #[test]
    fn test_check_next_char_zero_mask() {
        assert!(!check_next_char(0, b'a'));
        assert!(!check_next_char(0, b'z'));
    }

    #[test]
    fn test_adjacency_check_all_positions() {
        // For each position p in 0..8, check that p and p+1 (mod 8) are adjacent
        for p in 0u8..8 {
            let mask_a = 1u8 << p;
            let mask_b = 1u8 << ((p + 1) % 8);
            assert!(
                check_adjacency(mask_a, mask_b),
                "Position {} should be adjacent to position {}",
                p,
                (p + 1) % 8
            );
        }
    }

    #[test]
    fn test_adjacency_non_adjacent_all_positions() {
        // For each position p, check that p and p+2 (mod 8) are NOT adjacent
        // (unless there's overlap from other bits, which there isn't here)
        for p in 0u8..8 {
            let mask_a = 1u8 << p;
            let mask_b = 1u8 << ((p + 2) % 8);
            assert!(
                !check_adjacency(mask_a, mask_b),
                "Position {} should NOT be adjacent to position {}",
                p,
                (p + 2) % 8
            );
        }
    }
}
