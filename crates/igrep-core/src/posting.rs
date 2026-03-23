use crate::types::DocId;

/// Encode a u32 value as a varint (LEB128-style: 7 bits per byte, MSB=continuation).
pub fn encode_varint(value: u32, buf: &mut Vec<u8>) {
    let mut v = value;
    loop {
        let mut byte = (v & 0x7F) as u8;
        v >>= 7;
        if v != 0 {
            byte |= 0x80; // set continuation bit
        }
        buf.push(byte);
        if v == 0 {
            break;
        }
    }
}

/// Decode a varint from `data` starting at `*pos`. Advances `*pos` past the consumed bytes.
/// Returns `None` if the data is truncated or invalid.
pub fn decode_varint(data: &[u8], pos: &mut usize) -> Option<u32> {
    let mut result: u32 = 0;
    let mut shift: u32 = 0;
    loop {
        if *pos >= data.len() {
            return None;
        }
        let byte = data[*pos];
        *pos += 1;
        result |= ((byte & 0x7F) as u32) << shift;
        if byte & 0x80 == 0 {
            return Some(result);
        }
        shift += 7;
        if shift >= 35 {
            // u32 can have at most 5 bytes in varint (ceil(32/7)*7 = 35)
            return None;
        }
    }
}

/// Encode a posting list using delta encoding with varint compression.
/// The format is: varint(count) followed by varint-encoded deltas.
/// Input doc_ids are sorted before encoding.
pub fn encode_posting_list(doc_ids: &[DocId], buf: &mut Vec<u8>) {
    let mut sorted = doc_ids.to_vec();
    sorted.sort_unstable();
    sorted.dedup();

    // Prepend count
    encode_varint(sorted.len() as u32, buf);

    // Encode deltas
    let mut prev: u32 = 0;
    for &id in &sorted {
        let delta = id - prev;
        encode_varint(delta, buf);
        prev = id;
    }
}

