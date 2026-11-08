"""Breakpoints on the Linux BDS under gdb (the Linux side of bds_dbg.py; docs/research/worldgen-bds-debugger.md).

Loaded by gdb (`tools/bds-drill.sh` with BDS_GDB=<config.json>, or by hand):
    gdb -q -batch -ex 'set disable-randomization on' -x bds_gdb.py -ex run --args ./bedrock_server
Config: {"log": "<out file>", "breakpoints": [...]}, each breakpoint
    {"at": "0xcf63820", "label": "ore tick",
     "mem": [["rsi", 8, "iii"], ["rdi", 8, "Q", [0]]],   # register, offset, struct format, pointer hops first
     "box": [0, x0, y0, z0, x1, y1, z1],                  # keep hits whose mem[0] lies in the box
     "backtrace": 4,                                      # return addresses, ELF-relative
     "gates": "draws", "when": [[-263], [-392]],          # enable breakpoints labelled "draws" while mem == when
     "gated": true,                                       # start disabled, for a gates breakpoint to enable
     "watch": [{"label": "pos z", "expr": "*(long*)($rdi+0x208)+8", "fmt": "f", "limit": 40}],
     "rearm": false}                                      # hardware write watchpoints armed at each hit
                                                          # (replacing the last set; rearm false: once)
Addresses are the ELF's own (bds_elf.py prints them); the image loads at BASE with randomization off. Each
hit is one line: wall time, label, thread, the mem values, the backtrace. With "hits": true in the config the
lines are bds_dbg.py's (`hit <label> t<thread> ... rax=<hex> [reg+off]=(...)`), which dbg_loads.py reads
(data/dbg_canon_linux.json). BDS_GDB_LOG overrides the config's log path.
"""

import json
import os
import struct
import time

import gdb

BASE = 0x555555554000
REGS = ("rax", "rbx", "rcx", "rdx", "rsi", "rdi", "rbp", "rsp", "r8", "r9", "r10", "r11", "r12", "r13", "r14", "r15")

cfg = json.load(open(os.environ["BDS_GDB_CFG"]))
out = open(os.environ.get("BDS_GDB_LOG") or cfg.get("log", "gdb-hits.log"), "a", buffering=1)


def read(inferior, reg, off, fmt, hops=()):
    addr = int(gdb.parse_and_eval(f"${reg}")) & 0xFFFFFFFFFFFFFFFF
    for hop in hops:
        addr = struct.unpack("<Q", bytes(inferior.read_memory(addr + hop, 8)))[0]
    raw = bytes(inferior.read_memory(addr + off, struct.calcsize("<" + fmt)))
    return struct.unpack("<" + fmt, raw)


def backtrace(depth):
    frames, frame = [], gdb.newest_frame().older()
    while frame is not None and len(frames) < depth:
        frames.append(f"{frame.pc() - BASE:#x}")
        frame = frame.older()
    return frames


class Logged(gdb.Breakpoint):
    def __init__(self, spec):
        super().__init__(f"*{BASE + int(spec['at'], 16):#x}", internal=True)
        self.spec = spec
        self.enabled = not spec.get("gated")
        self.watches = []

    def arm(self):
        for old in self.watches:
            old.delete()
        self.watches = []
        for w in self.spec["watch"]:
            addr = int(gdb.parse_and_eval(w["expr"])) & 0xFFFFFFFFFFFFFFFF
            self.watches.append(Watched(w, addr, self.spec.get("backtrace", 0)))
        out.write(f"{time.time():.3f} | armed | {gdb.selected_thread().num} | "
                  + " ".join(f"{w.spec['label']}@{w.addr:#x}={w.value()}" for w in self.watches) + "\n")

    def stop(self):
        spec, inferior = self.spec, gdb.selected_inferior()
        try:
            values = [read(inferior, *m) for m in spec.get("mem", [])]
        except gdb.MemoryError as error:
            values = [f"unreadable: {error}"]
        if spec.get("gates"):
            wanted = [list(v) for v in values] == spec["when"]
            for other in LOGGED:
                if other.spec.get("label") == spec["gates"]:
                    other.enabled = wanted
        box = spec.get("box")
        if box and not isinstance(values[0], tuple):
            return False
        if box and not all(lo <= v <= hi for v, lo, hi in zip(values[box[0]], box[1:4], box[4:7])):
            return False
        if spec.get("watch") and (spec.get("rearm", True) or not self.watches):
            self.arm()
        if cfg.get("hits"):
            regs = [f"{r}={int(gdb.parse_and_eval('$' + r)) & 0xFFFFFFFFFFFFFFFF:x}" for r in spec.get("regs", [])]
            mems = [f"[{m[0]}+{m[1]:x}]={v}" for m, v in zip(spec.get("mem", []), values)]
            # "ints": registers holding an int argument (a ChunkPos passed in r8d, r9d), logged as one-int tuples
            mems += [f"[{r}]=({struct.unpack('<i', struct.pack('<I', int(gdb.parse_and_eval('$' + r)) & 0xFFFFFFFF))[0]},)"
                     for r in spec.get("ints", [])]
            out.write(" ".join([f"hit {spec['label']} t{gdb.selected_thread().num}"] + regs + mems) + "\n")
            return False
        line = [f"{time.time():.3f}", spec.get("label", spec["at"]), str(gdb.selected_thread().num)]
        line += [" ".join(map(str, v)) if isinstance(v, tuple) else v for v in values]
        if spec.get("regs"):
            line += [f"{r}={int(gdb.parse_and_eval('$' + r)) & 0xFFFFFFFFFFFFFFFF:#x}" for r in spec["regs"] if r in REGS]
        if spec.get("backtrace"):
            line.append("bt " + " ".join(backtrace(spec["backtrace"])))
        out.write(" | ".join(line) + "\n")
        return False


class Watched(gdb.Breakpoint):
    """A hardware write watchpoint: one line per change (wall time, label, thread, new value, pc, backtrace)."""

    SIZES = {1: "char", 2: "short", 4: "int", 8: "long"}

    def __init__(self, spec, addr, depth):
        self.spec, self.addr, self.depth, self.count = spec, addr, depth, 0
        size = struct.calcsize("<" + spec["fmt"])
        super().__init__(f"*({self.SIZES[size]}*){addr:#x}", gdb.BP_WATCHPOINT, gdb.WP_WRITE, internal=True)

    def value(self):
        raw = bytes(gdb.selected_inferior().read_memory(self.addr, struct.calcsize("<" + self.spec["fmt"])))
        return " ".join(map(str, struct.unpack("<" + self.spec["fmt"], raw)))

    def stop(self):
        self.count += 1
        if self.count > self.spec.get("limit", 50):
            self.enabled = False
            return False
        frame = gdb.newest_frame()
        line = [f"{time.time():.3f}", self.spec["label"], str(gdb.selected_thread().num), self.value(),
                f"pc {frame.pc() - BASE:#x}"]
        if self.depth:
            line.append("bt " + " ".join(backtrace(self.depth)))
        out.write(" | ".join(line) + "\n")
        return False


for name in ("SIGPIPE", "SIGUSR1", "SIGUSR2", "SIG32", "SIG33", "SIG34", "SIG35"):
    gdb.execute(f"handle {name} nostop noprint pass", to_string=True)
gdb.execute("set pagination off")
gdb.execute("set confirm off")
LOGGED = [Logged(spec) for spec in cfg["breakpoints"]]
out.write(f"# {time.time():.3f} armed {len(cfg['breakpoints'])} breakpoints\n")
