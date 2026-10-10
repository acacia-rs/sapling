"""Win32 bindings for bds_dbg.py: kernel32 and dbghelp prototypes, debug event codes and a thread's x64 CONTEXT."""
import ctypes as C
import struct
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
CONTEXT_SIZE = 1232
REGS = ["rax", "rcx", "rdx", "rbx", "rsp", "rbp", "rsi", "rdi", "r8", "r9", "r10", "r11", "r12", "r13", "r14", "r15"]
REG_OFF = {r: 0x78 + 8 * i for i, r in enumerate(REGS)} | {"rip": 0xF8}


def aligned_context():
    """A 16-byte aligned buffer for a CONTEXT, and its address."""
    raw = C.create_string_buffer(CONTEXT_SIZE + 16)
    return raw, (C.addressof(raw) + 15) & ~15


class Context:
    """An aligned x64 CONTEXT (control + integer)."""

    def __init__(self, thread):
        self.thread = thread
        self.raw, self.addr = aligned_context()
        C.memmove(self.addr + 0x30, struct.pack("<I", 0x10000B), 4)
        if not k32.GetThreadContext(thread, self.addr):
            raise OSError(C.get_last_error())

    def __getitem__(self, reg):
        return struct.unpack("<Q", C.string_at(self.addr + REG_OFF[reg], 8))[0]

    def __setitem__(self, reg, value):
        C.memmove(self.addr + REG_OFF[reg], struct.pack("<Q", value & (2**64 - 1)), 8)

    def xmm(self, index):
        """The low float of an xmm register (FltSave.XmmRegisters at 0x1a0)."""
        return struct.unpack("<f", C.string_at(self.addr + 0x1A0 + 16 * index, 4))[0]

    def _or_flags(self, bits):
        flags = struct.unpack("<I", C.string_at(self.addr + 0x44, 4))[0] | bits
        C.memmove(self.addr + 0x44, struct.pack("<I", flags), 4)

    def set_trap(self):
        self._or_flags(0x100)

    def set_resume(self):
        """RF: the next instruction runs without its hardware breakpoint firing."""
        self._or_flags(0x10000)

    def save(self):
        if not k32.SetThreadContext(self.thread, self.addr):
            raise OSError(C.get_last_error())


def set_hw_breakpoint(thread, addr):
    """An execute breakpoint for one thread alone (DR0); 0 clears it."""
    _raw, at = aligned_context()
    C.memmove(at + 0x30, struct.pack("<I", 0x100010), 4)
    k32.GetThreadContext(thread, at)
    dr7 = struct.unpack("<Q", C.string_at(at + 0x70, 8))[0] & ~0xF0003
    C.memmove(at + 0x48, struct.pack("<Q", addr), 8)
    C.memmove(at + 0x70, struct.pack("<Q", dr7 | bool(addr)), 8)
    k32.SetThreadContext(thread, at)