/// Decode a delta-encoded posting list from `data` starting at `*pos`.
/// Returns the reconstructed sorted list of DocIds.
pub fn decode_posting_list(data: &[u8], pos: &mut usize) -> Vec<DocId> {
    let count = match decode_varint(data, pos) {
        Some(c) => c as usize,
        None => return Vec::new(),
    };

    let mut result = Vec::with_capacity(count);
    let mut acc: u32 = 0;
    for _ in 0..count {
        match decode_varint(data, pos) {
            Some(delta) => {
                acc += delta;
                result.push(acc);
            }
            None => break,
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── Varint Tests ───

    #[test]
    fn test_varint_zero() {
        let mut buf = Vec::new();
        encode_varint(0, &mut buf);
        assert_eq!(buf, vec![0x00]);

        let mut pos = 0;
        assert_eq!(decode_varint(&buf, &mut pos), Some(0));
        assert_eq!(pos, 1);
    }

    #[test]
    fn test_varint_127() {
        let mut buf = Vec::new();
        encode_varint(127, &mut buf);
        assert_eq!(buf, vec![0x7F]);

        let mut pos = 0;
        assert_eq!(decode_varint(&buf, &mut pos), Some(127));
        assert_eq!(pos, 1);
    }

    #[test]
    fn test_varint_128() {
        let mut buf = Vec::new();
        encode_varint(128, &mut buf);
        // 128 = 0b10000000 -> low 7 bits = 0x00 with continuation, then 0x01
        assert_eq!(buf, vec![0x80, 0x01]);

        let mut pos = 0;
        assert_eq!(decode_varint(&buf, &mut pos), Some(128));
        assert_eq!(pos, 2);
    }

    #[test]
    fn test_varint_300() {
        let mut buf = Vec::new();
        encode_varint(300, &mut buf);
        // 300 = 0b100101100 -> low 7: 0b0101100 = 0x2C | 0x80 = 0xAC, then 0b10 = 0x02
        assert_eq!(buf, vec![0xAC, 0x02]);

        let mut pos = 0;
        assert_eq!(decode_varint(&buf, &mut pos), Some(300));
        assert_eq!(pos, 2);
    }

    #[test]
    fn test_varint_u32_max() {
        let mut buf = Vec::new();
        encode_varint(u32::MAX, &mut buf);
        assert_eq!(buf.len(), 5); // ceil(32/7) = 5 bytes

        let mut pos = 0;
        assert_eq!(decode_varint(&buf, &mut pos), Some(u32::MAX));
        assert_eq!(pos, 5);
    }

    #[test]
    fn test_varint_roundtrip_various() {
        let values = [0, 1, 127, 128, 255, 256, 16383, 16384, 1_000_000, u32::MAX];
        for &v in &values {
            let mut buf = Vec::new();
            encode_varint(v, &mut buf);
            let mut pos = 0;
            assert_eq!(
                decode_varint(&buf, &mut pos),
                Some(v),
                "roundtrip failed for {v}"
            );
            assert_eq!(pos, buf.len(), "pos mismatch for {v}");
        }
    }

    #[test]
    fn test_varint_multiple_in_buffer() {
        let mut buf = Vec::new();
        encode_varint(1, &mut buf);
        encode_varint(300, &mut buf);
        encode_varint(u32::MAX, &mut buf);

        let mut pos = 0;
        assert_eq!(decode_varint(&buf, &mut pos), Some(1));
        assert_eq!(decode_varint(&buf, &mut pos), Some(300));
        assert_eq!(decode_varint(&buf, &mut pos), Some(u32::MAX));
        assert_eq!(pos, buf.len());
    }

    #[test]
    fn test_decode_varint_empty() {
        let mut pos = 0;
        assert_eq!(decode_varint(&[], &mut pos), None);
    }

    #[test]
    fn test_decode_varint_truncated() {
        // 0x80 has continuation bit set but no following byte
        let mut pos = 0;
        assert_eq!(decode_varint(&[0x80], &mut pos), None);
    }

    // ─── Delta Encoding Tests ───

    #[test]
    fn test_delta_encoding_known_values() {
        // Input: [7, 18, 19, 22, 25, 63]
        // Deltas: [7, 11, 1, 3, 3, 38]
        let ids: Vec<DocId> = vec![7, 18, 19, 22, 25, 63];
        let mut buf = Vec::new();
        encode_posting_list(&ids, &mut buf);

        // Verify: count=6 (varint 0x06), then deltas 7,11,1,3,3,38 as varints
        let expected: Vec<u8> = vec![
            6,  // count
            7,  // delta: 7
            11, // delta: 11
            1,  // delta: 1
            3,  // delta: 3
            3,  // delta: 3
            38, // delta: 38
        ];
        assert_eq!(buf, expected);
    }

    #[test]
    fn test_posting_list_empty() {
        let ids: Vec<DocId> = vec![];
        let mut buf = Vec::new();
        encode_posting_list(&ids, &mut buf);

        let mut pos = 0;
        let decoded = decode_posting_list(&buf, &mut pos);
        assert!(decoded.is_empty());
        assert_eq!(pos, buf.len());
    }

    #[test]
    fn test_posting_list_single_element() {
        let ids: Vec<DocId> = vec![42];
        let mut buf = Vec::new();
        encode_posting_list(&ids, &mut buf);

        let mut pos = 0;
        let decoded = decode_posting_list(&buf, &mut pos);
        assert_eq!(decoded, vec![42]);
        assert_eq!(pos, buf.len());
    }

    #[test]
    fn test_posting_list_roundtrip() {
        let ids: Vec<DocId> = vec![7, 18, 19, 22, 25, 63];
        let mut buf = Vec::new();
        encode_posting_list(&ids, &mut buf);

        let mut pos = 0;
        let decoded = decode_posting_list(&buf, &mut pos);
        assert_eq!(decoded, ids);
        assert_eq!(pos, buf.len());
    }

    #[test]
    fn test_posting_list_unsorted_input() {
        // encode_posting_list should sort internally
        let ids: Vec<DocId> = vec![63, 7, 25, 18, 22, 19];
        let mut buf = Vec::new();
        encode_posting_list(&ids, &mut buf);

        let mut pos = 0;
        let decoded = decode_posting_list(&buf, &mut pos);
        assert_eq!(decoded, vec![7, 18, 19, 22, 25, 63]);
    }

    #[test]
    fn test_posting_list_with_duplicates() {
        let ids: Vec<DocId> = vec![5, 5, 10, 10, 10, 20];
        let mut buf = Vec::new();
        encode_posting_list(&ids, &mut buf);

        let mut pos = 0;
        let decoded = decode_posting_list(&buf, &mut pos);
        assert_eq!(decoded, vec![5, 10, 20]);
    }

    #[test]
    fn test_posting_list_large_ids() {
        let ids: Vec<DocId> = vec![1_000_000, 2_000_000, 3_000_000];
        let mut buf = Vec::new();
        encode_posting_list(&ids, &mut buf);

        let mut pos = 0;
        let decoded = decode_posting_list(&buf, &mut pos);
        assert_eq!(decoded, ids);
    }

    #[test]
    fn test_posting_list_consecutive() {
        let ids: Vec<DocId> = vec![100, 101, 102, 103, 104];
        let mut buf = Vec::new();
        encode_posting_list(&ids, &mut buf);

        // Deltas are all 1 except first (100), so very compact
        let mut pos = 0;
        let decoded = decode_posting_list(&buf, &mut pos);
        assert_eq!(decoded, ids);
    }

    #[test]
    fn test_multiple_posting_lists_in_buffer() {
        let mut buf = Vec::new();
        let list1: Vec<DocId> = vec![1, 5, 10];
        let list2: Vec<DocId> = vec![100, 200];

        encode_posting_list(&list1, &mut buf);
        encode_posting_list(&list2, &mut buf);

        let mut pos = 0;
        let decoded1 = decode_posting_list(&buf, &mut pos);
        let decoded2 = decode_posting_list(&buf, &mut pos);
        assert_eq!(decoded1, list1);
        assert_eq!(decoded2, list2);
        assert_eq!(pos, buf.len());
    }

    #[test]
    fn test_decode_posting_list_empty_data() {
        let mut pos = 0;
        let decoded = decode_posting_list(&[], &mut pos);
        assert!(decoded.is_empty());
    }

    // ─── Property Tests ───

    mod proptests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn varint_roundtrip(value: u32) {
                let mut buf = Vec::new();
                encode_varint(value, &mut buf);
                let mut pos = 0;
                let decoded = decode_varint(&buf, &mut pos).unwrap();
                prop_assert_eq!(decoded, value);
                prop_assert_eq!(pos, buf.len());
            }

            #[test]
            fn posting_list_roundtrip(mut ids in prop::collection::vec(0u32..1_000_000, 0..200)) {
                ids.sort_unstable();
                ids.dedup();

                let mut buf = Vec::new();
                encode_posting_list(&ids, &mut buf);

                let mut pos = 0;
                let decoded = decode_posting_list(&buf, &mut pos);
                prop_assert_eq!(&decoded, &ids);
                prop_assert_eq!(pos, buf.len());
            }

            #[test]
            fn decoded_list_is_always_sorted(ids in prop::collection::vec(0u32..1_000_000, 0..200)) {
                let mut buf = Vec::new();
                encode_posting_list(&ids, &mut buf);

                let mut pos = 0;
                let decoded = decode_posting_list(&buf, &mut pos);

                // Verify the decoded list is sorted
                for window in decoded.windows(2) {
                    prop_assert!(window[0] < window[1],
                        "decoded list not sorted: {} >= {}", window[0], window[1]);
                }
            }
        }
    }
}
