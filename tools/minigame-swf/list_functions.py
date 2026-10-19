"""List every ActionScript function defined in a GFX or SWF movie.

Usage:
  python -I tools/minigame-swf/list_functions.py <movie.gfx|movie.swf>
  python -I tools/minigame-swf/list_functions.py <movie> --strings
  python -I tools/minigame-swf/list_functions.py <movie> --function <name>

Walks the DoAction, DoInitAction and DefineSprite tags and prints one line per
DefineFunction/DefineFunction2: `<timeline> <frame> <name>(<args>)`, where the
timeline is `root`, `sprite<id>` or `init<id>` (a DoInitAction for that
sprite). An anonymous function prints as `~<hint>`, the last string pushed
before it (usually the member it is assigned to). --strings dumps the constant
pools; --function disassembles the body of each function with that name.
The movie is client data: keep it, and any saved output, outside the repo.
"""

import argparse
import importlib.util
import pathlib
import struct
import sys

_spec = importlib.util.spec_from_file_location('as2dis', pathlib.Path(__file__).with_name('as2dis.py'))
as2dis = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(as2dis)


class Function:
    def __init__(self, timeline, frame, name, args, body, cpool):
        self.timeline = timeline
        self.frame = frame
        self.name = name
        self.args = args
        self.body = body
        self.cpool = cpool

    def line(self):
        return f"{self.timeline} {self.frame} {self.name}({', '.join(self.args)})"


def _cstr(buf, pos):
    end = buf.index(b'\0', pos)
    return buf[pos:end].decode('latin1'), end + 1


def parse_function(op, payload):
    """(name, args, code_size) for a DefineFunction (0x9B) or DefineFunction2 (0x8E)."""
    name, q = _cstr(payload, 0)
    count = struct.unpack_from('<H', payload, q)[0]
    q += 2
    args = []
    if op == 0x9B:
        for _ in range(count):
            arg, q = _cstr(payload, q)
            args.append(arg)
    else:
        q += 3  # register count u8, flags u16
        for _ in range(count):
            reg = payload[q]
            arg, q = _cstr(payload, q + 1)
            args.append(arg if arg else f'r{reg}')
    code_size = struct.unpack_from('<H', payload, q)[0]
    return name, args, code_size


def walk_actions(code, timeline, frame, funcs, pools):
    """Scan one action block; nested function bodies are scanned in place."""
    cpool = []
    hint = ''
    i = 0
    while i < len(code):
        op = code[i]
        i += 1
        if op == 0:
            continue
        payload = b''
        if op >= 0x80:
            ln = struct.unpack_from('<H', code, i)[0]
            i += 2
            payload = code[i:i + ln]
            i += ln
        if op == 0x88:
            cnt = struct.unpack_from('<H', payload, 0)[0]
            cpool = [p.decode('latin1') for p in payload[2:].split(b'\0')[:cnt]]
            pools.append((timeline, frame, cpool))
        elif op == 0x96:
            strings = [v for v in as2dis.push_values(payload, cpool) if v.startswith(("'", '"'))]
            if strings:
                hint = strings[-1][1:-1]
        elif op in (0x9B, 0x8E):
            name, args, size = parse_function(op, payload)
            funcs.append(Function(timeline, frame, name or f'~{hint}', args, code[i:i + size], list(cpool)))


def walk_tags(buf, timeline, funcs, pools):
    frame = 1
    p = 0
    while p + 2 <= len(buf):
        h = struct.unpack_from('<H', buf, p)[0]
        p += 2
        code, ln = h >> 6, h & 0x3F
        if ln == 0x3F:
            ln = struct.unpack_from('<I', buf, p)[0]
            p += 4
        data = buf[p:p + ln]
        p += ln
        if code == 0:
            return
        if code == 1:
            frame += 1
        elif code == 12:
            walk_actions(data, timeline, frame, funcs, pools)
        elif code == 59:
            sprite = struct.unpack_from('<H', data, 0)[0]
            walk_actions(data[2:], f'init{sprite}', 1, funcs, pools)
        elif code == 39:
            sprite = struct.unpack_from('<H', data, 0)[0]
            walk_tags(data[4:], f'sprite{sprite}', funcs, pools)


def scan(movie_bytes):
    """(functions, constant pools) for a whole movie."""
    body = as2dis.movie_body(movie_bytes)
    funcs, pools = [], []
    walk_tags(body[as2dis.tags_offset(body):], 'root', funcs, pools)
    return funcs, pools


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.strip().splitlines()[0])
    parser.add_argument('movie', help='an extracted .gfx or .swf movie')
    parser.add_argument('--strings', action='store_true', help='dump the constant pools')
    parser.add_argument('--function', metavar='NAME', help='disassemble the functions with this name')
    args = parser.parse_args(argv)
    funcs, pools = scan(pathlib.Path(args.movie).read_bytes())
    if args.strings:
        for timeline, frame, pool in pools:
            print(f'--- {timeline} {frame} cpool[{len(pool)}]')
            for k, s in enumerate(pool):
                print(f'  {k}: {s!r}')
        return 0
    if args.function:
        matches = [f for f in funcs if f.name == args.function]
        if not matches:
            print(f'no function named {args.function!r}', file=sys.stderr)
            return 1
        for f in matches:
            out = []
            as2dis.dis(f.body, out, list(f.cpool))
            print(f'--- {f.line()}')
            print('\n'.join(out))
        return 0
    for f in funcs:
        print(f.line())
    print(f'{len(funcs)} functions', file=sys.stderr)
    return 0


if __name__ == '__main__':
    sys.exit(main())
