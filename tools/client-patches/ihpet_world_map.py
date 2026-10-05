#!/usr/bin/env python3
"""Re-bake the Ihpet_Crater_Light world-map overview texture for client patch 013.

The client's world map draws the Texture2D `world__default_` from
`Maps/Ihpet_Crater_Light/Ihpet_Crater_Light_MapData.upk`. In the stock file that
texture is a 1.99x zoom of the map's top-left corner, so the map art disagrees
with the world coordinates every icon, blip and POI is placed with. The same
package holds the 154 correct 256x256 tiles (`thumb_WorldMap_<hi16><lo16>`), which
the client never draws. This script rebuilds `world__default_` from those tiles at
the scale the map record describes, and writes a new MapData package.

Findings (static decode and live client, 2026-10-05) are in
data/client-patches/README.md, patch 013.

Layout the map record defines (WorldMapCollection "Maps", element "_default_"):
    chunk bounds  lo -3..7 (11 columns, west to east), hi -13..0 (14 rows, north to south)
    UV extents    0.7857 (= 11/14) x 1.0 of the 1024x1024 texture
Both axes come out at 73.14 texels per 100 m chunk. Texels outside the extents are
magenta (255,0,255), which is what the stock default textures of Harset and Menfa use,
after an eight-texel carry of the last content column so no DXT1 block or bilinear tap
mixes content with magenta.

What is rewritten: the `world__default_` export (new mip chain, same DXT1 format, same
LZO chunking as the stock export, mips 8..10 kept as 4x4 tail blocks), the export
table's serial size for it, and the serial offset of the one export that follows it.
Nothing else in the package changes.

Usage:
    python tools/client-patches/ihpet_world_map.py --stock <stock MapData.upk> --out <new MapData.upk>
    python tools/client-patches/ihpet_world_map.py --stock <stock MapData.upk> --out <new> --preview <dir>

Needs Pillow (DXT1 encode, resampling) and lzallright (LZO). The output is
deterministic for fixed library versions; the committed patch zip is the reference.
"""
import argparse
import hashlib
import io
import struct
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools"))

import lzallright  # noqa: E402
from PIL import Image  # noqa: E402
from upk_parser import PackageReader  # noqa: E402

# SHA-256 of the stock Ihpet_Crater_Light_MapData.upk this script was written against.
STOCK_SHA256 = "ea86f7c32b6d8e230c86191b5f048e080fc979e80d584bbdc878e3db5404a1f4"
TEXTURE = "world__default_"
TILE_PREFIX = "thumb_WorldMap_"
CHUNK_SIZE = 256            # tile edge in texels
LO_MIN, LO_MAX = -3, 7      # columns, west to east
HI_MIN, HI_MAX = -13, 0     # rows, hi 0 is the top (north)
TEX = 1024
PAD = (255, 0, 255)
PACKAGE_TAG = 0x9E2A83C1
LZO_BLOCK = 0x20000
LZO_FLAG = 0x10             # bulk data stored LZO-compressed, as in the stock export
NONE_FNAME = bytes.fromhex("1100000000000000")  # name 17 "None", number 0


def signed16(v):
    return v - 0x10000 if v >= 0x8000 else v


class Mapdata:
    def __init__(self, path):
        self.path = path
        self.raw = Path(path).read_bytes()
        self.pkg = PackageReader(str(path))
        self.pkg.parse()
        self.pkg.f = open(path, "rb")
        self.records = self._export_records()

    def _export_records(self):
        """File offsets of each export record's serial_size and serial_offset fields."""
        p = self.pkg
        p.f.seek(p.header.export_offset)
        out = []
        for _ in range(p.header.export_count):
            p.read_i32(); p.read_i32(); p.read_i32()      # class, super, outer
            p.read_fname()                                  # object name
            p.read_i32(); p.read_u64()                      # archetype, object flags
            size_pos = p.f.tell()
            size = p.read_i32()
            off_pos = p.f.tell()
            off = p.read_i32()
            n = p.read_i32()                                # component map
            p.f.seek(n * 12, 1)
            p.read_u32()                                    # export flags
            n = p.read_i32()                                # generation net object counts
            p.f.seek(n * 4, 1)
            p.f.seek(16, 1)                                 # package guid
            out.append({"size_pos": size_pos, "off_pos": off_pos, "size": size, "off": off})
        return out

    def export(self, name):
        for i, e in enumerate(self.pkg.exports):
            if e.object_name == name:
                return i, e
        raise KeyError(name)

    def blob(self, e):
        return self.raw[e.serial_offset:e.serial_offset + e.serial_size]


