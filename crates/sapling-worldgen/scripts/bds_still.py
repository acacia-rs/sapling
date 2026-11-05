"""Patches a BDS copy so loaded chunks never run their block tick queues: worlds it saves hold worldgen only.

usage: python3 -s bds_still.py <bedrock_server(.exe) of your own copy> [--check]

`LevelChunk::tick` (Windows 1409f98a0) calls `BlockTickingQueue::tickPendingTicks` (1416042f0; Linux c8e4770)
four times: the block and random queues in the first-ticks flush (instaticking, until both are empty) and in the
regular tick. Each call becomes `xor eax, eax` (nothing ran), so the flush ends at once and no scheduled tick ever
runs in game time; decoration's own calls (148215955, 148215a15; Linux 517e324, 517e3d4) stay. The original is
kept beside the exe as `.orig`. Why: docs/research/worldgen-bds-liquids.md, "Game time".
"""
import shutil
import sys

from bds_image import calls, file_offset, is_elf

STILL = b"\x31\xc0\x90\x90\x90"
WINDOWS = (0x1416042F0, [0x1409FACFB, 0x1409FAD28, 0x1409FC307, 0x1409FC3C8])
LINUX = (0xC8E4770, [0xC9E2AF7, 0xC9E2B20, 0xC9E51C9, 0xC9E5263])


def main():
    exe, check = sys.argv[1], "--check" in sys.argv
    data = bytearray(open(exe, "rb").read())
    target, sites = LINUX if is_elf(data) else WINDOWS
    offsets = [file_offset(data, va) for va in sites]
    if all(bytes(data[o : o + 5]) == STILL for o in offsets):
        print("already still")
        return
    for va, o in zip(sites, offsets):
        if calls(data, va) != target:
            raise SystemExit(f"{va:x} is not a call of {target:x}: another BDS build?")
        data[o : o + 5] = STILL
    if check:
        print("unpatched; the four call sites match")
        return
    shutil.copyfile(exe, exe + ".orig")
    open(exe, "wb").write(data)
    print(f"patched {len(sites)} call sites; original in {exe}.orig")


main()
