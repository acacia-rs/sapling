"""Breakpoints on the Linux BDS under gdb (the Linux side of bds_dbg.py; docs/research/worldgen-bds-debugger.md).

Loaded by gdb (`tools/bds-drill.sh` with BDS_GDB=<config.json>, or by hand):
    gdb -q -batch -ex 'set disable-randomization on' -x bds_gdb.py -ex run --args ./bedrock_server
Config: {"log": "<out file>", "breakpoints": [...]}, each breakpoint
    {"at": "0xcf63820", "label": "ore tick",
     "mem": [["rsi", 8, "iii"], ["rdi", 8, "Q", [0]]],   # register, offset, struct format, pointer hops first
     "box": [0, x0, y0, z0, x1, y1, z1],                  # keep hits whose mem[0] lies in the box
     "backtrace": 4}                                      # return addresses, ELF-relative
Addresses are the ELF's own (bds_elf.py prints them); the image loads at BASE with randomization off. Each
hit is one line: wall time, label, thread, the mem values, the backtrace.
"""

import json
import os
import struct
import time

import gdb

BASE = 0x555555554000
REGS = ("rax", "rbx", "rcx", "rdx", "rsi", "rdi", "rbp", "rsp", "r8", "r9", "r10", "r11", "r12", "r13", "r14", "r15")

cfg = json.load(open(os.environ["BDS_GDB_CFG"]))
out = open(cfg.get("log", "gdb-hits.log"), "a", buffering=1)


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

    def stop(self):
        spec, inferior = self.spec, gdb.selected_inferior()
        try:
            values = [read(inferior, *m) for m in spec.get("mem", [])]
        except gdb.MemoryError as error:
            values = [f"unreadable: {error}"]
        box = spec.get("box")
        if box and not isinstance(values[0], tuple):
            return False
        if box and not all(lo <= v <= hi for v, lo, hi in zip(values[box[0]], box[1:4], box[4:7])):
            return False
        line = [f"{time.time():.3f}", spec.get("label", spec["at"]), str(gdb.selected_thread().num)]
        line += [" ".join(map(str, v)) if isinstance(v, tuple) else v for v in values]
        if spec.get("regs"):
            line += [f"{r}={int(gdb.parse_and_eval('$' + r)) & 0xFFFFFFFFFFFFFFFF:#x}" for r in spec["regs"] if r in REGS]
        if spec.get("backtrace"):
            line.append("bt " + " ".join(backtrace(spec["backtrace"])))
        out.write(" | ".join(line) + "\n")
        return False


for name in ("SIGPIPE", "SIGUSR1", "SIGUSR2", "SIG32", "SIG33", "SIG34", "SIG35"):
    gdb.execute(f"handle {name} nostop noprint pass", to_string=True)
gdb.execute("set pagination off")
gdb.execute("set confirm off")
for spec in cfg["breakpoints"]:
    Logged(spec)
out.write(f"# {time.time():.3f} armed {len(cfg['breakpoints'])} breakpoints\n")
