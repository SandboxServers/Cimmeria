"""Render exact XRC polygon footprints from the four committed navmeshes."""

from pathlib import Path
import struct

import numpy as np
from PIL import Image, ImageDraw, ImageFont

ROOT = Path('data/spaces')
OUT = Path('docs/analysis/debug-area')
OUT.mkdir(parents=True, exist_ok=True)


def read_block(buf, offset):
    nverts, npolys, nvp, border = struct.unpack_from('<4I', buf, offset)
    offset += 16
    cs, ch = struct.unpack_from('<2f', buf, offset)
    offset += 8
    bmin = np.array(struct.unpack_from('<3f', buf, offset), dtype=np.float32)
    offset += 24  # bmin and bmax
    verts = np.frombuffer(buf, dtype='<u2', count=nverts * 3, offset=offset).reshape(-1, 3)
    offset += nverts * 6
    polys = np.frombuffer(buf, dtype='<u2', count=npolys * nvp * 2, offset=offset).reshape(npolys, nvp * 2)
    offset += npolys * nvp * 4 + npolys * 5  # polygon refs, regs, flags, areas
    dm, dv, dt = struct.unpack_from('<3I', buf, offset)
    offset += 12 + dm * 16 + dv * 12 + dt * 4
    return (verts, polys[:, :nvp], cs, ch, bmin), offset


def polygons(path):
    buf = path.read_bytes()
    blocks = []
    if buf[:4] == b'XRCT':
        ntiles = struct.unpack_from('<I', buf, 40)[0]
        offset = 48
        for _ in range(ntiles):
            offset += 8  # tile x,z
            block, offset = read_block(buf, offset)
            blocks.append(block)
    else:
        block, offset = read_block(buf, 12)
        blocks.append(block)
    assert offset == len(buf), (path, offset, len(buf))
    for verts, polyrefs, cs, ch, bmin in blocks:
        xyz = bmin + verts * np.array([cs, ch, cs], dtype=np.float32)
        for refs in polyrefs:
            refs = refs[refs != 0xffff]
            if len(refs) < 3:
                continue
            pts = xyz[refs]
            yield [(float(p[0]), float(p[2])) for p in pts], float(pts[:, 1].mean())


FONT = ImageFont.load_default()


def draw_map(name, bounds, sites, output, size=(1300, 1050), y_filter=None):
    x0, z0, x1, z1 = bounds
    width, height = size
    margin = 70
    scale = min((width - margin * 2) / (x1 - x0), (height - margin * 2) / (z1 - z0))
    to_px = lambda x, z: (int(margin + (x - x0) * scale), int(height - margin - (z - z0) * scale))
    im = Image.new('RGB', size, '#f7f7f4')
    d = ImageDraw.Draw(im)
    # Draw low surfaces first so upper floors remain visible in layered buildings.
    polys = [(points, y) for points, y in polygons(ROOT / f'{name}.nav')
             if any(x0 <= x <= x1 and z0 <= z <= z1 for x, z in points)
             and (y_filter is None or y_filter[0] <= y <= y_filter[1])]
    for points, y in sorted(polys, key=lambda p: p[1]):
        if y < -10:
            fill = '#b9d3dd'
        elif y < 10:
            fill = '#d4dedb'
        elif y < 45:
            fill = '#a9c9b1'
        elif y < 100:
            fill = '#c9b8a1'
        else:
            fill = '#c9a9a3'
        d.polygon([to_px(x, z) for x, z in points], fill=fill)
    step = next((s for s in (100, 200, 500) if s * scale > 80), 500)
    for x in range((x0 // step + 1) * step, x1, step):
        px = to_px(x, z0)[0]
        d.line((px, margin, px, height-margin), fill='#bec6c3', width=1)
        d.text((px+3, height-margin+7), str(x), fill='#394744', font=FONT)
    for z in range((z0 // step + 1) * step, z1, step):
        py = to_px(x0, z)[1]
        d.line((margin, py, width-margin, py), fill='#bec6c3', width=1)
        d.text((6, py-5), str(z), fill='#394744', font=FONT)
    for label, x, z, color in sites:
        px, py = to_px(x, z)
        d.ellipse((px-5, py-5, px+5, py+5), fill=color, outline='#ffffff', width=2)
        d.text((px+8, py-11), label, fill='#202825', font=FONT, stroke_width=2, stroke_fill='#ffffff')
    d.text((margin, 18), f'{name.replace("_", " ").title()} | committed navmesh footprint | BigWorld X/Z (m)',
           fill='#202825', font=FONT)
    d.text((margin, height-30), f'{len(polys):,} polygons shown | color by floor elevation; markers are candidate anchors, not verified spawns',
           fill='#394744', font=FONT)
    im.save(OUT / output)
    print(output, len(polys))


draw_map('tollana', (-1050, -1030, 1100, 760), [
    ('gate / service', 211, -548, '#a73528'), ('gate ring', 235, -571, '#d97728'),
    ('middle ring lead', -140, 312, '#d97728'), ('gallery ring', -766, 380, '#d97728'),
    ('gallery rooms', -742, 408, '#7b399b'), ('urban cover', 540, -530, '#265b9e'),
    ('urban pad lead', 570, -530, '#d97728'), ('target lab', 540, -865, '#265b9e'),
    ('lab pad lead', 570, -865, '#d97728'), ('hostile rows', 895, -530, '#a73528'),
    ('hostile pad lead', 925, -530, '#d97728'), ('combat theatre', 895, -865, '#a73528'),
    ('combat pad lead', 925, -865, '#d97728')], 'map-study-tollana.png')

draw_map('castle', (150, 340, 1140, 1120), [
    ('armory ring', 466, 991, '#d97728'), ('infirmary', 362, 886, '#265b9e'),
    ('interrogation', 260, 1040, '#7b399b'), ('throne', 370, 650, '#a73528'),
    ('courtyard', 535, 620, '#a73528'), ('checkpoint', 894, 527, '#265b9e'),
    ('bunker', 1052, 432, '#265b9e')], 'map-study-castle.png')

draw_map('dakara_e1', (-850, -850, 850, 850), [
    ('gate', 96, 253, '#d97728'), ('city west', -100, 100, '#265b9e'),
    ('city east', 300, 100, '#265b9e'), ('city north', 300, 300, '#265b9e'),
    ('west native ring', -139, 49, '#d97728'), ('east native ring', 358, 102, '#d97728'),
    ('ship effect A', 367, 136, '#a73528'), ('ship effect B', -112, 78, '#a73528'),
    ('outer ground lead', 700, -600, '#7b399b')], 'map-study-dakara.png')

draw_map('agnos', (-1450, -900, 900, 1050), [
    ('ring north', -1194, 179, '#d97728'), ('ring south', -1167, -619, '#d97728'),
    ('terminal A', -370, 270, '#265b9e'), ('terminal B', -200, 570, '#265b9e'),
    ('terminal C', 0, 810, '#265b9e'), ('gate arrival', 21, 16, '#a73528')],
    'map-study-agnos.png')
