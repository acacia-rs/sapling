"""Patches a BDS copy so every default-constructed `Random` is seeded with 0 instead of system entropy.

usage: python3 -s bds_seed0.py <bedrock_server(.exe) of your own copy> [--check]

`Random::Random()` (Windows 1411889d0, Linux ed49c60) asks the platform for four random bytes (call 141ccbe30 at
141188a3a; Linux edd9bb0 at ed49cbb) into a stack word it has just zeroed; the call becomes nops, so the seed is 0.
The per-thread generator that picks `MultifaceFeature`'s faces and spread directions is such a Random: with this
patch and `max-threads=1` a world is a function of the seed and the chunk requests alone. The original is kept as
`.seed.orig`. Why: docs/research/worldgen-bds-order.md.
"""
import shutil
import sys

from bds_image import calls, file_offset, is_elf

NOPS = b"\x0f\x1f\x44\x00\x00"


def main():
    exe, check = sys.argv[1], "--check" in sys.argv
    data = bytearray(open(exe, "rb").read())
    site, target = (0xED49CBB, 0xEDD9BB0) if is_elf(data) else (0x141188A3A, 0x141CCBE30)
    o = file_offset(data, site)
    if bytes(data[o : o + 5]) == NOPS:
        print("already seeded with 0")
        return
    if calls(data, site) != target:
        raise SystemExit(f"{site:x} is not a call of {target:x}: another BDS build?")
    if check:
        print("unpatched; the call site matches")
        return
    shutil.copyfile(exe, exe + ".seed.orig")
    data[o : o + 5] = NOPS
    open(exe, "wb").write(data)
    print(f"patched; original in {exe}.seed.orig")


main()
