/// Bigram frequency table for the sparse n-gram algorithm.
///
/// Each byte is assigned a "rarity score" (0–255) reflecting how common it is
/// in typical source code. The bigram weight for a pair `(b1, b2)` combines
/// both rarity scores into a single `u32` in the range `0..=65535`.
///
/// **Invariant**: common source-code pairs like `e `, ` t`, `th` produce low
/// weights, while rare pairs like `qz`, `#!`, or control-char combinations
/// produce high weights.  The sparse n-gram algorithm uses high-weight bigrams
/// as segment boundaries to generate longer, more selective n-grams.

/// Rarity score for every byte value (0–255).
///
/// Lower score = more common in source code.  The table is hand-tuned from
/// observed byte frequencies across large source-code corpora.
///
/// Tiers (approximate):
///   0–15   most common: space, e, t, a, o, i, n, s, r, l, newline, etc.
///  16–39   common letters / digits / basic punctuation
///  40–79   less common printable ASCII
///  80–119  rare printable ASCII
/// 120–199  non-printable / uncommon control chars
/// 200–255  very rare: high-ASCII, exotic control chars
static BYTE_RARITY: [u8; 256] = {
    let mut table = [200u8; 256]; // default: rare

    // ----- Tier 0: most common (0–15) -----
    table[b' ' as usize] = 0; // space — most common byte in code
    table[b'e' as usize] = 1;
    table[b't' as usize] = 2;
    table[b'a' as usize] = 3;
    table[b'o' as usize] = 4;
    table[b'i' as usize] = 5;
    table[b'n' as usize] = 6;
    table[b's' as usize] = 7;
    table[b'r' as usize] = 8;
    table[b'l' as usize] = 9;
    table[b'\n' as usize] = 10; // newline
    table[b'c' as usize] = 11;
    table[b'd' as usize] = 12;
    table[b'u' as usize] = 13;
    table[b'p' as usize] = 14;
    table[b'm' as usize] = 15;

    // ----- Tier 1: common (16–39) -----
    table[b'f' as usize] = 16;
    table[b'h' as usize] = 17;
    table[b'g' as usize] = 18;
    table[b'b' as usize] = 19;
    table[b'y' as usize] = 20;
    table[b'v' as usize] = 21;
    table[b'w' as usize] = 22;
    table[b'k' as usize] = 23;
    table[b'_' as usize] = 24; // underscore — very common in code
    table[b'(' as usize] = 25;
    table[b')' as usize] = 25;
    table[b'=' as usize] = 26;
    table[b';' as usize] = 27;
    table[b'{' as usize] = 28;
    table[b'}' as usize] = 28;
    table[b',' as usize] = 29;
    table[b'.' as usize] = 30;
    table[b':' as usize] = 31;
    table[b'"' as usize] = 32;
    table[b'\'' as usize] = 33;
    table[b'/' as usize] = 34;
    table[b'x' as usize] = 35;
    table[b'0' as usize] = 36;
    table[b'1' as usize] = 37;
    table[b'2' as usize] = 38;
    table[b'\t' as usize] = 39; // tab

    // ----- Tier 2: less common (40–79) -----
    table[b'j' as usize] = 40;
    table[b'q' as usize] = 41;
    table[b'z' as usize] = 42;
    table[b'3' as usize] = 43;
    table[b'4' as usize] = 44;
    table[b'5' as usize] = 45;
    table[b'6' as usize] = 46;
    table[b'7' as usize] = 47;
    table[b'8' as usize] = 48;
    table[b'9' as usize] = 49;
    table[b'<' as usize] = 50;
    table[b'>' as usize] = 50;
    table[b'[' as usize] = 51;
    table[b']' as usize] = 51;
    table[b'-' as usize] = 52;
    table[b'+' as usize] = 53;
    table[b'*' as usize] = 54;
    table[b'&' as usize] = 55;
    table[b'|' as usize] = 56;
    table[b'!' as usize] = 57;
    table[b'#' as usize] = 58;
    table[b'@' as usize] = 59;
    table[b'%' as usize] = 60;
    table[b'\\' as usize] = 61;
    table[b'\r' as usize] = 62; // carriage return

    // ----- Tier 3: uppercase letters (63–88) -----
    // Uppercase letters are less common than lowercase in most code.
    table[b'S' as usize] = 63;
    table[b'T' as usize] = 64;
    table[b'C' as usize] = 65;
    table[b'I' as usize] = 66;
    table[b'A' as usize] = 67;
    table[b'E' as usize] = 68;
    table[b'R' as usize] = 69;
    table[b'N' as usize] = 70;
    table[b'O' as usize] = 71;
    table[b'L' as usize] = 72;
    table[b'P' as usize] = 73;
    table[b'D' as usize] = 74;
    table[b'M' as usize] = 75;
    table[b'F' as usize] = 76;
    table[b'U' as usize] = 77;
    table[b'H' as usize] = 78;
    table[b'G' as usize] = 79;
    table[b'B' as usize] = 80;
    table[b'W' as usize] = 81;
    table[b'V' as usize] = 82;
    table[b'Y' as usize] = 83;
    table[b'K' as usize] = 84;
    table[b'X' as usize] = 85;
    table[b'J' as usize] = 86;
    table[b'Q' as usize] = 87;
    table[b'Z' as usize] = 88;

    // ----- Tier 4: rare printable (89–119) -----
    table[b'$' as usize] = 90;
    table[b'^' as usize] = 95;
    table[b'~' as usize] = 100;
    table[b'`' as usize] = 105;
    table[b'?' as usize] = 55; // fairly common in code (ternary, URLs)

    // ----- Tier 5: control characters (120–199) -----
    // Most control chars already default to 200; bring a few down slightly.
    table[0x00] = 180; // NUL — can appear in binary
    table[0x1B] = 170; // ESC
    table[0x7F] = 190; // DEL

    // Remaining control chars (0x01–0x08, 0x0E–0x1A, 0x1C–0x1F) stay at 200.
    // High-ASCII (0x80–0xFF) stays at 200 — these are rare in source code.

    table
};

