"""Writes data/block-resistance.tsv: each Block's mExplosionResistance (+0xac) out of a running Windows BDS.

Blocks are found by their BlockType (+0x68), whose name HashedString sits at BlockType+0xe0 (FNV-1 64 hash,
then an MSVC std::string); the Blocks are the objects pointing there that share the commonest vtable. Offsets
are LeviLamina's for 1.26 and the explosion code's reads (bdsre: Explosion::explode at 1.26.52.3's 140795690).
Usage: start bedrock_server.exe, wait for "Server started", then
py -3 data/generate/block-resistance.py <its pid> data/block-resistance.tsv
"""
import ctypes
import ctypes.wintypes as wt
import struct
import sys
from collections import Counter, defaultdict

import numpy as np

PROCESS_VM_READ, PROCESS_QUERY_INFORMATION = 0x10, 0x400
MEM_COMMIT, MEM_PRIVATE = 0x1000, 0x20000
READABLE = {0x02, 0x04, 0x20, 0x40}


class MBI(ctypes.Structure):
    _fields_ = [("BaseAddress", ctypes.c_void_p), ("AllocationBase", ctypes.c_void_p), ("AllocationProtect", wt.DWORD), ("PartitionId", wt.WORD),
                ("RegionSize", ctypes.c_size_t), ("State", wt.DWORD), ("Protect", wt.DWORD), ("Type", wt.DWORD)]


k32 = ctypes.WinDLL("kernel32", use_last_error=True)
k32.OpenProcess.restype = wt.HANDLE
k32.VirtualQueryEx.argtypes = [wt.HANDLE, ctypes.c_void_p, ctypes.POINTER(MBI), ctypes.c_size_t]
k32.ReadProcessMemory.argtypes = [wt.HANDLE, ctypes.c_void_p, ctypes.c_void_p, ctypes.c_size_t, ctypes.POINTER(ctypes.c_size_t)]


def fnv64(data: bytes) -> int:
    h = 0xCBF29CE484222325
    for b in data:
        h = ((h * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF) ^ b
    return h


def main():
    pid, out = int(sys.argv[1]), sys.argv[2]
    proc = k32.OpenProcess(PROCESS_VM_READ | PROCESS_QUERY_INFORMATION, False, pid)
    regions, addr, mbi = [], 0, MBI()
    while k32.VirtualQueryEx(proc, ctypes.c_void_p(addr), ctypes.byref(mbi), ctypes.sizeof(mbi)):
        base, size = mbi.BaseAddress or 0, mbi.RegionSize
        if mbi.State == MEM_COMMIT and mbi.Type == MEM_PRIVATE and (mbi.Protect & 0xFF) in READABLE:
            buf = ctypes.create_string_buffer(size)
            got = ctypes.c_size_t()
            if k32.ReadProcessMemory(proc, ctypes.c_void_p(base), buf, size, ctypes.byref(got)) and got.value == size:
                regions.append((base, buf.raw))
        addr = base + size
    print(f"{len(regions)} regions, {sum(len(r) for _, r in regions) >> 20} MiB", file=sys.stderr)

    import bisect
    regions.sort()
    bases = [base for base, _ in regions]

    def read(a, n):
        k = bisect.bisect_right(bases, a) - 1
        if k >= 0:
            base, raw = regions[k]
            if a + n <= base + len(raw):
                return raw[a - base:a - base + n]
        return None

    def q(a):
        b = read(a, 8)
        return None if b is None else struct.unpack("<Q", b)[0]

    def msvc_string(a):
        b = read(a, 32)
        if b is None:
            return None
        size, cap = struct.unpack("<QQ", b[16:32])
        if size > 200 or cap < size:
            return None
        if cap < 16:
            return b[:size]
        return read(struct.unpack("<Q", b[:8])[0], size)

    # BlockType candidates: a HashedString of a "minecraft:" name at T+0xe0.
    types = {}
    for base, raw in regions:
        start = 0
        while (i := raw.find(b"minecraft:", start)) >= 0:
            start = i + 1
            a = base + i
            # SSO string object at a, or a heap buffer at a pointed to by a string object.
            for obj in [a] if a % 8 == 0 else []:
                name = msvc_string(obj)
                if name and name.startswith(b"minecraft:") and q(obj - 8) == fnv64(name):
                    types[obj - 8 - 0xE0] = name.decode()
    heap_names = {}
    for base, raw in regions:
        start = 0
        while (i := raw.find(b"minecraft:", start)) >= 0:
            start = i + 1
            heap_names[base + i] = None
    keys = np.array(sorted(heap_names), dtype=np.uint64)
    for base, raw in regions:
        arr = np.frombuffer(raw[: len(raw) // 8 * 8], dtype=np.uint64)
        for j in np.nonzero(np.isin(arr, keys))[0]:
            obj = base + int(j) * 8
            name = msvc_string(obj)
            if name and name.startswith(b"minecraft:") and q(obj - 8) == fnv64(name):
                types[obj - 8 - 0xE0] = name.decode()
    print(f"{len(types)} named HashedStrings", file=sys.stderr)

    tkeys = np.array(sorted(types), dtype=np.uint64)
    blocks = []
    for base, raw in regions:
        arr = np.frombuffer(raw[: len(raw) // 8 * 8], dtype=np.uint64)
        for j in np.nonzero(np.isin(arr, tkeys))[0]:
            block = base + int(j) * 8 - 0x68
            head = read(block, 0xB8)
            if head is None:
                continue
            vt = struct.unpack("<Q", head[:8])[0]
            blocks.append((block, vt, types[int(arr[j])], struct.unpack("<f", head[0xAC:0xB0])[0], struct.unpack("<f", head[0xB4:0xB8])[0]))
    vts = Counter(b[1] for b in blocks)
    print("vtables", vts.most_common(5), file=sys.stderr)
    vt = vts.most_common(1)[0][0]
    per = defaultdict(Counter)
    for _, v, name, resist, _destroy in blocks:
        if v == vt:
            per[name][round(resist, 4)] += 1
    # Every permutation of a block held one value in BDS 1.26.52; a second would need a state column.
    split = [name for name in per if len(per[name]) > 1]
    assert not split, f"resistance varies by state: {split}"
    with open(out, "w", newline="\n") as f:
        f.write(HEADER)
        for name in sorted(per):
            f.write(f"{name}\t{next(iter(per[name])):g}\n")
    print(f"{len(per)} block types", file=sys.stderr)


HEADER = """# Each block's explosion resistance as BDS holds it (Block::mDirectData.mExplosionResistance, +0xac, which
# Explosion::explode subtracts as (resistance + 0.3) x 0.3 a step), read out of a running Windows BDS 1.26.52.3 by
# data/generate/block-resistance.py.
"""


main()
