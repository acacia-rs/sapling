// Loads each candidate temple, finds the probe template's three markers and dumps every probe cell's block.
import { world, system, BlockVolume } from "@minecraft/server";
import { CANDIDATES, CELLS } from "./data.js";

const log = (text) => console.warn("PROBE " + text);
const ticks = (n) => new Promise((resolve) => system.runTimeout(resolve, n));

function loaded(dim, x, z) {
  try {
    const b = dim.getBlock({ x, y: -64, z });
    return b !== undefined && b.isValid;
  } catch {
    return false;
  }
}

function find(dim, type, from, to) {
  const found = [];
  const volume = dim.getBlocks(new BlockVolume(from, to), { includeTypes: [type] }, true);
  for (const p of volume.getBlockLocationIterator()) found.push(p);
  return found;
}

async function probe(dim, [bx, bz], n) {
  const cx = Math.floor(bx / 16), cz = Math.floor(bz / 16);
  const name = `p${n}`;
  dim.runCommand(`tickingarea add ${(cx - 3) * 16} 0 ${(cz - 3) * 16} ${(cx + 3) * 16 + 15} 0 ${(cz + 3) * 16 + 15} ${name}`);
  for (let i = 0; i < 400; i++) {
    let all = true;
    for (let x = cx - 3; x <= cx + 3 && all; x++) for (let z = cz - 3; z <= cz + 3 && all; z++) all = loaded(dim, x * 16, z * 16);
    if (all) break;
    await ticks(5);
  }
  await ticks(40);
  try {
    const from = { x: (cx - 3) * 16, y: -40, z: (cz - 3) * 16 }, to = { x: (cx + 3) * 16 + 15, y: 300, z: (cz + 3) * 16 + 15 };
    const s = find(dim, "minecraft:sponge", from, to), h = find(dim, "minecraft:honeycomb_block", from, to), k = find(dim, "minecraft:dried_kelp_block", from, to);
    log(`temple ${bx} ${bz} markers ${JSON.stringify([s, h, k])}`);
    if (s.length !== 1 || h.length !== 1 || k.length !== 1) return;
    const [ws, wh, wk] = [s[0], h[0], k[0]];
    const ex = { x: (wh.x - ws.x) / 2, z: (wh.z - ws.z) / 2 }, ez = { x: (wk.x - ws.x) / 2, z: (wk.z - ws.z) / 2 };
    log(`frame ${bx} ${bz} ex ${ex.x},${ex.z} ez ${ez.x},${ez.z}`);
    for (const [i, x, y, z] of CELLS) {
      const at = { x: ws.x + (x - 1) * ex.x + (z - 1) * ez.x, y: ws.y + (y - 1), z: ws.z + (x - 1) * ex.z + (z - 1) * ez.z };
      const b = dim.getBlock(at);
      log(`cell ${ex.x},${ex.z},${ez.x},${ez.z} ${i} ${b?.typeId} ${JSON.stringify(b?.permutation.getAllStates())} ${b?.isWaterlogged}`);
    }
  } finally {
    dim.runCommand(`tickingarea remove ${name}`);
  }
}

async function main() {
  const dim = world.getDimension("overworld");
  let n = 0;
  for (const c of CANDIDATES) await probe(dim, c, n++);
  log("done");
}

system.run(() => main().catch((e) => log(`error ${e} ${e.stack}`)));
