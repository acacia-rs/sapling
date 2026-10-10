"""Patches a BDS copy so loaded chunks never run their block tick queues: worlds it saves hold worldgen only.

usage: py -3 -I bds_still.py <bedrock_server.exe of your own copy> [--check]

`LevelChunk::tick` (1409f98a0) calls `BlockTickingQueue::tickPendingTicks` (1416042f0) four times: the block and
random queues in the first-ticks flush (instaticking, until both are empty) and in the regular tick. Each call
becomes `xor eax, eax` (nothing ran), so the flush ends at once and no scheduled tick ever runs in game time;
decoration's own calls (148215955, 148215a15) stay. The original is kept beside the exe as `.orig`.
Why: docs/research/worldgen-bds-liquids.md, "Game time".
"""
import shutil
import struct
import sys

BASE = 0x140000000
TARGET = 0x1416042F0
SITES = [0x1409FACFB, 0x1409FAD28, 0x1409FC307, 0x1409FC3C8]
STILL = b"\x31\xc0\x90\x90\x90"


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
    offsets = [file_offset(data, va) for va in SITES]
    done = [bytes(data[o : o + 5]) == STILL for o in offsets]
    if all(done):
        print("already still")
        return
    for va, o in zip(SITES, offsets):
        rel = struct.unpack_from("<i", data, o + 1)[0]
        if data[o] != 0xE8 or va + 5 + rel != TARGET:
            raise SystemExit(f"{va:x} is not a call of {TARGET:x}: another BDS build?")
        data[o : o + 5] = STILL
    if check:
        print("unpatched; the four call sites match")
        return
    shutil.copyfile(exe, exe + ".orig")
    open(exe, "wb").write(data)
    print(f"patched {len(SITES)} call sites; original in {exe}.orig")


main()
