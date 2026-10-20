"""jbastion.py <bast_rules.txt> <bast_elems.txt> <bast_pools.txt> > src/structure/jigsaw/pools_bastion.txt: the
bastion's processors (1481118c0), elements (14811b3f0) and pools (1480ed800) in pools.txt's format (jdata.py).

The inputs are jpools.py's emulation written out per line (`r` lines; `name factory location processors
[bytes]` elements; `p` pools). The emulation lost the five mob elements and the hoglin stable's starting pool,
which are read off the disassembly here (MOBS, STARTING_STAIRS)."""
import re
import sys

# 1481315f0 / 145e17b00 at 1481294b4..148129d90: plain elements of the Bedrock mob templates, rigid, no
# processors, no foundation flags; `bastion/mobs/empty` names no template.
MOBS = ['sword_piglin', 'crossbow_piglin', 'hoglin', 'melee_piglin', 'melee_piglin_always', 'empty']
STARTING_STAIRS = ' '.join(f'bastion_hoglin_stable_starting_pieces_starting_stairs_{i}*1' for i in range(5))
POSITION = re.compile(r'pos=0x14a89af30\([^)]*\)')


def short(name):
    return name.replace('minecraft:', '') if name and name not in ('0', 'None') else '-'


def main():
    rules, elements, pools = (open(p).read().splitlines() for p in sys.argv[1:4])
    print('# BDS hardcoded jigsaw data (bastion remnant); scripts/jigsaw_extract/jbastion.py')
    for line in rules:
        print(POSITION.sub('lp=y:0.0:0.05:0:100', line))
    for line in elements:
        name, factory, location, procs, flags = line.split(' ', 4)
        if name.startswith('bastion_mobs_'):
            continue
        flags = [int(b) for b in re.findall(r'\d+', flags)]
        projection, post = flags[-2:]
        print(f'e {name} t {location} {short(procs)} {projection} {post} -')
    for mob in MOBS:
        print(f'e bastion_mobs_{mob} t bastion/mobs/{mob} - 0 0 -')
    for line in pools:
        line = re.sub(r'\(145e22c70: fallback r8\)', 'empty', line)
        if line.startswith('p bastion/hoglin_stable/starting_pieces '):
            line = f'p bastion/hoglin_stable/starting_pieces - {STARTING_STAIRS}'
        print(line)


if __name__ == '__main__':
    main()
