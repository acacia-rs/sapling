// Loads every chunk of REGIONS tile by tile and touches its bedrock floor so BDS saves it; see ../../README.md.
import { world, system } from "@minecraft/server";
import { REGIONS, TILE, WORKERS, SETTLE } from "./regions.js";

const FLOOR = -64;
const LOAD_TIMEOUT = 600;
const LOAD_ATTEMPTS = 4;
const log = (text) => console.warn("WORLDGEN " + text);
const ticks = (n) => new Promise((resolve) => system.runTimeout(resolve, n));

function tilesOf([x0, z0, x1, z1]) {
  const tiles = [];
  for (let x = x0; x <= x1; x += TILE)
    for (let z = z0; z <= z1; z += TILE) tiles.push([x, z, Math.min(x + TILE - 1, x1), Math.min(z + TILE - 1, z1)]);
  return tiles;
}

const blocks = ([x0, z0, x1, z1]) => `${x0 * 16} ${FLOOR} ${z0 * 16} ${x1 * 16 + 15} ${FLOOR} ${z1 * 16 + 15}`;

function command(dimension, text) {
  const count = dimension.runCommand(text).successCount;
  if (!count) throw new Error(`"${text}" did nothing`);
  return count;
}

function loaded(dimension, cx, cz) {
  try {
    const block = dimension.getBlock({ x: cx * 16, y: FLOOR, z: cz * 16 });
    return block !== undefined && (typeof block.isValid === "function" ? block.isValid() : block.isValid);
  } catch {
    return false;
  }
}

function unloaded(dimension, [x0, z0, x1, z1]) {
  const chunks = [];
  for (let x = x0; x <= x1; x++) for (let z = z0; z <= z1; z++) if (!loaded(dimension, x, z)) chunks.push(`${x},${z}`);
  return chunks;
}

// Some areas stall with a few chunks never loaded (seed 20261003, chunks 12,14..16); adding the area afresh loads them.
async function loadArea(dimension, area, name) {
  for (let attempt = 1; ; attempt++) {
    command(dimension, `tickingarea add ${blocks(area)} ${name}`);
    for (let waited = 0; waited <= LOAD_TIMEOUT; waited += 5) {
      if (unloaded(dimension, area).length === 0) return;
      await ticks(5);
    }
    const missing = unloaded(dimension, area);
    dimension.runCommand(`tickingarea remove ${name}`);
    if (attempt === LOAD_ATTEMPTS) throw new Error(`chunks ${missing.slice(0, 8).join(" ")} not loaded after ${attempt} tries`);
    log(`${name} retry: chunks ${missing.slice(0, 8).join(" ")} not loaded`);
    await ticks(20);
  }
}

// The ticking area reaches one chunk past the tile, so every touched chunk's neighbours have decorated it.
async function runTile(dimension, tile, name) {
  const [x0, z0, x1, z1] = tile;
  await loadArea(dimension, [x0 - 1, z0 - 1, x1 + 1, z1 + 1], name);
  try {
    await ticks(SETTLE);
    // fill reports no block count, so read the floor: all bedrock, then all stone, then all bedrock again.
    const cells = (x1 - x0 + 1) * (z1 - z0 + 1) * 256;
    const floor = (type) => {
      let count = 0;
      for (let x = x0 * 16; x <= x1 * 16 + 15; x++)
        for (let z = z0 * 16; z <= z1 * 16 + 15; z++) count += dimension.getBlock({ x, y: FLOOR, z })?.typeId === type ? 1 : 0;
      return count;
    };
    const counts = [floor("minecraft:bedrock")];
    command(dimension, `fill ${blocks(tile)} stone replace bedrock`);
    counts.push(floor("minecraft:stone"));
    command(dimension, `fill ${blocks(tile)} bedrock replace stone`);
    counts.push(floor("minecraft:bedrock"));
    if (counts.some((count) => count !== cells)) throw new Error(`floor bedrock/stone/bedrock ${counts.join("/")} of ${cells}`);
    return cells / 256;
  } finally {
    dimension.runCommand(`tickingarea remove ${name}`);
  }
}

async function runAll(dimension) {
  const tiles = REGIONS.flatMap(tilesOf);
  let next = 0;
  let chunks = 0;
  let failed = 0;
  const worker = async () => {
    while (next < tiles.length) {
      const index = next++;
      try {
        const touched = await runTile(dimension, tiles[index], "wg" + index);
        chunks += touched;
        log(`tile ${index + 1}/${tiles.length} ${tiles[index].join(",")} ok`);
      } catch (error) {
        failed++;
        log(`tile ${index + 1}/${tiles.length} ${tiles[index].join(",")} error ${String(error).replace(/\s+/g, " ")}`);
      }
    }
  };
  await Promise.all(Array.from({ length: WORKERS }, worker));
  log(`done ${chunks} chunks touched, ${failed} tiles failed`);
}

world.afterEvents.worldLoad.subscribe(() => {
  const dimension = world.getDimension("overworld");
  // Ticking areas tick their chunks: stop everything that would change blocks or add mobs meanwhile.
  for (const rule of ["randomtickspeed 0", "domobspawning false", "doweathercycle false", "dofiretick false", "dodaylightcycle false"])
    dimension.runCommand("gamerule " + rule);
  dimension.runCommand("weather clear");
  runAll(dimension).catch((error) => log("done error " + String(error).replace(/\s+/g, " ")));
});
