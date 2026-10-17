# Bone meal

Measured 2026-10-11 with `bds_command_probe bonemeal`
(`crates/sapling/examples/shared/use_drill.rs`): the actor, in creative on stone and looking east,
clicks the top of a plant a block east of it with bone meal, several times. Captures:
`research/captures/bds-bonemeal.txt` and `sapling-bonemeal.txt`. Sapling's side is
`blocks/bonemeal.rs`.

A click that takes sends `LevelEvent ParticleCropGrowth`, data 0, from the corner of the clicked
cell, before the blocks change.

## What BDS did, in creative

| Plant | A click |
| --- | --- |
| wheat, carrots, beetroot at growth 0 | growth 7 at once; each later click still sent the particles |
| melon stem at growth 0 | growth 3, 6, 7; the fourth click sent nothing |
| sweet berry bush at growth 0 | growth 3; the next click picked it, as a bare hand does |
| oak sapling | a tree at once, its logs where the sapling was; later clicks sent nothing |
| short grass | tall grass, both halves |
| poppy | two more poppies on the grass beside it |
| grass block with air over it | short grass over it and on eight cells within two of it, a fern and an azure bluet among them; a second click, the grass now covered, sent nothing |
| sugar cane one high | three high (on sand without water, where it then broke) |
| bamboo sapling | bamboo with a small-leaved bamboo over it; a click more, one more bamboo on top |
| stone | nothing |

## Sapling

The same, for all but the tree: a sapling takes the bone meal and stays a sapling. Where grass
and flowers come up around a grass block or a flower, and which, is sapling's own scatter: tries
at random cells within two (one for a flower), a sprout in eight a flower.

## In survival

Measured with `bds_command_probe bonemeal2` (`bds-bonemeal2.txt`), eight clicks a plant:

| Plant | What BDS did |
| --- | --- |
| wheat at growth 0 | growth 2, 4, 6, 7; the four clicks after that sent nothing |
| carrots, beetroot | ripe at the second click, then nothing |
| melon stem | growth 3, then two more steps to 7, then nothing |
| sweet berry bush | a stage a click to 3; the click after picked it |
| oak sapling | `age_bit` set at the first click, the particles alone for five more, a tree at the seventh |

Sapling grows a crop 2 to 5 stages a click, the Java game's step, which these fit, and a bush
one; a ripe crop takes none and spends none, where in creative it still took it. A sapling
takes it as BDS's `SaplingBlock::onFertilized` does (saplings.md): a tree at once in creative,
else 45 in 100 a click to age and then to grow.

## Left

- Kelp, sea grass, vines, mushrooms, cocoa, moss, rooted dirt and the other plants that take it.
- A click on sugar cane or bamboo that is fully grown.
