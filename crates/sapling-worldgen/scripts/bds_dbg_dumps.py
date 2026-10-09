"""Memory dumps for bds_dbg.py (its docstring describes the config): `dump`, `dump_ptr` and `dump_on_return`."""
import struct


class Dumps:
    def __init__(self, debugger):
        self.dbg = debugger
        self.pending = {}  # thread id -> calls whose buffer is written when they return
        self.count = 0

    def write(self, name, address, size):
        with open(f"{self.dbg.out}/{name}.bin", "wb") as f:
            f.write(self.dbg.mem(address, size))

    def on_match(self, tid, bp, ctx, pos):
        """A matching hit: writes its ranges, and remembers its output buffer until the call returns."""
        dbg, tag = self.dbg, f"{bp['name']}-{'_'.join(map(str, pos or ()))}"
        for i, (reg, off, size) in enumerate(bp.get("dump", [])):
            self.write(f"mem-{tag}-{i}", ctx[reg] + off, size)
        for i, (reg, off, size) in enumerate(bp.get("dump_ptr", [])):
            self.write(f"ptr-{tag}-{i}", struct.unpack("<Q", dbg.mem(ctx[reg] + off, 8))[0], size)
        if "dump_on_return" in bp:
            reg, size = bp["dump_on_return"]
            ret = struct.unpack("<Q", dbg.mem(ctx["rsp"], 8))[0]
            self.pending.setdefault(tid, []).append((ctx["rsp"], ctx[reg], size, tag))
            if ret not in dbg.bps:
                dbg.bps[ret] = ({"returns": True}, dbg.mem(ret, 1))
                dbg.poke(ret, b"\xcc")

    def on_return(self, tid, ctx):
        """Writes the buffers of this thread's calls that have returned to dump-<n>-<name>.bin."""
        calls = self.pending.get(tid, [])
        while calls and ctx["rsp"] > calls[-1][0]:
            _, pointer, size, tag = calls.pop()
            self.count += 1
            self.write(f"dump-{self.count}-{tag}", pointer, size)
            self.dbg.log.write(f"dump {self.count} {tag} t{tid}\n")