def mip_chain(blob):
    """Split a Texture2D export into (prefix, [(flags, elem, payload, sx, sy)], mip_array_pos)."""
    i = blob.find(NONE_FNAME, 100)
    o = i + len(NONE_FNAME)
    q = o + 16
    n = struct.unpack_from("<i", blob, q)[0]
    q += 4
    mips = []
    for _ in range(n):
        flags, elem, sod, _off = struct.unpack_from("<4i", blob, q)
        payload = blob[q + 16:q + 16 + sod]
        q += 16 + sod
        sx, sy = struct.unpack_from("<2i", blob, q)
        q += 8
        mips.append((flags, elem, payload, sx, sy))
    assert q == len(blob), "unexpected bytes after the mip chain"
    return o, mips


def lzo_unpack(payload):
    tag, block, _ct, ut = struct.unpack_from("<4I", payload, 0)
    assert tag == PACKAGE_TAG
    n = (ut + block - 1) // block
    table = [struct.unpack_from("<II", payload, 16 + 8 * k) for k in range(n)]
    q = 16 + 8 * n
    lzo = lzallright.LZOCompressor()
    out = b""
    for c, u in table:
        out += lzo.decompress(payload[q:q + c], u)
        q += c
    return out


def lzo_pack(data):
    lzo = lzallright.LZOCompressor()
    blocks = [bytes(lzo.compress(data[k:k + LZO_BLOCK])) for k in range(0, len(data), LZO_BLOCK)]
    sizes = [min(LZO_BLOCK, len(data) - k) for k in range(0, len(data), LZO_BLOCK)]
    head = struct.pack("<4I", PACKAGE_TAG, LZO_BLOCK, sum(len(b) for b in blocks), len(data))
    table = b"".join(struct.pack("<II", len(b), u) for b, u in zip(blocks, sizes))
    return head + table + b"".join(blocks)


def dxt1(img):
    """DXT1 payload for an RGB image; sizes under 4 are padded to a 4x4 block."""
    w, h = img.size
    if w < 4 or h < 4:
        img = img.resize((4, 4), Image.BOX) if (w, h) != (4, 4) else img
    buf = io.BytesIO()
    img.save(buf, "DDS", pixel_format="DXT1")
    return buf.getvalue()[128:]


def decode_dxt1(data, w, h):
    hdr = (b"DDS " + struct.pack("<7I", 124, 0x81007, h, w, len(data), 0, 0) + b"\0" * 44
           + struct.pack("<2I4s5I", 32, 4, b"DXT1", 0, 0, 0, 0, 0) + struct.pack("<5I", 0x1000, 0, 0, 0, 0))
    return Image.open(io.BytesIO(hdr + data)).convert("RGB")


def stitch_tiles(md):
    cols, rows = LO_MAX - LO_MIN + 1, HI_MAX - HI_MIN + 1
    canvas = Image.new("RGB", (cols * CHUNK_SIZE, rows * CHUNK_SIZE), PAD)
    seen = 0
    for e in md.pkg.exports:
        if not e.object_name.startswith(TILE_PREFIX):
            continue
        h = e.object_name[len(TILE_PREFIX):]
        hi, lo = signed16(int(h[:4], 16)), signed16(int(h[4:], 16))
        if not (LO_MIN <= lo <= LO_MAX and HI_MIN <= hi <= HI_MAX):
            continue
        _o, mips = mip_chain(md.blob(e))
        flags, elem, payload, sx, sy = mips[0]
        assert (sx, sy) == (CHUNK_SIZE, CHUNK_SIZE) and flags == LZO_FLAG
        tile = decode_dxt1(lzo_unpack(payload), sx, sy)
        canvas.paste(tile, ((lo - LO_MIN) * CHUNK_SIZE, (0 - hi) * CHUNK_SIZE))
        seen += 1
    assert seen == cols * rows, f"expected {cols * rows} tiles, found {seen}"
    return canvas


