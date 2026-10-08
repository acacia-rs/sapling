"""jemu.py <start hex> <end hex> [out.json]: straight-line emulation of BDS's hardcoded jigsaw registration
functions (village / ancient city pools, elements, processors). Follows forward jmps, ignores conditional
branches (fast paths), models stack/heap bytes enough to rebuild std::strings, and logs the calls that
register things, with resolved arguments."""
import os
import re
import sys

sys.path.insert(0, os.environ['BDSRE'])  # the bdsre checkout: its core.py reads BDS_EXE
import core  # noqa: E402

ALLOC_HINT = 0x14cabeab8
GUARD = 0x14adaa220

REG64 = ['rax', 'rbx', 'rcx', 'rdx', 'rsi', 'rdi', 'rbp', 'rsp', 'r8', 'r9', 'r10', 'r11', 'r12', 'r13', 'r14', 'r15']
ALIAS = {}
for r in REG64:
    base = r[1:] if r.startswith('r') and r[1:2].isalpha() else r
    if r in ('rax', 'rbx', 'rcx', 'rdx'):
        ALIAS.update({'e' + r[1:]: r, r[1:]: r, r[1] + 'l': r})
    elif r in ('rsi', 'rdi', 'rbp', 'rsp'):
        ALIAS.update({'e' + r[1:]: r, r[1:]: r, r[1:] + 'l': r})
    else:
        ALIAS.update({r + 'd': r, r + 'w': r, r + 'b': r})
SIZE = {'byte': 1, 'word': 2, 'dword': 4, 'qword': 8, 'xmmword': 16}


def canon(r):
    return ALIAS.get(r, r)


