# Third-party material

sapling is MIT-licensed ([LICENSE](LICENSE)). It includes or derives from these MIT-licensed works.

| Source | Copyright | Used in |
|---|---|---|
| [Cubitect/cubiomes](https://github.com/Cubitect/cubiomes), [xpple/cubiomes](https://github.com/xpple/cubiomes) | (c) 2020 Cubitect | `sapling-worldgen/src/biome/btree/` (generated from `btree262` by `scripts/btree_to_rust.py`); the density terms in `sapling-worldgen/src/shaper/overworld.rs` (`terrainnoise.c`) |
| [stirante/featurelab](https://github.com/stirante/featurelab) | (c) 2026 stirante | the `sapling-worldgen/src/feature/` files that name it in their header (trees, ores, vegetation patches, growing plants and others) |
| [df-mc/dragonfly](https://github.com/df-mc/dragonfly) | (c) 2019 Dragonfly Tech | server design and game values, as noted in [docs/DESIGN.md](docs/DESIGN.md) and the source |

Each is used under the MIT License:

> Permission is hereby granted, free of charge, to any person obtaining a copy of this software and
> associated documentation files (the "Software"), to deal in the Software without restriction, including
> without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
> copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the
> following conditions:
>
> The above copyright notice and this permission notice shall be included in all copies or substantial
> portions of the Software.
>
> THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT
> LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO
> EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER
> IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE
> USE OR OTHER DEALINGS IN THE SOFTWARE.

Other projects (Dragonfly, featurelab, cubiomes, ChunkBiomes, BedrockUtil, pmmp/BedrockData) are cited in
comments and research notes for how the game behaves; beyond the table above, no code is taken from them.

Mojang's game data is not part of this repository and is generated locally from your own copy of the
game ([crates/sapling-data/README.md](crates/sapling-data/README.md#provenance)).
