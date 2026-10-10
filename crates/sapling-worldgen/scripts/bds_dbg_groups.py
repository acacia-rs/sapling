"""Breakpoint groups for bds_dbg.py (`group` / `arms` in its docstring): int3s planted only while one call runs."""
import struct

from bds_dbg_win import Context, set_hw_breakpoint


class Groups:
    def __init__(self, debugger):
        self.dbg = debugger
        self.sites = {}  # group -> addresses
        self.armed = {}  # group -> (thread id, rsp at the arming entry, return address)

    def add(self, group, addr):
        self.sites.setdefault(group, []).append(addr)

    def live(self, bp):
        """Whether the breakpoint's int3 is in memory now."""
        return "group" not in bp or bp["group"] in self.armed

    def owner(self, bp):
        """The thread whose call armed the breakpoint's group, or None."""
        entry = self.armed.get(bp.get("group"))
        return entry and entry[0]

    def owns_any(self, tid):
        return any(owner == tid for owner, _, _ in self.armed.values())

    def arm(self, group, tid, rsp):
        """Plants the group's int3s until the call entered at `rsp` returns on this thread."""
        if group in self.armed:
            return
        dbg = self.dbg
        ret = struct.unpack("<Q", dbg.mem(rsp, 8))[0]
        self.armed[group] = (tid, rsp, ret)
        set_hw_breakpoint(dbg.thread(tid), ret)
        for addr in self.sites.get(group, []):
            dbg.poke(addr, b"\xcc")

    def on_return(self, tid, addr):
        """Disarms the groups whose arming call this thread has just returned from."""
        dbg = self.dbg
        for group, (owner, rsp, ret) in list(self.armed.items()):
            if owner != tid or ret != addr:
                continue
            ctx = Context(dbg.thread(tid))
            if ctx["rsp"] <= rsp:  # a nested call's return to the same address
                ctx.set_resume()
                ctx.save()
                continue
            del self.armed[group]
            set_hw_breakpoint(dbg.thread(tid), 0)
            for site in self.sites.get(group, []):
                dbg.poke(site, dbg.bps[site][1])