/// Return the bigram weight for the byte pair `(b1, b2)`.
///
/// The weight is in the range `0..=65535` (a full `u16` range packed in `u32`).
/// Lower values indicate common source-code pairs; higher values indicate rare
/// pairs.  The sparse n-gram algorithm picks high-weight positions as segment
/// boundaries.
///
/// The function is deterministic and allocation-free (pure table lookup + arithmetic).
#[inline]
pub fn bigram_weight(b1: u8, b2: u8) -> u32 {
    let r1 = BYTE_RARITY[b1 as usize] as u32;
    let r2 = BYTE_RARITY[b2 as usize] as u32;
    // r1 * 256 + r2 naturally spans 0..=65535 when both rarity values are 0..=255.
    r1 * 256 + r2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn common_pairs_have_low_weight() {
        // "th", "he", "in", "er", "an" — among the most common English bigrams,
        // also very common in source code identifiers.
        let common_pairs: &[(u8, u8)] = &[
            (b't', b'h'),
            (b'h', b'e'),
            (b'i', b'n'),
            (b'e', b'r'),
            (b'a', b'n'),
            (b'e', b' '),
            (b' ', b't'),
        ];

        for &(a, b) in common_pairs {
            let w = bigram_weight(a, b);
            // Both bytes have rarity < 20, so weight < 20*256 + 20 = 5140.
            assert!(
                w < 5200,
                "common pair ({:?}, {:?}) should have low weight, got {}",
                a as char,
                b as char,
                w
            );
        }
    }

    #[test]
    fn rare_pairs_have_high_weight() {
        // Rare pairs: control chars, high-ASCII combos
        let rare_pairs: &[(u8, u8)] = &[
            (0xFF, 0xFE), // high-ASCII
            (0x01, 0x02), // control chars
            (0x80, 0x90), // high-ASCII
        ];

        for &(a, b) in rare_pairs {
            let w = bigram_weight(a, b);
            // Both bytes have rarity >= 200, so weight >= 200*256 + 200 = 51400.
            assert!(
                w > 50000,
                "rare pair (0x{:02X}, 0x{:02X}) should have high weight, got {}",
                a,
                b,
                w
            );
        }
    }

    #[test]
    fn common_less_than_rare() {
        // The core invariant: common code bigrams weigh less than rare ones.
        let common = bigram_weight(b'e', b' ');
        let rare_qz = bigram_weight(b'q', b'z');
        let rare_shebang = bigram_weight(b'#', b'!');

        assert!(
            common < rare_qz,
            "e-space ({}) should be less than q-z ({})",
            common,
            rare_qz
        );
        assert!(
            common < rare_shebang,
            "e-space ({}) should be less than #! ({})",
            common,
            rare_shebang
        );
    }

    #[test]
    fn space_letter_common() {
        // Space followed by any common letter should have very low weight.
        let common_letters = b"etaoinsrlcdu";
        for &ch in common_letters {
            let w = bigram_weight(b' ', ch);
            // space rarity=0, letter rarity<16, so weight = 0*256 + rarity < 16.
            assert!(
                w < 16,
                "space + {:?} should have very low weight, got {}",
                ch as char,
                w
            );
        }
    }

    #[test]
    fn all_entries_populated() {
        // Every possible byte pair must produce a weight in 0..=65535.
        let mut min_w = u32::MAX;
        let mut max_w = 0u32;

        for b1 in 0u8..=255 {
            for b2 in 0u8..=255 {
                let w = bigram_weight(b1, b2);
                assert!(
                    w <= 65535,
                    "weight for (0x{:02X}, 0x{:02X}) = {} exceeds 65535",
                    b1,
                    b2,
                    w
                );
                min_w = min_w.min(w);
                max_w = max_w.max(w);
            }
        }

        // Verify reasonable spread: min should be very low, max very high.
        assert!(min_w < 10, "minimum weight {} is unexpectedly high", min_w);
        assert!(
            max_w > 50000,
            "maximum weight {} is unexpectedly low",
            max_w
        );
    }

    #[test]
    fn deterministic() {
        // Same inputs must always produce the same output.
        let pairs: &[(u8, u8)] = &[
            (0, 0),
            (255, 255),
            (b'a', b'b'),
            (b' ', b'\n'),
            (0x80, 0x7F),
        ];

        for &(a, b) in pairs {
            let w1 = bigram_weight(a, b);
            let w2 = bigram_weight(a, b);
            assert_eq!(
                w1, w2,
                "bigram_weight(0x{:02X}, 0x{:02X}) not deterministic: {} vs {}",
                a, b, w1, w2
            );
        }
    }

    #[test]
    fn weight_ordering_granularity() {
        // Verify specific ordering: very common < somewhat common < rare < very rare
        let very_common = bigram_weight(b' ', b'e'); // rarity 0, 1
        let somewhat_common = bigram_weight(b'(', b')'); // rarity 25, 25
        let rare = bigram_weight(b'Q', b'Z'); // rarity 87, 88
        let very_rare = bigram_weight(0xFF, 0xFF); // rarity 200, 200

        assert!(
            very_common < somewhat_common,
            "space-e ({}) should < parens ({})",
            very_common,
            somewhat_common
        );
        assert!(
            somewhat_common < rare,
            "parens ({}) should < QZ ({})",
            somewhat_common,
            rare
        );
        assert!(
            rare < very_rare,
            "QZ ({}) should < 0xFF-0xFF ({})",
            rare,
            very_rare
        );
    }
}
