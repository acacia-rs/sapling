# Third-party notices

sapling contains code ported or generated from the projects below. Each is under the MIT License,
reproduced once at the end of this file with its copyright line.

| Project | Copyright | Used in |
|---|---|---|
| [Cubitect/cubiomes](https://github.com/Cubitect/cubiomes) (and the [xpple/cubiomes](https://github.com/xpple/cubiomes) fork) | Copyright (c) 2020 Cubitect | `crates/sapling-worldgen/src/biome/btree/` (generated from `btree262`), `crates/sapling-worldgen/src/shaper/overworld.rs` (`terrainnoise.c`) |
| [stirante/featurelab](https://github.com/stirante/featurelab) | Copyright (c) 2026 stirante | the `crates/sapling-worldgen/src/feature/` files that name it in their header |
| [df-mc/dragonfly](https://github.com/df-mc/dragonfly) | Copyright (c) 2019 Dragonfly Tech | server design and game values, as noted in `docs/DESIGN.md` and the source |

## MIT License

Copyright (c) 2020 Cubitect<br>
Copyright (c) 2026 stirante<br>
Copyright (c) 2019 Dragonfly Tech

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

## Minecraft

sapling is not an official Minecraft product and is not approved by or associated with Mojang or
Microsoft. The game data it needs at build time is not in this repository; see
`crates/sapling-data/README.md` "Provenance".
