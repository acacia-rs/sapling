"""Minimal Win32 debugger for BDS: int3 breakpoints at static VAs (image base 0x140000000), register/memory logs,
and single-step traces of a function call when a position argument matches.

usage: py -3 -s bds_dbg.py <exe> <config.json> <out dir>    (stdin/stdout pass through to the exe)
Run through bds_dbg_run.sh; recipe and examples in docs/research/worldgen-bds-debugger.md.
Modules: bds_dbg_win.py (Win32, thread contexts), bds_dbg_hits.py (filters and what a hit logs),
bds_dbg_groups.py (group / arms), bds_dbg_dumps.py (dump*).

config: {"breakpoints": [{"va": "0x143b7ff60", "name": "place", "regs": ["rax"], "mem": [["r8", 8, "iii"]],
          "filter": ["r8", 8, [[-5309, 5447]]], "trace": true, "log": "match"|"all"|"none",
          "strings": [["rax", 3]], "backtrace": 12, "stack": 100}]}
filter: three ints at [reg+off] are x, y, z; the hit matches when (x, z) is in the list, or inside
  {"box": [x0, z0, x1, z1]} or one of {"boxes": [[x0, z0, x1, z1], ...]}. No filter: every hit matches. filter2: two ints (a ChunkPos) in the list, or any
  with null for the list (the position still names the dumps). filter_format "fff" reads the three as floats.
dump: [[reg, off, size], ...]: on a matching hit, writes each range to mem-<name>-<pos>-<i>.bin (a later hit
  at the same position overwrites it).
dump_ptr: the same from the pointer stored at [reg+off], to ptr-<name>-<pos>-<i>.bin.
dump_on_return: ["rdx", size]: on a matching hit, remembers the pointer in the register and writes `size`
  bytes from it to dump-<n>-<name>-<pos>.bin when the call returns (an output buffer).
xmm: indices of xmm registers whose low float is logged (a float result or argument at that instruction).
mem: struct formats read at [reg+off]. strings: printable strings within N pointer hops of a register (a Block*
  shows its name; air shows unrelated names). backtrace: return addresses by StackWalk64 (exact). stack: every
  image address among the first N stack words (includes stale ones).
trace: single-steps the hitting thread until the call returns; writes trace-<n>.txt lines "va rax".
names: like strings, cached per pointer value (for immutable objects such as a Block*).
probe: a Python file whose `probe(dbg, ctx)` runs once, at the first matching hit, with the process stopped
  (dbg.mem, dbg.base, dbg.out; scripts/dbg_blocks.py walks the block registry with it).
group / arms: a breakpoint with "group": "g" is planted only while a matching hit of a breakpoint with "arms": "g"
  (a function entry) is running, and logs that thread alone; it is removed when that call returns. Use it for hot
  code (a setBlock, a per-cell callee) that is wanted inside one feature call only.
Addresses in the log are static VAs. dbg.log holds one line per logged hit, in order.
"""
import ctypes as C
import json
import os
import struct
import subprocess
import sys

import bds_dbg_hits as hits
from bds_dbg_dumps import Dumps
from bds_dbg_groups import Groups
from bds_dbg_win import BREAKPOINT, DBG_CONTINUE, DBG_NOT_HANDLED, REGS, SINGLE_STEP, STATIC_BASE, Context, k32


class Debugger:
    def __init__(self, exe, config, out):
        self.cfg = config
        self.exe = exe
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
        self.dumps = Dumps(self)
        self.groups = Groups(self)
        self.inspector = hits.Inspector(self)

    def mem(self, addr, size):
        buf = C.create_string_buffer(size)
        n = C.c_size_t()
        k32.ReadProcessMemory(self.hproc, addr, buf, size, C.byref(n))
        return buf.raw[: n.value]

    def poke(self, addr, data):
        n = C.c_size_t()
        k32.WriteProcessMemory(self.hproc, addr, data, len(data), C.byref(n))
        k32.FlushInstructionCache(self.hproc, addr, len(data))

    def rva(self, addr):
        return addr - self.base + STATIC_BASE

    def arm(self):
        for bp in self.cfg["breakpoints"]:
            addr = int(bp["va"], 16) - STATIC_BASE + self.base
            self.bps[addr] = (bp, self.mem(addr, 1))
            if "group" in bp:
                self.groups.add(bp["group"], addr)
            else:
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
                    self.groups.on_return(tid, addr)
                elif exc == SINGLE_STEP and self.groups.owns_any(tid):
                    self.groups.on_return(tid, addr)
                elif exc != BREAKPOINT:
                    status = DBG_NOT_HANDLED
            k32.ContinueDebugEvent(pid, tid, status)
        self.log.write("exit\n")

    def on_breakpoint(self, tid, addr):
        bp, orig = self.bps[addr]
        ctx = Context(self.thread(tid))
        ctx["rip"] = addr
        if not self.groups.live(bp):
            # Its group was disarmed after this thread ran into the int3: the original byte is back.
            ctx.save()
            return
        if self.stepping_over and not self.push_reg(addr, orig):
            # Another thread is mid-step with this one asleep: run into this int3 again once it is done.
            ctx.save()
            return
        if bp.get("returns"):
            self.dumps.on_return(tid, ctx)
            self.execute_original(tid, ctx, addr, orig)
            return
        if self.groups.owner(bp) not in (None, tid):
            self.execute_original(tid, ctx, addr, orig)
            return
        pos, match = hits.match(self, bp, ctx)
        mode = bp.get("log", "match")
        if mode == "all" or mode == "match" and match:
            self.log.write(f"hit {bp['name']} t{tid} pos={pos} match={match} {' '.join(self.inspector.parts(bp, ctx))}\n")
        if match:
            self.on_match(tid, bp, ctx, pos)
        self.execute_original(tid, ctx, addr, orig)

    def on_match(self, tid, bp, ctx, pos):
        """A matching hit's dumps, probe, group arming and trace start."""
        self.dumps.on_match(tid, bp, ctx, pos)
        if "probe" in bp:
            scope = {}
            # A relative probe path names a file beside this script.
            exec(open(os.path.join(os.path.dirname(os.path.abspath(__file__)), bp.pop("probe"))).read(), scope)
            scope["probe"](self, ctx)
        if "arms" in bp:
            self.groups.arm(bp["arms"], tid, ctx["rsp"])
        if bp.get("trace") and tid not in self.traces:
            self.n_trace += 1
            ret = struct.unpack("<Q", self.mem(ctx["rsp"], 8))[0]
            f = open(f"{self.out}/trace-{self.n_trace}.txt", "w")
            f.write(f"# {bp['name']} pos={pos} t{tid}\n")
            self.traces[tid] = {"ret": ret, "rsp": ctx["rsp"], "file": f}

    def push_reg(self, addr, orig):
        """The register a `push r64` at addr pushes and the instruction's size, or None for anything else."""
        op = orig[0]
        if 0x50 <= op <= 0x57:
            return REGS[op - 0x50], 1
        if op == 0x41 and 0x50 <= self.mem(addr + 1, 1)[0] <= 0x57:  # REX.B: r8..r15
            return REGS[self.mem(addr + 1, 1)[0] - 0x48], 2
        return None

    def execute_original(self, tid, ctx, addr, orig):
        push = self.push_reg(addr, orig)
        if push:  # emulate so the int3 never leaves memory
            reg, size = push
            value = ctx[reg]
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
            if self.groups.live(self.bps[addr][0]):
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
