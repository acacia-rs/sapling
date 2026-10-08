"""registry.py <out.tsv>: every vanilla block state (name, properties) from acacia-world's data/blocks.bin."""
import os
import struct
import sys

B = open(os.environ['ACACIA_BLOCKS_BIN'], 'rb').read()  # acacia-world's data/blocks.bin
p = 0


def take(fmt):
    global p
    v = struct.unpack_from('<' + fmt, B, p)
    p += struct.calcsize('<' + fmt)
    return v[0] if len(v) == 1 else v


assert B[:4] == b'BWB3'
p = 4
for _ in range(take('B')):
    take('f')
strings = []
for _ in range(take('I')):
    n = take('H')
    strings.append(B[p:p + n].decode())
    p += n
shapes = []
for _ in range(take('I')):
    n = take('B')
    shapes.append(n)
    for _ in range(n):
        take('6f')
for _ in range(take('H')):
    take('fBBB')
with open(sys.argv[1], 'w') as f:
    for _ in range(take('I')):
        name, props, shape, flags, _ld, _fr, _hash, _mining, _light = take('HHHHBBIHB')
        # flags: bit 0 full cube, bit 1 has collision boxes
        f.write(f'{strings[name]}\t{strings[props]}\t{(flags >> 1 & 1) | (2 if shapes[shape] else 0)}\n')