class Emu:
    def __init__(self):
        self.regs = {}
        self.mem = {}  # (base, off) -> byte value (int) or ('obj', value) for qword-sized objects
        self.objs = {}  # (base, off) -> object value stored by a qword write
        self.heap = 0
        self.rbp_off = 0
        self.events = []
        self.recent_alloc = 0
        self.vectors = {}
        self.src = {}  # register -> stack offset it was last loaded from (vector end pointers)  # stack offset of vector begin -> list of (value, weight)

    # addresses -------------------------------------------------------------------------------
    def addr(self, expr):
        """'[rbp + 0x10]' -> (base, off)."""
        m = re.match(r'\[(\w+)(?: ([+-]) (0x[0-9a-f]+|\d+))?(?: \+ (\w+)(?:\*(\d))?)?\]', expr)
        if not m:
            return None
        reg, sign, disp, idx, scale = m.groups()
        off = int(disp, 0) if disp else 0
        if sign == '-':
            off = -off
        if idx:
            return None
        if reg == 'rip':
            return None
        reg = canon(reg)
        if reg == 'rbp':
            return ('S', off)
        if reg == 'rsp':
            return ('S', off - self.rbp_off)
        v = self.regs.get(reg)
        if isinstance(v, tuple) and v[0] in ('S', 'H'):
            return (v[0], v[1] + off) if v[0] == 'S' else (v[0], v[1], v[2] + off)
        if isinstance(v, tuple) and v[0] == 'V':
            return ('V', v[1], off)
        if reg in self.src:
            return ('V', self.src[reg], off)
        return None

    def write_bytes(self, a, bs):
        for i, b in enumerate(bs):
            self.mem[self.shift(a, i)] = b
            self.objs.pop(self.shift(a, i), None)

    @staticmethod
    def shift(a, i):
        return (a[0], a[1] + i) if a[0] == 'S' else (a[0], a[1], a[2] + i)

    def write(self, a, size, value):
        if a is None:
            return
        if a[0] == 'V':
            vec = self.vectors.setdefault(a[1], [])
            if a[2] == 0:
                vec.append([value, None])
            elif a[2] == 8 and vec and isinstance(value, int):
                vec[-1][1] = value
            return
        if isinstance(value, int):
            self.write_bytes(a, (value & ((1 << (8 * size)) - 1)).to_bytes(size, 'little'))
        elif isinstance(value, tuple) and value[0] == 'bytes':
            self.write_bytes(a, value[1][:size])
        else:
            for i in range(size):
                self.mem.pop(self.shift(a, i), None)
            self.objs[a] = value

    def read(self, a, size):
        if a is None:
            return None
        if a in self.objs:
            return self.objs[a]
        bs = [self.mem.get(self.shift(a, i)) for i in range(size)]
        if any(b is None for b in bs):
            return None
        return int.from_bytes(bytes(bs), 'little')

    def string_at(self, a):
        """A std::string object at address a."""
        if a is None:
            return None
        size, cap = self.read(self.shift(a, 0x10), 8), self.read(self.shift(a, 0x18), 8)
        if not isinstance(size, int) or not isinstance(cap, int) or size > 4096:
            return None
        src = a
        if cap >= 16:
            src = self.objs.get(a)
            if not (isinstance(src, tuple) and src[0] == 'H'):
                return None
        bs = [self.mem.get(self.shift(src, i)) for i in range(size)]
        if any(b is None for b in bs):
            return None
        return bytes(bs).decode(errors='replace')

    # values ----------------------------------------------------------------------------------
    def value(self, op):
        op = op.strip()
        if op.startswith(('0x', '-0x')) or op.lstrip('-').isdigit():
            return int(op, 0)
        if op in ALIAS or op in REG64:
            return self.regs.get(canon(op))
        return None

    def arg_str(self, v):
        if isinstance(v, tuple) and v[0] == 'cstr':
            return v[1]
        if isinstance(v, tuple) and v[0] in ('S', 'H'):
            return self.string_at(v)
        return None

    def deref(self, v, size=8):
        if isinstance(v, tuple) and v[0] in ('S', 'H'):
            return self.read(v, size)
        return None

    # main loop -------------------------------------------------------------------------------
    def run(self, a, b, handlers):
        insns = list(core.disasm(a, b, 10_000_000))
        index = {ins.address: i for i, ins in enumerate(insns)}
        i = 0
        while i < len(insns):
            ins = insns[i]
            i += 1
            mn, ops = ins.mnemonic, ins.op_str
            parts = [p.strip() for p in re.split(r',(?![^\[]*\])', ops)] if ops else []
            kept = None
            if parts and 'ptr' not in parts[0] and mn not in ('cmp', 'test', 'push', 'call') and not mn.startswith('j'):
                kept = self.src.pop(canon(parts[0]), None)
            if mn == 'lea' and parts[0] == 'rbp' and 'rsp' in parts[1]:
                self.rbp_off = int(re.search(r'0x[0-9a-f]+', parts[1]).group(0), 16)
                self.regs['rbp'] = ('S', 0)
                continue
            if mn == 'jmp':
                try:
                    t = int(ops, 16)
                except ValueError:
                    continue
                if t > ins.address and t in index:
                    i = index[t]
                continue
            if mn.startswith('j') or mn in ('cmp', 'test', 'nop', 'push', 'pop', 'int3', 'ret'):
                continue
            if mn == 'call':
                self.call(ins, handlers)
                continue
            rip = core.rip_target(ins) if '[rip' in ops else None
            if mn in ('movups', 'movaps', 'movdqu', 'movdqa'):
                dst, src = parts
                if dst.startswith('xmm') and 'ptr' not in dst:
                    if rip is not None:
                        o = core.va_to_off(rip)
                        self.regs[dst] = ('bytes', bytes(core.data[o:o + 16]))
                    else:
                        a2 = self.addr(src.split('ptr ')[-1])
                        bs = [self.mem.get(self.shift(a2, k)) for k in range(16)] if a2 else [None]
                        self.regs[dst] = ('bytes', bytes(bs)) if None not in bs else None
                else:
                    a2 = self.addr(dst.split('ptr ')[-1])
                    self.write(a2, 16, self.regs.get(src))
                continue
            if mn == 'xorps' or mn == 'pxor':
                self.regs[parts[0]] = ('bytes', bytes(16))
                continue
            if mn == 'lea':
                dst = canon(parts[0])
                if rip is not None:
                    s = core.cstr(rip, 300)
                    self.regs[dst] = ('cstr', s) if s else ('va', rip)
                else:
                    self.regs[dst] = self.addr(parts[1])
                continue
            if mn in ('mov', 'movabs', 'movzx', 'movsxd', 'movsx'):
                dst, src = parts
                if 'ptr' in dst:
                    size = SIZE[dst.split()[0]]
                    a2 = self.addr(dst.split('ptr ')[-1])
                    v = self.value(src)
                    self.write(a2, size, v)
                    continue
                d = canon(dst)
                if 'ptr' in src:
                    size = SIZE[src.split()[0]]
                    if rip is not None:
                        if rip == ALLOC_HINT:
                            self.regs[d] = ('global', rip)
                            self.recent_alloc = 4
                        else:
                            o = core.va_to_off(rip)
                            self.regs[d] = int.from_bytes(bytes(core.data[o:o + size]), 'little') if o is not None else None
                        continue
                    a2 = self.addr(src.split('ptr ')[-1])
                    v = self.read(a2, size)
                    if v is None and a2 is not None and a2[0] in ('S', 'H') and size == 8:
                        v = ('V', a2)
                    self.regs[d] = v
                    if a2 is not None and a2[0] in ('S', 'H') and size == 8 and not isinstance(v, tuple):
                        self.src[d] = a2
                    else:
                        self.src.pop(d, None)
                    continue
                else:
                    self.regs[d] = self.value(src)
                continue
            if mn in ('add', 'sub', 'inc', 'dec', 'and', 'or', 'xor', 'shl', 'shr'):
                d = canon(parts[0])
                if mn == 'xor' and len(parts) == 2 and canon(parts[1]) == d:
                    self.regs[d] = 0
                elif 'ptr' not in parts[0]:
                    v = self.regs.get(d)
                    k = self.value(parts[1]) if len(parts) > 1 else 1
                    if isinstance(v, tuple) and v[0] == 'S' and isinstance(k, int) and mn in ('add', 'sub'):
                        self.regs[d] = ('S', v[1] + (k if mn == 'add' else -k))
                    elif isinstance(v, tuple) and v[0] == 'H' and isinstance(k, int) and mn in ('add', 'sub'):
                        self.regs[d] = ('H', v[1], v[2] + (k if mn == 'add' else -k))
                    elif isinstance(v, tuple) and v[0] == 'V' and mn in ('add', 'sub'):
                        self.regs[d] = v
                    else:
                        self.regs[d] = None
                        if kept is not None and mn in ('add', 'sub'):
                            self.src[d] = kept
                continue
            if mn in ('cmovge', 'cmovae', 'cmovb', 'cmovl', 'cmovg', 'cmovle', 'cmove', 'cmovne', 'sete', 'setne'):
                self.regs[canon(parts[0])] = None
                continue

    def call(self, ins, handlers):
        target = None
        if ins.op_str.startswith('0x'):
            target = int(ins.op_str, 16)
        elif '[rip' in ins.op_str and core.rip_target(ins) is not None:
            target = core.rip_target(ins)
        args = {r: self.regs.get(r) for r in ('rcx', 'rdx', 'r8', 'r9')}
        args['stack'] = [self.read(('S', 0x20 + 8 * k - self.rbp_off), 8) for k in range(4)]
        args['stack_addr'] = [('S', 0x20 + 8 * k - self.rbp_off) for k in range(4)]
        result = None
        if target == GUARD or target is None:
            if self.recent_alloc:
                self.heap += 1
                result = ('H', self.heap, 0)
        elif target in handlers:
            result = handlers[target](self, ins.address, args)
        else:
            self.events.append({'at': f'{ins.address:x}', 'call': f'{target:x}'})
        self.recent_alloc = max(0, self.recent_alloc - 1) if result is None else 0
        for r in ('rax', 'rcx', 'rdx', 'r8', 'r9', 'r10', 'r11'):
            self.regs[r] = None
            self.src.pop(r, None)
        for x in [k for k in self.regs if k.startswith('xmm') and k not in ('xmm6', 'xmm7', 'xmm8', 'xmm9', 'xmm10', 'xmm11', 'xmm12', 'xmm13', 'xmm14', 'xmm15')]:
            self.regs[x] = None
        self.regs['rax'] = result
