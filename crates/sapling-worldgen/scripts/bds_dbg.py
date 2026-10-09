"""Minimal Win32 debugger for BDS: int3 breakpoints at static VAs (image base 0x140000000), register/memory logs,
and single-step traces of a function call when a position argument matches.

usage: py -3 -s bds_dbg.py <exe> <config.json> <out dir>    (stdin/stdout pass through to the exe)
Run through bds_dbg_run.sh; recipe and examples in docs/research/worldgen-bds-debugger.md.

config: {"breakpoints": [{"va": "0x143b7ff60", "name": "place", "regs": ["rax"], "mem": [["r8", 8, "iii"]],
          "filter": ["r8", 8, [[-5309, 5447]]], "trace": true, "log": "match"|"all"|"none",
          "strings": [["rax", 3]], "backtrace": 12, "stack": 100}]}
filter: three ints at [reg+off] are x, y, z; the hit matches when (x, z) is in the list, or inside
  {"box": [x0, z0, x1, z1]} or one of {"boxes": [[x0, z0, x1, z1], ...]}. No filter: every hit matches.
mem: struct formats read at [reg+off]. strings: printable strings within N pointer hops of a register (a Block*
  shows its name; air shows unrelated names). backtrace: return addresses by StackWalk64 (exact). stack: every
  image address among the first N stack words (includes stale ones).
trace: single-steps the hitting thread until the call returns; writes trace-<n>.txt lines "va rax".
Addresses in the log are static VAs. dbg.log holds one line per logged hit, in order.
"""
import ctypes as C
import json
import struct
import subprocess
import sys
from ctypes import wintypes as W

k32 = C.WinDLL("kernel32", use_last_error=True)
k32.OpenThread.restype = W.HANDLE
k32.ReadProcessMemory.argtypes = [W.HANDLE, C.c_uint64, C.c_void_p, C.c_size_t, C.POINTER(C.c_size_t)]
k32.WriteProcessMemory.argtypes = [W.HANDLE, C.c_uint64, C.c_void_p, C.c_size_t, C.POINTER(C.c_size_t)]
k32.FlushInstructionCache.argtypes = [W.HANDLE, C.c_uint64, C.c_size_t]
k32.GetThreadContext.argtypes = [W.HANDLE, C.c_void_p]
k32.SetThreadContext.argtypes = [W.HANDLE, C.c_void_p]
k32.SuspendThread.argtypes = [W.HANDLE]
k32.ResumeThread.argtypes = [W.HANDLE]
k32.WaitForDebugEvent.argtypes = [C.c_void_p, W.DWORD]
k32.ContinueDebugEvent.argtypes = [W.DWORD, W.DWORD, W.DWORD]

dbghelp = C.WinDLL("dbghelp", use_last_error=True)
dbghelp.SymInitialize.argtypes = [W.HANDLE, C.c_char_p, W.BOOL]
dbghelp.SymLoadModuleEx.argtypes = [W.HANDLE, W.HANDLE, C.c_char_p, C.c_char_p, C.c_uint64, W.DWORD, C.c_void_p, W.DWORD]
dbghelp.SymLoadModuleEx.restype = C.c_uint64
dbghelp.SymFunctionTableAccess64.argtypes = [W.HANDLE, C.c_uint64]
dbghelp.SymFunctionTableAccess64.restype = C.c_void_p
dbghelp.SymGetModuleBase64.argtypes = [W.HANDLE, C.c_uint64]
dbghelp.SymGetModuleBase64.restype = C.c_uint64
dbghelp.StackWalk64.argtypes = [W.DWORD, W.HANDLE, W.HANDLE, C.c_void_p, C.c_void_p, C.c_void_p, C.c_void_p, C.c_void_p, C.c_void_p]
DBG_CONTINUE, DBG_NOT_HANDLED = 0x00010002, 0x80010001
BREAKPOINT, SINGLE_STEP = 0x80000003, 0x80000004
STATIC_BASE = 0x140000000
REGS = ["rax", "rcx", "rdx", "rbx", "rsp", "rbp", "rsi", "rdi", "r8", "r9", "r10", "r11", "r12", "r13", "r14", "r15"]
REG_OFF = {r: 0x78 + 8 * i for i, r in enumerate(REGS)} | {"rip": 0xF8}


class Context:
    """An aligned x64 CONTEXT (control + integer)."""

    def __init__(self, thread):
        self.thread = thread
        self.raw = C.create_string_buffer(1232 + 16)
        self.addr = (C.addressof(self.raw) + 15) & ~15
        C.memmove(self.addr + 0x30, struct.pack("<I", 0x100003), 4)
        if not k32.GetThreadContext(thread, self.addr):
            raise OSError(C.get_last_error())

    def __getitem__(self, reg):
        return struct.unpack("<Q", C.string_at(self.addr + REG_OFF[reg], 8))[0]

    def __setitem__(self, reg, value):
        C.memmove(self.addr + REG_OFF[reg], struct.pack("<Q", value & (2**64 - 1)), 8)

    def set_trap(self):
        flags = struct.unpack("<I", C.string_at(self.addr + 0x44, 4))[0] | 0x100
        C.memmove(self.addr + 0x44, struct.pack("<I", flags), 4)

    def save(self):
        if not k32.SetThreadContext(self.thread, self.addr):
            raise OSError(C.get_last_error())


