"""Patches a BDS copy so every default-constructed `Random` is seeded with 0 instead of system entropy.

usage: py -3 -I bds_seed0.py <bedrock_server.exe of your own copy> [--check]

`Random::Random()` (1411889d0) asks the platform for four random bytes (call 141ccbe30 at 141188a3a) into a
stack word it has just zeroed; the call becomes nops, so the seed is 0. The per-thread generator that picks
`MultifaceFeature`'s faces and spread directions is such a Random: with this patch and `max-threads=1` a
world is a function of the seed and the chunk requests alone. The original is kept as `.seed.orig`.
Why: docs/research/worldgen-bds-order.md.
"""
import shutil
import struct
import sys

BASE = 0x140000000
SITE, TARGET = 0x141188A3A, 0x141CCBE30
NOPS = b"\x0f\x1f\x44\x00\x00"


def file_offset(data, va):
    pe = struct.unpack_from("<I", data, 0x3C)[0]
    count, optional = struct.unpack_from("<H", data, pe + 6)[0], struct.unpack_from("<H", data, pe + 20)[0]
    for i in range(count):
        size, rva, _, raw = struct.unpack_from("<IIII", data, pe + 24 + optional + 40 * i + 8)
        if rva <= va - BASE < rva + size:
            return va - BASE - rva + raw
    raise SystemExit(f"{va:x} is in no section")


def main():
    exe, check = sys.argv[1], "--check" in sys.argv
    data = bytearray(open(exe, "rb").read())
    o = file_offset(data, SITE)
    if bytes(data[o : o + 5]) == NOPS:
        print("already seeded with 0")
        return
    rel = struct.unpack_from("<i", data, o + 1)[0]
    if data[o] != 0xE8 or SITE + 5 + rel != TARGET:
        raise SystemExit(f"{SITE:x} is not a call of {TARGET:x}: another BDS build?")
    if check:
        print("unpatched; the call site matches")
        return
    shutil.copyfile(exe, exe + ".seed.orig")
    data[o : o + 5] = NOPS
    open(exe, "wb").write(data)
    print(f"patched; original in {exe}.seed.orig")


main()
