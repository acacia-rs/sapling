"""usage: py -3 -I cells.py <feature_trace txt> <'rule try N' substring> <bds moss list> [up|down]
Per cell of the try: our depth and how many moss blocks BDS has from the cell away from the open side."""
import sys

trace, key, moss_path = sys.argv[1:4]
step = 1 if (sys.argv[4] if len(sys.argv) > 4 else 'up') == 'up' else -1
moss = set()
for line in open(moss_path):
    f = line.split()
    moss.add((int(f[0]), int(f[1]), int(f[2])))
on = False
i = 0
for line in open(trace):
    if not line.startswith(' '):
        if on:
            break
        on = key in line
        continue
    if on and line.startswith('  cell'):
        f = line.split()
        x, y, z, d = int(f[1]), int(f[2]), int(f[3]), int(f[5])
        b = 0
        while (x, y + b * step, z) in moss:
            b += 1
        i += 1
        print(i, x, y, z, 'ours', d, 'bds', b, '' if d == b else '<<<')
