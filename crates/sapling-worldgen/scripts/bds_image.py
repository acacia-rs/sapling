"""File offsets of static addresses in a BDS executable, Windows (PE, image base 0x140000000) or Linux (ELF, PIE)."""
import struct


def is_elf(data):
    return data[:4] == b"\x7fELF"


def file_offset(data, va):
    if is_elf(data):
        phoff, = struct.unpack_from("<Q", data, 0x20)
        phentsize, phnum = struct.unpack_from("<HH", data, 0x36)
        for i in range(phnum):
            kind, _, offset, vaddr, _, filesz = struct.unpack_from("<IIQQQQ", data, phoff + i * phentsize)
            if kind == 1 and vaddr <= va < vaddr + filesz:
                return va - vaddr + offset
        raise SystemExit(f"{va:x} is in no loaded segment")
    base = 0x140000000
    pe = struct.unpack_from("<I", data, 0x3C)[0]
    count, optional = struct.unpack_from("<H", data, pe + 6)[0], struct.unpack_from("<H", data, pe + 20)[0]
    for i in range(count):
        size, rva, _, raw = struct.unpack_from("<IIII", data, pe + 24 + optional + 40 * i + 8)
        if rva <= va - base < rva + size:
            return va - base - rva + raw
    raise SystemExit(f"{va:x} is in no section")


def calls(data, va):
    """The target of the `call rel32` at va, or None."""
    o = file_offset(data, va)
    return va + 5 + struct.unpack_from("<i", data, o + 1)[0] if data[o] == 0xE8 else None
