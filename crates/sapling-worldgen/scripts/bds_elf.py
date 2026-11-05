"""Virtual tables of the stripped Linux BDS (clang, Itanium ABI), read off its RTTI and relocations.

The ELF keeps no symbols, but every polymorphic class keeps its typeinfo name ("16RedStoneOreBlock"), and
its vtable is a run of R_X86_64_RELATIVE relocations next to a pointer to that typeinfo. Addresses printed are
the ELF's own (PIE, base 0); gdb with ASLR off loads the image at 0x555555554000. A BlockEvents subscription
(`subscribe<RedStoneOreBlock, BlockRandomTickEvent>`) is a libc++ std::function: name its mangled `__func`
type (`strings` finds it); slot 6 is its operator(), entered with the event in rsi.

usage: bds_elf.py <bedrock_server> vtable <Class> [<BaseClass>]   slots, '*' where they differ from the base's
       bds_elf.py <bedrock_server> slot <Class> <index>            one slot's function
       bds_elf.py <bedrock_server> refs <hex address>              relocations whose addend is the address
"""

import mmap
import re
import struct
import sys

import numpy as np

R_X86_64_RELATIVE = 8


class Elf:
    def __init__(self, path):
        f = open(path, "rb")
        self.m = mmap.mmap(f.fileno(), 0, access=mmap.ACCESS_READ)
        shoff, = struct.unpack_from("<Q", self.m, 0x28)
        shentsize, shnum, shstrndx = struct.unpack_from("<HHH", self.m, 0x3A)
        heads = [struct.unpack_from("<IIQQQQIIQQ", self.m, shoff + i * shentsize) for i in range(shnum)]
        names = heads[shstrndx][4]
        self.sections = {}
        for h in heads:
            end = self.m.find(b"\0", names + h[0])
            self.sections[self.m[names + h[0]:end].decode()] = (h[3], h[4], h[5])  # addr, offset, size
        _, off, size = self.sections[".rela.dyn"]
        rela = np.frombuffer(self.m, dtype=np.dtype([("off", "<u8"), ("info", "<u8"), ("add", "<i8")]), count=size // 24, offset=off)
        rela = rela[(rela["info"] & 0xFFFFFFFF) == R_X86_64_RELATIVE]
        self.at = dict(zip(rela["off"].tolist(), rela["add"].tolist()))
        self.by_addend = {}
        for o, a in self.at.items():
            self.by_addend.setdefault(a, []).append(o)

    def typeinfo_name(self, cls):
        """The address of the typeinfo name of `cls`: a plain class name, or a whole mangled one
        (`NSt3__110__function6__funcI...E` for a std::function's lambda)."""
        _, off, size = self.sections[".rodata"]
        for name in (b"%d%s" % (len(cls), cls.encode()), cls.encode()):
            hit = re.compile(rb"\0(%s)\0" % re.escape(name)).search(self.m, off, off + size)
            if hit:
                return hit.start(1)  # .rodata's file offset equals its address in the first LOAD
        raise SystemExit(f"no typeinfo name for {cls}")

    def typeinfo(self, cls):
        refs = self.by_addend.get(self.typeinfo_name(cls), [])
        return [r - 8 for r in refs]  # {vptr, name, base...}: the name pointer is the second word

    def vtable(self, cls):
        """Slots of the primary vtable of `cls`, from its first virtual function on."""
        for ti in self.typeinfo(cls):
            for slot in self.by_addend.get(ti, []):
                if self.at.get(slot - 8, 1) != 0 and (slot - 8) in self.at:
                    continue  # a typeinfo's base pointer, not a vtable's
                slots, addr = [], slot + 8
                while addr in self.at:
                    slots.append(self.at[addr])
                    addr += 8
                if slots:
                    return slot + 8, slots
        raise SystemExit(f"no vtable for {cls}")


def main():
    elf = Elf(sys.argv[1])
    cmd = sys.argv[2]
    if cmd == "vtable":
        point, slots = elf.vtable(sys.argv[3])
        base = elf.vtable(sys.argv[4])[1] if len(sys.argv) > 4 else []
        print(f"vtable {sys.argv[3]} at {point:#x}, {len(slots)} slots")
        for i, f in enumerate(slots):
            mark = "*" if base and (i >= len(base) or base[i] != f) else " "
            print(f"{mark} {i:4d} vt+{i * 8:#06x} {f:#x}")
    elif cmd == "slot":
        print(f"{elf.vtable(sys.argv[3])[1][int(sys.argv[4])]:#x}")
    elif cmd == "refs":
        print("\n".join(f"{r:#x}" for r in elf.by_addend.get(int(sys.argv[3], 16), [])))


if __name__ == "__main__":
    main()
