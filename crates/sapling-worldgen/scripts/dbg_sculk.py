"""dbg_sculk.py <cx> <cz> [probe.py] > cfg.json: a bds_dbg.py config for one source chunk's sculk patches and
multiface features: their entries, and every `setBlock` call site inside them, armed for those calls only.
Run the chunk as a one-chunk region, then dbg_sculk_cmp.py sets the log beside `examples/feature_trace`."""
import json
import sys

cx, cz = int(sys.argv[1]), int(sys.argv[2])
chunk = {"box": [cx * 16, cz * 16, cx * 16 + 15, cz * 16 + 15]}
wide = {"box": [cx * 16 - 16, cz * 16 - 16, cx * 16 + 31, cz * 16 + 31]}
bps = [
    {"va": "0x143b83e50", "name": "mf", "regs": ["rcx"], "arms": "w", "filter": ["r8", 8, chunk]},
    {"va": "0x143ba12e0", "name": "patch", "arms": "w", "filter": ["r8", 8, chunk]},
    {"va": "0x143b84ccc", "name": "mfset", "filter": ["rdx", 0, wide], "mem": [["r8", 0x120, "H"]]},
    {"va": "0x148b41132", "name": "regrow", "filter": ["rdi", 0, wide], "mem": [["r8", 0x120, "H"]]},
    {"va": "0x148b42676", "name": "conv", "filter": ["rdx", 0, wide]},
    {"va": "0x148b41ae2", "name": "growth", "filter": ["rdx", 0, wide]},
    {"va": "0x148b41cee", "name": "growth", "filter": ["rdx", 0, wide]},
    {"va": "0x148b6b1f7", "name": "rmface", "filter": ["rdx", 0, wide], "mem": [["r8", 0x120, "H"]]},
    {"va": "0x143ba17e8", "name": "catalyst", "filter": ["rdx", 0, wide]},
    {"va": "0x143ba1853", "name": "shrieker", "filter": ["rdx", 0, wide]},
]
for va in ("0x1465e946d", "0x1465e9517", "0x1465e95c1", "0x1465e966b", "0x1465e9715", "0x1465e97b5"):
    bps.append({"va": va, "name": "spset", "filter": ["rdx", 0, wide], "mem": [["r8", 0x120, "H"]]})
for va in ("0x1465e9bc7", "0x1465e9c70", "0x1465e9d1b", "0x1465e9dc6", "0x1465e9e71", "0x1465e9f0b"):
    bps.append({"va": va, "name": "saset", "filter": ["rdx", 0, wide], "mem": [["r8", 0x120, "H"]], "backtrace": 2})
for bp in bps[2:]:
    bp["group"] = "w"
if len(sys.argv) > 3:
    bps[1]["probe"] = sys.argv[3]
print(json.dumps({"breakpoints": bps}, indent=1))
