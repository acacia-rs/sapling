"""What bds_dbg.py reads at a breakpoint hit: the position filter and the logged line's parts (its docstring
describes the config fields)."""
import ctypes as C
import struct

from bds_dbg_win import CONTEXT_SIZE, aligned_context, dbghelp


def match(dbg, bp, ctx):
    """(position or None, whether the hit passes the breakpoint's filter)."""
    if "filter2" in bp:  # two ints (a ChunkPos)
        reg, off, xzs = bp["filter2"]
        pos = struct.unpack("<ii", dbg.mem(ctx[reg] + off, 8))
        return pos, xzs is None or list(pos) in xzs
    if "filter" not in bp:
        return None, True
    reg, off, xzs = bp["filter"]
    pos = struct.unpack("<" + bp.get("filter_format", "iii"), dbg.mem(ctx[reg] + off, 12))
    if isinstance(xzs, dict):  # {"box": [x0, z0, x1, z1]} or {"boxes": [[x0, z0, x1, z1], ...]}
        boxes = xzs.get("boxes") or [xzs["box"]]
        return pos, any(x0 <= pos[0] <= x1 and z0 <= pos[2] <= z1 for x0, z0, x1, z1 in boxes)
    return pos, [pos[0], pos[2]] in xzs


class Inspector:
    def __init__(self, debugger):
        self.dbg = debugger
        self.sym = False
        self.names = {}  # (pointer, depth) -> strings

    def strings(self, addr, depth, seen=None):
        """Printable strings reachable from addr through up to `depth` pointer hops (offsets in hex)."""
        seen = set() if seen is None else seen
        found = []
        data = self.dbg.mem(addr, 0x100)
        for off in range(0, len(data) - 7, 8):
            p = struct.unpack_from("<Q", data, off)[0]
            if p < 0x10000 or p > 0x7FFFFFFFFFFF or p in seen:
                continue
            seen.add(p)
            text = self.dbg.mem(p, 48).split(b"\0")[0]
            if len(text) >= 4 and all(32 <= c < 127 for c in text):
                found.append(f"+{off:x}:{text.decode()}")
            elif depth > 1:
                inner = self.strings(p, depth - 1, seen)
                if inner:
                    found.append(f"+{off:x}{{{inner}}}")
        return " ".join(found)

    def backtrace(self, ctx, depth):
        """Return addresses by StackWalk64 over the exe's unwind data (the context is copied)."""
        dbg = self.dbg
        if not self.sym:
            dbghelp.SymInitialize(dbg.hproc, None, False)
            dbghelp.SymLoadModuleEx(dbg.hproc, None, dbg.exe.encode(), None, dbg.base, 0, None, 0)
            self.sym = True
        _copy, addr = aligned_context()
        C.memmove(addr, ctx.addr, CONTEXT_SIZE)
        frame = C.create_string_buffer(0x400)
        for off, reg in ((0, "rip"), (32, "rbp"), (48, "rsp")):
            struct.pack_into("<QHxxI", frame, off, ctx[reg], 0, 3)
        table, modbase = dbghelp.SymFunctionTableAccess64, dbghelp.SymGetModuleBase64
        out = []
        for _ in range(depth):
            if not dbghelp.StackWalk64(0x8664, dbg.hproc, ctx.thread, frame, addr, None, C.cast(table, C.c_void_p), C.cast(modbase, C.c_void_p), None):
                break
            out.append(struct.unpack_from("<Q", frame, 0)[0])
        return out

    def parts(self, bp, ctx):
        """The fields a logged hit shows after its position."""
        dbg = self.dbg
        parts = [f"{r}={ctx[r]:x}" for r in bp.get("regs", [])]
        parts += [f"xmm{i}={ctx.xmm(i)!r}/{struct.unpack('<I', struct.pack('<f', ctx.xmm(i)))[0]:08x}" for i in bp.get("xmm", [])]
        for reg, off, fmt in bp.get("mem", []):
            parts.append(f"[{reg}+{off:x}]={struct.unpack('<' + fmt, dbg.mem(ctx[reg] + off, struct.calcsize(fmt)))}")
        if bp.get("backtrace"):
            parts.append("bt=" + ",".join(f"{dbg.rva(a):x}" for a in self.backtrace(ctx, bp["backtrace"])))
        if bp.get("stack"):
            words = struct.unpack(f"<{bp['stack']}Q", dbg.mem(ctx["rsp"], 8 * bp["stack"]))
            code = [f"{dbg.rva(w):x}" for w in words if dbg.base <= w < dbg.base + 0xD000000]
            parts.append("stack=" + ",".join(code))
        for reg, depth in bp.get("strings", []):
            parts.append(f"{reg}->{self.strings(ctx[reg], depth)}")
        for reg, depth in bp.get("names", []):
            key = (ctx[reg], depth)
            if key not in self.names:
                self.names[key] = self.strings(*key)
            parts.append(f"{reg}->{self.names[key]}")
        return parts