def build_overview(mosaic):
    """1024x1024 overview: the mosaic scaled to the UV extents, top-left, magenta elsewhere."""
    cols, rows = LO_MAX - LO_MIN + 1, HI_MAX - HI_MIN + 1
    h = TEX
    w = round(TEX * cols / rows)
    out = Image.new("RGB", (TEX, TEX), PAD)
    out.paste(mosaic.resize((w, h), Image.LANCZOS), (0, 0))
    # Carry the last column eight texels into the magenta: DXT1 blocks are 4x4 and the
    # sampler is bilinear, so a content column beside a magenta one comes out pink.
    edge = out.crop((w - 1, 0, w, h))
    for k in range(8):
        out.paste(edge, (w + k, 0))
    return out


def new_blob(old_blob, export_offset, overview):
    o, old = mip_chain(old_blob)
    levels = [(TEX >> k, TEX >> k) for k in range(len(old))]
    chunks, img = [], overview
    for k, (flags, _elem, _payload, sx, sy) in enumerate(old):
        assert (sx, sy) == levels[k] or k >= 8 and (sx, sy) == (4, 4), (k, sx, sy)
        if k:
            side = max(TEX >> k, 1)
            img = overview.resize((side, side), Image.BOX)
        raw = dxt1(img)
        chunks.append((flags, len(raw), lzo_pack(raw) if flags == LZO_FLAG else raw, sx, sy))
    array_pos = export_offset + o + 16
    head = old_blob[:o + 12] + struct.pack("<i", array_pos) + struct.pack("<i", len(chunks))
    q = len(head)
    body = b""
    for flags, elem, payload, sx, sy in chunks:
        payload_pos = export_offset + q + len(body) + 16
        body += struct.pack("<4i", flags, elem, len(payload), payload_pos) + payload + struct.pack("<2i", sx, sy)
    return head + body


def build(stock_path, out_path, preview=None):
    md = Mapdata(stock_path)
    sha = hashlib.sha256(md.raw).hexdigest()
    if STOCK_SHA256 != "TO-BE-FILLED" and sha != STOCK_SHA256:
        raise SystemExit(f"stock MapData sha256 {sha} is not the file this script targets ({STOCK_SHA256})")
    idx, exp = md.export(TEXTURE)
    nxt = md.records[idx + 1:]
    mosaic = stitch_tiles(md)
    overview = build_overview(mosaic)
    blob = new_blob(md.blob(exp), exp.serial_offset, overview)
    delta = len(blob) - exp.serial_size
    raw = bytearray(md.raw[:exp.serial_offset] + blob + md.raw[exp.serial_offset + exp.serial_size:])
    rec = md.records[idx]
    struct.pack_into("<i", raw, rec["size_pos"], len(blob))
    for r in nxt:
        struct.pack_into("<i", raw, r["off_pos"], r["off"] + delta)
    # Anything after this export that holds absolute offsets would need shifting too.
    after = [e for e in md.pkg.exports if e.serial_offset > exp.serial_offset]
    assert all(e.object_name == "Maps" for e in after), "unexpected exports after the texture"
    Path(out_path).write_bytes(bytes(raw))
    if preview:
        Path(preview).mkdir(parents=True, exist_ok=True)
        overview.save(Path(preview) / "overview.png")
        mosaic.save(Path(preview) / "mosaic.png")
    return sha, hashlib.sha256(bytes(raw)).hexdigest(), len(raw), delta


def check(path):
    """Decode the written package back: the texture's mip chain must parse to the end."""
    md = Mapdata(path)
    _i, exp = md.export(TEXTURE)
    o, mips = mip_chain(md.blob(exp))
    top = decode_dxt1(lzo_unpack(mips[0][2]), TEX, TEX)
    assert top.getpixel((TEX - 1, TEX - 1)) == PAD
    return top


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--stock", required=True, help="stock Ihpet_Crater_Light_MapData.upk")
    ap.add_argument("--out", required=True, help="where to write the new package")
    ap.add_argument("--preview", help="directory for overview.png and mosaic.png")
    a = ap.parse_args()
    if Path(a.out).resolve() == Path(a.stock).resolve():
        raise SystemExit("--out is the stock file; write to a new path")
    sha_in, sha_out, size, delta = build(a.stock, a.out, a.preview)
    check(a.out)
    print(f"stock  sha256 {sha_in}\nresult sha256 {sha_out}\nresult size   {size} bytes (texture export {delta:+d})")


if __name__ == "__main__":
    main()
