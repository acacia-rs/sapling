"""Scan effective Java .nbt templates: per family block counts and the distinct Java states."""
import collections
import gzip
import os
import struct
import sys

BP = os.path.join(os.environ['BDS'], 'behavior_packs')


def version(pack):
    if pack == 'vanilla':
        return (0,)
    return tuple(int(p) for p in pack.split('_')[1].split('.'))


def effective():
    files = {}
    packs = sorted((p for p in os.listdir(BP) if p == 'vanilla' or p.startswith('vanilla_')), key=version)
    for pack in packs:
        root = os.path.join(BP, pack, 'structures')
        for dirpath, _, names in os.walk(root):
            for n in names:
                if n.endswith('.nbt'):
                    full = os.path.join(dirpath, n)
                    files[os.path.relpath(full, root).replace('\\', '/')] = full
    return files


class Reader:
    def __init__(self, data):
        self.d, self.i = data, 0

    def take(self, fmt):
        v = struct.unpack_from('>' + fmt, self.d, self.i)
        self.i += struct.calcsize('>' + fmt)
        return v[0]

    def string(self):
        n = self.take('H')
        s = self.d[self.i:self.i + n].decode('utf-8')
        self.i += n
        return s

    def payload(self, t):
        if t == 1: return self.take('b')
        if t == 2: return self.take('h')
        if t == 3: return self.take('i')
        if t == 4: return self.take('q')
        if t == 5: return self.take('f')
        if t == 6: return self.take('d')
        if t == 7:
            n = self.take('i'); v = self.d[self.i:self.i + n]; self.i += n; return v
        if t == 8: return self.string()
        if t == 9:
            et = self.take('b'); n = self.take('i')
            return [self.payload(et) for _ in range(n)]
        if t == 10:
            out = {}
            while True:
                et = self.take('b')
                if et == 0: return out
                k = self.string()
                out[k] = self.payload(et)
        if t == 11:
            n = self.take('i'); return [self.take('i') for _ in range(n)]
        if t == 12:
            n = self.take('i'); return [self.take('q') for _ in range(n)]
        raise ValueError(t)


def load(path):
    data = gzip.decompress(open(path, 'rb').read())
    r = Reader(data)
    assert r.take('b') == 10
    r.string()
    return r.payload(10)


def state_key(s):
    props = s.get('Properties', {})
    return s['Name'] + ('[' + ','.join(f'{k}={v}' for k, v in sorted(props.items())) + ']' if props else '')


if __name__ == '__main__':
    families = sys.argv[1:]
    files = effective()
    states = collections.Counter()
    for fam in families:
        n_blocks = n_files = 0
        for rel, full in sorted(files.items()):
            if not rel.startswith(fam + '/'):
                continue
            t = load(full)
            n_files += 1
            n_blocks += len(t['blocks'])
            pal = t.get('palette') or t['palettes'][0]
            for b in t['blocks']:
                states[state_key(pal[b['state']])] += 1
        print(fam, n_files, 'files', n_blocks, 'blocks')
    print(len(states), 'distinct states')
    for k, v in sorted(states.items()):
        print(f'{v:8} {k}')