class Debugger:
    def __init__(self, exe, config, out):
        self.cfg = config
        self.exe = exe
        self.sym = False
        self.out = out
        self.log = open(f"{out}/dbg.log", "w", buffering=1)
        self.proc = subprocess.Popen([exe], creationflags=0x2, cwd=None)  # DEBUG_ONLY_THIS_PROCESS
        self.hproc = None
        self.base = 0
        self.threads = {}
        self.bps = {}  # address -> (config, original byte)
        self.traces = {}  # thread id -> trace state
        self.stepping_over = {}  # thread id -> breakpoint address to re-arm
        self.n_trace = 0

    def mem(self, addr, size):
        buf = C.create_string_buffer(size)
        n = C.c_size_t()
        k32.ReadProcessMemory(self.hproc, addr, buf, size, C.byref(n))
        return buf.raw[: n.value]

    def poke(self, addr, data):
        n = C.c_size_t()
        k32.WriteProcessMemory(self.hproc, addr, data, len(data), C.byref(n))
        k32.FlushInstructionCache(self.hproc, addr, len(data))

    def strings(self, addr, depth, seen=None):
        """Printable strings reachable from addr through up to `depth` pointer hops (offsets in hex)."""
        seen = set() if seen is None else seen
        found = []
        data = self.mem(addr, 0x100)
        for off in range(0, len(data) - 7, 8):
            p = struct.unpack_from("<Q", data, off)[0]
            if p < 0x10000 or p > 0x7FFFFFFFFFFF or p in seen:
                continue
            seen.add(p)
            text = self.mem(p, 48).split(b"\0")[0]
            if len(text) >= 4 and all(32 <= c < 127 for c in text):
                found.append(f"+{off:x}:{text.decode()}")
            elif depth > 1:
                inner = self.strings(p, depth - 1, seen)
                if inner:
                    found.append(f"+{off:x}{{{inner}}}")
        return " ".join(found)

    def backtrace(self, ctx, depth):
        """Return addresses by StackWalk64 over the exe's unwind data (the context is copied)."""
        if not self.sym:
            dbghelp.SymInitialize(self.hproc, None, False)
            dbghelp.SymLoadModuleEx(self.hproc, None, self.exe.encode(), None, self.base, 0, None, 0)
            self.sym = True
        copy = C.create_string_buffer(1232 + 16)
        addr = (C.addressof(copy) + 15) & ~15
        C.memmove(addr, ctx.addr, 1232)
        frame = C.create_string_buffer(0x400)
        for off, reg in ((0, "rip"), (32, "rbp"), (48, "rsp")):
            struct.pack_into("<QHxxI", frame, off, ctx[reg], 0, 3)
        table, modbase = dbghelp.SymFunctionTableAccess64, dbghelp.SymGetModuleBase64
        out = []
        for _ in range(depth):
            if not dbghelp.StackWalk64(0x8664, self.hproc, ctx.thread, frame, addr, None, C.cast(table, C.c_void_p), C.cast(modbase, C.c_void_p), None):
                break
            out.append(struct.unpack_from("<Q", frame, 0)[0])
        return out

    def rva(self, addr):
        return addr - self.base + STATIC_BASE

    def arm(self):
        for bp in self.cfg["breakpoints"]:
            addr = int(bp["va"], 16) - STATIC_BASE + self.base
            self.bps[addr] = (bp, self.mem(addr, 1))
            self.poke(addr, b"\xcc")
        self.log.write(f"base {self.base:x}, {len(self.bps)} breakpoints\n")

    def thread(self, tid):
        if tid not in self.threads:
            self.threads[tid] = k32.OpenThread(0x1FFFFF, False, tid)
        return self.threads[tid]

    def run(self):
        ev = C.create_string_buffer(176)
        while True:
            if not k32.WaitForDebugEvent(ev, 1000):
                if self.proc.poll() is not None:
                    break
                continue
            code, pid, tid = struct.unpack_from("<III", ev, 0)
            status = DBG_CONTINUE
            if code == 3:  # CREATE_PROCESS
                self.hproc, hthread, self.base = struct.unpack_from("<QQQ", ev, 24)
                self.threads[tid] = hthread
                self.arm()
            elif code == 2:  # CREATE_THREAD
                self.threads[tid] = struct.unpack_from("<Q", ev, 16)[0]
            elif code == 4:  # EXIT_THREAD
                self.threads.pop(tid, None)
            elif code == 5:  # EXIT_PROCESS
                k32.ContinueDebugEvent(pid, tid, DBG_CONTINUE)
                break
            elif code == 1:
                exc, = struct.unpack_from("<I", ev, 16)
                addr, = struct.unpack_from("<Q", ev, 32)
                if exc == BREAKPOINT and addr in self.bps:
                    self.on_breakpoint(tid, addr)
                elif exc == SINGLE_STEP and (tid in self.traces or tid in self.stepping_over):
                    self.on_step(tid)
                elif exc != BREAKPOINT:
                    status = DBG_NOT_HANDLED
            k32.ContinueDebugEvent(pid, tid, status)
        self.log.write("exit\n")

    def on_breakpoint(self, tid, addr):
        bp, orig = self.bps[addr]
        ctx = Context(self.thread(tid))
        ctx["rip"] = addr
        if self.stepping_over and not 0x50 <= orig[0] <= 0x5F and orig[0] != 0x41:
            # Another thread is mid-step with this one asleep: run into this int3 again once it is done.
            ctx.save()
            return
        pos = None
        if "filter" in bp:
            reg, off, xzs = bp["filter"]
            pos = struct.unpack("<iii", self.mem(ctx[reg] + off, 12))
            if isinstance(xzs, dict):  # {"box": [x0, z0, x1, z1]} or {"boxes": [[x0, z0, x1, z1], ...]}
                boxes = xzs.get("boxes") or [xzs["box"]]
                match = any(x0 <= pos[0] <= x1 and z0 <= pos[2] <= z1 for x0, z0, x1, z1 in boxes)
            else:
                match = [pos[0], pos[2]] in xzs
        else:
            match = True
        mode = bp.get("log", "match")
        if mode == "all" or mode == "match" and match:
            parts = [f"{r}={ctx[r]:x}" for r in bp.get("regs", [])]
            for reg, off, fmt in bp.get("mem", []):
                parts.append(f"[{reg}+{off:x}]={struct.unpack('<' + fmt, self.mem(ctx[reg] + off, struct.calcsize(fmt)))}")
            if bp.get("backtrace"):
                parts.append("bt=" + ",".join(f"{self.rva(a):x}" for a in self.backtrace(ctx, bp["backtrace"])))
            if bp.get("stack"):
                words = struct.unpack(f"<{bp['stack']}Q", self.mem(ctx["rsp"], 8 * bp["stack"]))
                code = [f"{self.rva(w):x}" for w in words if self.base <= w < self.base + 0xD000000]
                parts.append("stack=" + ",".join(code))
            for reg, depth in bp.get("strings", []):
                parts.append(f"{reg}->{self.strings(ctx[reg], depth)}")
            self.log.write(f"hit {bp['name']} t{tid} pos={pos} match={match} {' '.join(parts)}\n")
        if match and bp.get("trace") and tid not in self.traces:
            self.n_trace += 1
            ret = struct.unpack("<Q", self.mem(ctx["rsp"], 8))[0]
            f = open(f"{self.out}/trace-{self.n_trace}.txt", "w")
            f.write(f"# {bp['name']} pos={pos} t{tid}\n")
            self.traces[tid] = {"ret": ret, "rsp": ctx["rsp"], "file": f}
        self.execute_original(tid, ctx, addr, orig)

    def execute_original(self, tid, ctx, addr, orig):
        op, size = orig[0], 1
        if op == 0x41:  # REX.B push r8..r15
            op, size = self.mem(addr + 1, 1)[0] + 8, 2
        if 0x50 <= op <= 0x5F:  # push r64: emulate so the int3 never leaves memory
            value = ctx[REGS[op - 0x50]]
            ctx["rsp"] = ctx["rsp"] - 8
            self.poke(ctx["rsp"], struct.pack("<Q", value))
            ctx["rip"] = addr + size
            if tid in self.traces:
                ctx.set_trap()
                self.traces[tid]["file"].write(f"{self.rva(addr):x} {ctx['rax']:x}\n")
            ctx.save()
            return
        # The other threads sleep while the original byte is back, or they would run through the breakpoint unseen.
        others = [h for t, h in self.threads.items() if t != tid]
        for h in others:
            k32.SuspendThread(h)
        self.poke(addr, orig)
        self.stepping_over[tid] = (addr, others)
        ctx.set_trap()
        ctx.save()

    def on_step(self, tid):
        ctx = Context(self.thread(tid))
        if tid in self.stepping_over:
            addr, others = self.stepping_over.pop(tid)
            self.poke(addr, b"\xcc")
            for h in others:
                k32.ResumeThread(h)
            if tid not in self.traces:
                return
        t = self.traces[tid]
        rip = ctx["rip"]
        if rip == t["ret"] and ctx["rsp"] > t["rsp"]:
            t["file"].write(f"ret {ctx['rax']:x}\n")
            t["file"].close()
            del self.traces[tid]
            return
        t["file"].write(f"{self.rva(rip):x} {ctx['rax']:x}\n")
        ctx.set_trap()
        ctx.save()


def main():
    exe, config, out = sys.argv[1:4]
    Debugger(exe, json.load(open(config)), out).run()


if __name__ == "__main__":
    main()
