//! The init data built for a key id when a file carries no Widevine `pssh` (§5).

/// Widevine's system id, `edef8ba9-79d6-4ace-a3c8-27dcd51d21ed`, as `pssh` boxes carry it.
pub const SYSTEM_ID: [u8; 16] = [
    0xed, 0xef, 0x8b, 0xa9, 0x79, 0xd6, 0x4a, 0xce, 0xa3, 0xc8, 0x27, 0xdc, 0xd5, 0x1d, 0x21, 0xed,
];

/// The `WidevinePsshData` fields this host writes, as protocol buffer tags (field number << 3
/// | wire type).
const ALGORITHM_TAG: u8 = 1 << 3; // field 1, varint
const KEY_ID_TAG: u8 = (2 << 3) | 2; // field 2, length-delimited
const PROVIDER_TAG: u8 = (3 << 3) | 2; // field 3, length-delimited
const POLICY_TAG: u8 = (6 << 3) | 2; // field 6, length-delimited
/// `algorithm` = AES-CTR.
const AES_CTR: u8 = 1;

/// A version 0 `pssh` box for Widevine naming one key id: CENC init data for
/// [`Cdm::open_session`](crate::Cdm::open_session).
///
/// Its data is a `WidevinePsshData` message with, in field order, `algorithm` = 1 (AES-CTR),
/// `key_id` = the key id, and `provider` and `policy` empty: the bytes Apple's licence server
/// accepts. A key id of another length than 16 bytes does not compile:
///
/// ```compile_fail
/// widevine::pssh_for_key(&[0u8; 15]);
/// ```
pub fn pssh_for_key(key_id: &[u8; 16]) -> Vec<u8> {
    let mut data = Vec::with_capacity(24);
    data.extend_from_slice(&[ALGORITHM_TAG, AES_CTR]);
    data.extend_from_slice(&[KEY_ID_TAG, 16]);
    data.extend_from_slice(key_id);
    data.extend_from_slice(&[PROVIDER_TAG, 0]);
    data.extend_from_slice(&[POLICY_TAG, 0]);

    // size, type, version and flags, system id, data size: 32 bytes before the data.
    let size = 32 + data.len();
    let mut pssh = Vec::with_capacity(size);
    // The data is 24 bytes: every size here fits its field.
    pssh.extend_from_slice(&(size as u32).to_be_bytes());
    pssh.extend_from_slice(b"pssh");
    pssh.extend_from_slice(&[0, 0, 0, 0]);
    pssh.extend_from_slice(&SYSTEM_ID);
    pssh.extend_from_slice(&(data.len() as u32).to_be_bytes());
    pssh.extend_from_slice(&data);
    pssh
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pssh_for_a_key_id_is_pinned() {
        let key_id: [u8; 16] = core::array::from_fn(|i| i as u8 + 1);
        let expected: Vec<u8> = [
            &[0x00, 0x00, 0x00, 0x38][..],
            b"pssh",
            &[0x00, 0x00, 0x00, 0x00],
            &[
                0xed, 0xef, 0x8b, 0xa9, 0x79, 0xd6, 0x4a, 0xce, 0xa3, 0xc8, 0x27, 0xdc, 0xd5, 0x1d,
                0x21, 0xed,
            ],
            &[0x00, 0x00, 0x00, 0x18],
            &[0x08, 0x01],
            &[0x12, 0x10],
            &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
            &[0x1a, 0x00],
            &[0x32, 0x00],
        ]
        .concat();
        assert_eq!(pssh_for_key(&key_id), expected);
    }
}
