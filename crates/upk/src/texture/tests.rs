//! The texture code is pinned by golden values: a patch recipe carries the
//! SHA-256 of a rebuilt file, so a change to the resampler, the DXT1 encoder,
//! the LZO compressor or the texture layout changes every player's result and
//! must fail here first. All inputs are synthetic; no game bytes are needed.

use super::{dxt1, lzo_chunks, rebake_world_map, resample, texture2d, WorldMapRebake};
use crate::patcher::tests::fixtures::{write_temp, Builder};
use crate::Package;

/// FNV-1a, 64 bit: a stable fingerprint for golden values without a hash crate.
fn fnv(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, &b| {
        (h ^ b as u64).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

/// A deterministic test picture with all three channels varying.
fn picture(w: usize, h: usize, seed: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(w * h * 3);
    for y in 0..h {
        for x in 0..w {
            out.push(((x * 255) / (w - 1) + seed * 31) as u8);
            out.push(((y * 255) / (h - 1) + seed * 17) as u8);
            out.push(((x * 7 + y * 13 + seed * 5) % 256) as u8);
        }
    }
    out
}

#[test]
fn resampling_averages_exactly_and_keeps_flat_colour() {
    // Two black and two white pixels average to 127.5, which rounds half up.
    let px = [0, 0, 0, 255, 255, 255, 255, 255, 255, 0, 0, 0];
    assert_eq!(resample::area(&px, 2, 2, 1, 1), vec![128, 128, 128]);
    // A flat image stays flat at a ratio that splits source pixels.
    let flat = vec![200u8; 7 * 5 * 3];
    assert!(resample::area(&flat, 7, 5, 3, 2).iter().all(|&v| v == 200));
    // Identity.
    let pic = picture(8, 8, 1);
    assert_eq!(resample::area(&pic, 8, 8, 8, 8), pic);
}

#[test]
fn resampling_output_is_pinned() {
    let out = resample::area(&picture(13, 9, 3), 13, 9, 5, 4);
    assert_eq!(fnv(&out), 11_072_565_570_581_062_804, "{out:?}");
}

#[test]
fn dxt1_encoding_is_pinned_and_decodes_close_to_the_input() {
    let pic = picture(8, 8, 2);
    let enc = dxt1::encode(&pic, 8, 8);
    assert_eq!(enc.len(), 32);
    assert_eq!(fnv(&enc), 4_195_798_115_472_123_330, "{enc:02x?}");
    // A smooth picture (no channel wraps) comes back within a bounded error.
    let smooth = picture(8, 8, 0);
    let dec = dxt1::decode(&dxt1::encode(&smooth, 8, 8), 8, 8).unwrap();
    let worst = smooth
        .iter()
        .zip(&dec)
        .map(|(&a, &b)| (a as i32 - b as i32).abs())
        .max()
        .unwrap();
    assert!(worst < 100, "worst channel error {worst}");
    // A flat block is exact up to RGB565 quantisation.
    let flat = dxt1::encode(&[100, 150, 200].repeat(16), 4, 4);
    assert_eq!(dxt1::decode(&flat, 4, 4).unwrap()[..3], [99, 150, 206]);
}

#[test]
fn lzo_chunks_round_trip_and_are_pinned() {
    let data: Vec<u8> = (0..300_000u32)
        .map(|i| (i % 251) as u8 ^ (i / 997) as u8)
        .collect();
    let packed = lzo_chunks::pack(&data).unwrap();
    assert_eq!(lzo_chunks::unpack(&packed, data.len()).unwrap(), data);
    assert_eq!(packed.len(), 20_087, "length");
    assert_eq!(fnv(&packed), 13_096_918_308_264_987_490, "hash");
}

/// A texture export with an empty property list and the given mips.
fn texture_data(b: &mut Builder, mips: &[(i32, i32, Vec<u8>, i32)]) -> Vec<u8> {
    let mut d = Vec::new();
    Builder::i32s(&mut d, &[0]); // net index
    b.none(&mut d);
    d.extend_from_slice(&[0u8; 12]);
    Builder::i32s(&mut d, &[0, mips.len() as i32]); // array offset (rewritten), count
    for (flags, elements, payload, edge) in mips {
        Builder::i32s(&mut d, &[*flags, *elements, payload.len() as i32, 0]);
        d.extend_from_slice(payload);
        Builder::i32s(&mut d, &[*edge, *edge]);
    }
    d
}

fn params() -> WorldMapRebake {
    WorldMapRebake {
        texture: "overview".into(),
        tile_prefix: "tile_".into(),
        lo: (-1, 0),
        hi: (0, 2),
        size: 16,
        pad: [255, 0, 255],
        carry: 2,
    }
}

/// A MapData-like package: six 8x8 tiles (two columns, three rows), an
/// overview texture with an LZO mip chain, and an unrelated export after it.
fn map_package() -> Vec<u8> {
    let mut b = Builder::default();
    for hi in 0..=2i32 {
        for lo in -1..=0i32 {
            let name = format!("tile_{:04x}{:04x}", hi as i16 as u16, lo as i16 as u16);
            let rgb = picture(8, 8, (hi * 2 + lo + 3) as usize);
            let raw = dxt1::encode(&rgb, 8, 8);
            // Alternate stored and LZO tiles: the real file's tiles are LZO.
            let (flags, payload) = if (hi + lo) % 2 == 0 {
                (texture2d::FLAG_LZO, lzo_chunks::pack(&raw).unwrap())
            } else {
                (0, raw)
            };
            let d = texture_data(&mut b, &[(flags, 32, payload, 8)]);
            b.export(0, 0, &name, d);
        }
    }
    let stock: Vec<(i32, i32, Vec<u8>, i32)> = [16usize, 8, 4, 4]
        .iter()
        .map(|&e| {
            let raw = dxt1::encode(&picture(e.max(4), e.max(4), 9), e.max(4), e.max(4));
            (
                texture2d::FLAG_LZO,
                raw.len() as i32,
                lzo_chunks::pack(&raw).unwrap(),
                e as i32,
            )
        })
        .collect();
    let d = texture_data(&mut b, &stock);
    b.export(0, 0, "overview", d);
    b.export(0, 0, "after", vec![1, 2, 3, 4]);
    // The real package lists export 165 in export 1's depends; so does this one.
    b.depends.insert(1, vec![8]);
    b.build()
}

#[test]
fn texture_layout_round_trips() {
    let path = write_temp("tex-rt", &map_package());
    let pkg = Package::open(&path).unwrap();
    let e = pkg
        .exports
        .iter()
        .find(|e| e.object_name == "overview")
        .unwrap();
    let data = pkg.read_export_data(e).unwrap();
    let tex = texture2d::parse(&data, &pkg.names).unwrap();
    assert_eq!(tex.mips.len(), 4);
    assert_eq!(tex.mips[0].width, 16);
    // The fixture writes 0 for its offsets, so only the structure round-trips.
    let again = texture2d::write(&tex, e.serial_offset as usize);
    assert_eq!(again.len(), data.len());
    assert_eq!(
        texture2d::parse(&again, &pkg.names).unwrap().mips[3].payload,
        tex.mips[3].payload
    );
    let _ = std::fs::remove_file(path);
}

#[test]
fn rebaking_a_world_map_is_deterministic_and_touches_only_the_overview() {
    let input = map_package();
    let path = write_temp("rebake-in", &input);
    let out = rebake_world_map(&path, &params()).unwrap();
    assert_eq!(
        rebake_world_map(&path, &params()).unwrap(),
        out,
        "two runs differ"
    );
    // The golden value: every byte of the rebuilt package.
    assert_eq!(
        fnv(&out),
        3_340_055_874_709_719_910,
        "rebuilt package, {} bytes",
        out.len()
    );

    let (before, after) = (
        Package::open(&path).unwrap(),
        Package::open(write_temp("rebake-out", &out)).unwrap(),
    );
    for (b, a) in before.exports.iter().zip(&after.exports) {
        let same = before.read_export_data(b).unwrap() == after.read_export_data(a).unwrap();
        assert_eq!(same, a.object_name != "overview", "{}", a.object_name);
    }
    // The depends table (a real one, with a list on export 1) is copied verbatim.
    let span = |bytes: &[u8], h: &crate::PackageHeader| {
        bytes[h.depends_offset as usize..h.total_header_size as usize].to_vec()
    };
    assert_eq!(span(&input, &before.header), span(&out, &after.header));
    assert_eq!(span(&input, &before.header).len(), 8 * 4 + 4);
    let e = after
        .exports
        .iter()
        .find(|e| e.object_name == "overview")
        .unwrap();
    let tex = texture2d::parse(&after.read_export_data(e).unwrap(), &after.names).unwrap();
    let dims: Vec<_> = tex
        .mips
        .iter()
        .map(|m| (m.width, m.height, m.flags))
        .collect();
    assert_eq!(
        dims,
        vec![(16, 16, 0x10), (8, 8, 0x10), (4, 4, 0x10), (4, 4, 0x10)]
    );
    // Absolute offsets point at the payloads where the writer put them.
    let mut at = e.serial_offset as usize + out_head_len(&tex);
    for m in &tex.mips {
        let field = i32::from_le_bytes(out[at + 12..at + 16].try_into().unwrap()) as usize;
        assert_eq!(&out[field..field + m.payload.len()], &m.payload[..]);
        at += 16 + m.payload.len() + 8;
    }
    let top = dxt1::decode(
        &lzo_chunks::unpack(&tex.mips[0].payload, 16 * 16 / 2).unwrap(),
        16,
        16,
    )
    .unwrap();
    // The picture fills the left 11 columns, so the pad (magenta) is on the right.
    assert!(
        top[(15 * 3)..(15 * 3 + 3)][1] < 90,
        "right edge is not pad: {:?}",
        &top[45..48]
    );
    let _ = std::fs::remove_file(path);
}

fn out_head_len(tex: &texture2d::Texture2dData) -> usize {
    tex.head.len() + 8
}

#[test]
fn a_package_without_the_tiles_is_refused() {
    let mut p = params();
    p.hi = (0, 5);
    let path = write_temp("rebake-missing", &map_package());
    let err = rebake_world_map(&path, &p).unwrap_err().to_string();
    assert!(err.contains("tile_"), "{err}");
    let _ = std::fs::remove_file(path);
}

/// Parameters that would allocate gigabytes or overflow must fail one patch
/// with an error, and fail before the package is even opened.
#[test]
fn oversized_or_overflowing_parameters_are_refused_before_anything_is_read() {
    let nowhere = std::path::Path::new("does-not-exist.upk");
    let cases: Vec<(&str, Box<dyn Fn(&mut WorldMapRebake)>)> = vec![
        ("huge size", Box::new(|p| p.size = 65_536)),
        ("size not a multiple of 4", Box::new(|p| p.size = 18)),
        ("tiny size", Box::new(|p| p.size = 4)),
        ("carry beyond size", Box::new(|p| p.carry = 17)),
        (
            "lo spanning all of i32",
            Box::new(|p| p.lo = (i32::MIN, i32::MAX)),
        ),
        ("hi beyond i16", Box::new(|p| p.hi = (30_000, 40_000))),
        ("lo below i16", Box::new(|p| p.lo = (-40_000, -39_990))),
        ("more than 256 columns", Box::new(|p| p.lo = (0, 256))),
        ("reversed rows", Box::new(|p| p.hi = (2, 0))),
    ];
    for (what, change) in cases {
        let mut p = params();
        change(&mut p);
        let err = rebake_world_map(nowhere, &p).unwrap_err().to_string();
        assert!(err.contains("invalid"), "{what}: {err}");
    }
}

#[test]
fn a_package_that_claims_huge_textures_is_refused_without_allocating() {
    // An overview mip of 2^30 texels a side.
    let mut b = Builder::default();
    for hi in 0..=2i32 {
        for lo in -1..=0i32 {
            let name = format!("tile_{:04x}{:04x}", hi as i16 as u16, lo as i16 as u16);
            let raw = dxt1::encode(&picture(8, 8, 1), 8, 8);
            let d = texture_data(&mut b, &[(0, 32, raw, 8)]);
            b.export(0, 0, &name, d);
        }
    }
    let d = texture_data(&mut b, &[(0, 8, vec![0; 8], 1 << 30)]);
    b.export(0, 0, "overview", d);
    let path = write_temp("rebake-huge-mip", &b.build());
    let err = rebake_world_map(&path, &params()).unwrap_err().to_string();
    assert!(err.contains("edge"), "{err}");

    // A tile that claims a huge LZO payload is refused before the allocation.
    let mut lie = lzo_chunks::pack(&[0u8; 32]).unwrap();
    lie[12..16].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(lzo_chunks::unpack(&lie, 32).is_err());
    let _ = std::fs::remove_file(path);
}
