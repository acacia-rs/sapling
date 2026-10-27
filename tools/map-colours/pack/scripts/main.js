// On `/scriptevent mapcol:grid [raised]` from a player: holds the 128x128 blocks around the world's
// middle loaded, lays every block type's default state south of the player on the ground's level (or
// one higher with `raised`), one cell every other block both ways, and prints where each went with its
// minecraft:map_color component (where it has one); see ../run.sh.
import { system, BlockTypes, BlockPermutation } from "@minecraft/server";

const HALF = 64;
const FIRST_ROW = -30;
const PER_ROW = HALF;
const log = (text) => console.warn("MAPCOL " + text);
const ticks = (n) => new Promise((resolve) => system.runTimeout(resolve, n));
// Whole floats: a tint can sit a hair under a whole byte, which the map's truncation shows.
const rgba = (c) => (c ? [c.red, c.green, c.blue, c.alpha].map((v) => v * 255).join(",") : "-");

function component(block) {
  try {
    const found = block.getComponent("minecraft:map_color");
    return found ? ` comp=${rgba(found.color)}|${found.tintMethod}|${rgba(found.tintedColor)}` : "";
  } catch (e) {
    return ` comp-error=${e}`;
  }
}

function loaded(dimension, y) {
  for (const x of [-HALF, HALF - 1]) {
    for (const z of [-HALF, HALF - 1]) {
      try {
        if (!dimension.getBlock({ x, y, z })) return false;
      } catch {
        return false;
      }
    }
  }
  return true;
}

// Rows every other block from FIRST_ROW south, skipping the three rows about the player.
function cell(i) {
  const row = Math.floor(i / PER_ROW);
  const z = FIRST_ROW + 2 * row;
  return [-HALF + 2 * (i % PER_ROW), z >= -2 ? z + 6 : z];
}

async function grid(dimension, feet, raised) {
  const ground = Math.floor(feet.y) - 1;
  dimension.runCommand(`tickingarea add -${HALF} ${ground} -${HALF} ${HALF - 1} ${ground} ${HALF - 1} mapcol`);
  for (let i = 0; i < 100 && !loaded(dimension, ground); i++) await ticks(5);
  const types = BlockTypes.getAll().map((type) => type.id).sort();
  types.forEach((id, i) => {
    // Raised on odd cells: a step up from the ground north of it shows at full brightness only
    // there (BDS's float shading puts an even cell's step at the threshold). Liquids stay down.
    const up = raised && !/water|lava/.test(id);
    const [x, z] = cell(i).map((c, axis) => (axis === 0 && raised ? c + 1 : c));
    const y = up ? ground + 1 : ground;
    try {
      const block = dimension.getBlock({ x, y, z });
      if (!block) return log(`cell ${x} ${z} ${id} unloaded`);
      block.setPermutation(BlockPermutation.resolve(id));
      log(`cell ${x} ${z} ${id}${component(block)}`);
    } catch (e) {
      log(`cell ${x} ${z} ${id} error ${e}`);
    }
  });
  log(`done ${types.length} at y ${ground}${raised ? " raised" : ""}`);
}

const TINT_SAMPLES = ["minecraft:grass_block", "minecraft:oak_leaves", "minecraft:water"];

// `/scriptevent mapcol:tint <biome>` from a player standing in that biome (the map drill's
// `!tint` locates it): a grass block, oak leaves and water set at its feet, their map_color
// tinted colours printed.
function tint(player, biome) {
  const at = { x: Math.floor(player.location.x), y: Math.floor(player.location.y), z: Math.floor(player.location.z) };
  try {
    const found = TINT_SAMPLES.map((name, i) => {
      const block = player.dimension.getBlock({ x: at.x + i, y: at.y, z: at.z });
      block.setPermutation(BlockPermutation.resolve(name));
      return `${name.slice(10)}${component(block)}`;
    });
    log(`tint ${biome} at ${at.x} ${at.y} ${at.z} ${found.join(" ")}`);
  } catch (e) {
    log(`tint ${biome} error ${e}`);
  }
}

system.afterEvents.scriptEventReceive.subscribe((event) => {
  if (event.id === "mapcol:grid" && event.sourceEntity) {
    grid(event.sourceEntity.dimension, event.sourceEntity.location, event.message === "raised").catch((e) => log(`done: failed ${e}`));
  }
  if (event.id === "mapcol:tint" && event.sourceEntity) {
    tint(event.sourceEntity, event.message);
  }
});
