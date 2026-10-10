"""template_dump.py <bds> <template path, e.g. ancient_city/city_center/city_center_1> <name part,...>

The template's blocks whose Java name holds one of the parts ('' for all), in file order: index, position,
name, properties, and a jigsaw's final state and pool. For setting a BDS cell beside what its template saved.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from templates_to_rust import effective, load  # noqa: E402


def main():
    bds, path, parts = sys.argv[1], sys.argv[2], sys.argv[3].split(',')
    template = load(effective(bds)[path])
    palette = template.get('palette') or template['palettes'][0]
    print('size', template['size'], 'blocks', len(template['blocks']))
    for i, block in enumerate(template['blocks']):
        state = palette[block['state']]
        if not any(part in state['Name'] for part in parts):
            continue
        props = ','.join(f'{k}={v}' for k, v in sorted(state.get('Properties', {}).items()))
        nbt = block.get('nbt', {})
        jigsaw = f" final={nbt.get('final_state', '')} pool={nbt.get('pool', '')}" if 'jigsaw' in state['Name'] else ''
        print(i, tuple(block['pos']), state['Name'].replace('minecraft:', ''), props + jigsaw)


if __name__ == '__main__':
    main()
