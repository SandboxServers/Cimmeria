//! `onVersionInfo` encoded for the two entities that receive it: the
//! Account at character select (client method 0, `0x80`) and SGWPlayer
//! in-world (client method 96, extended encoding).

use cimmeria_mercury::encryption::{EncryptionVersion, MercuryEncryption};

use super::{build_version_info, build_version_info_to_player, SGW_PLAYER_ON_VERSION_INFO};

const KEY: [u8; 32] = [0x42u8; 32];

/// `CategoryId, Version, RequiredUpdates, InvalidateAll, InvalidKeys` for
/// category 12, version 7000, no updates, invalidate, keys [5, 6].
fn expected_args() -> Vec<u8> {
    let mut a = Vec::new();
    a.extend_from_slice(&12u32.to_le_bytes());
    a.extend_from_slice(&7000u32.to_le_bytes());
    a.extend_from_slice(&0u32.to_le_bytes());
    a.push(1);
    a.extend_from_slice(&2u32.to_le_bytes());
    a.extend_from_slice(&5u32.to_le_bytes());
    a.extend_from_slice(&6u32.to_le_bytes());
    a
}

fn body(packet: &[u8]) -> Vec<u8> {
    let pt = MercuryEncryption::from_session_key(KEY)
        .decrypt(packet)
        .unwrap();
    pt[1..].to_vec()
}

/// In-world: `[0xBD][len][player id][96 - 61 = 35][args]`.
#[test]
fn in_world_version_info_is_sgw_player_method_96() {
    let out = build_version_info_to_player(
        &KEY,
        1,
        &[],
        12,
        7000,
        0,
        true,
        &[5, 6],
        0x0102_0304,
        EncryptionVersion::V1,
    );
    let b = body(&out);
    let args = expected_args();
    assert_eq!(b[0], 0xBD, "extended entity-method marker");
    assert_eq!(
        u16::from_le_bytes([b[1], b[2]]) as usize,
        4 + 1 + args.len()
    );
    assert_eq!(&b[3..7], &0x0102_0304u32.to_le_bytes(), "player entity id");
    assert_eq!(b[7], (SGW_PLAYER_ON_VERSION_INFO - 61) as u8);
    assert_eq!(b[7], 35);
    assert_eq!(&b[8..8 + args.len()], &args[..]);
}

/// Character select: `[0x80][len][account id][args]`, the same arguments.
#[test]
fn account_version_info_is_account_method_0() {
    let out = build_version_info(
        &KEY,
        1,
        &[],
        12,
        7000,
        0,
        true,
        &[5, 6],
        9,
        EncryptionVersion::V1,
    );
    let b = body(&out);
    let args = expected_args();
    assert_eq!(b[0], 0x80);
    assert_eq!(u16::from_le_bytes([b[1], b[2]]) as usize, 4 + args.len());
    assert_eq!(&b[3..7], &9u32.to_le_bytes());
    assert_eq!(&b[7..7 + args.len()], &args[..]);
}
