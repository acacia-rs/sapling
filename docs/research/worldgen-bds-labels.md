# BDS worldgen labels by milestone

Addresses named while porting each stage (bdsre labels; main write-up in [worldgen-bds.md](worldgen-bds.md)).

Terrain labels: 14822d4b0 `_computeDensityCells`, 14822e160 `getPreliminarySurfaceLevel`, 145e24e60
`MultiOctaveNoise::MultiOctaveNoise`, 145e26950 `ParityImprovedNoise::noise`, 145e24940/145e29ee0/145e23fc0
`LegacyBlendedNoise::{LegacyBlendedNoise,addOctave,fillOctave}`, 1481e10d0 `Cavifier::carve`, 1481dfe30/1481e0af0
`Aquifer::{computeSubstance,computeFluid}`, 1481def00/1481df0e0/1481df660 `OverworldNoises3d::{sampleQuart,
sampleClimate,terrainInfo}`, 14822cb50 `ChunkLocalNoiseCache::ChunkLocalNoiseCache`, 148078730/148096a00
`CubicSpline::{apply,applyMultipoint}`, 14805f140 `BiomeSource3d::fillBiomes`, 14805eb90 `fillLattice`, 148a73c40/
148a72d90/142f67a10/148a72910 `BiomeVolume::{fuzzyZoom,latticeJitter,nearestCorner,uniformCellMask}`, 14805fad0/
14805fb30/1480616f0/14805fc20 `Climate::{quantizeCoord,RTreeNode::distance,RTree::search,TargetPoint}`.

Feature labels (2026-10-07): 144c03360/144bffc40 `FeatureRulesLoader::{loadAndAttach,parseRule}`, 14027dd40
`HashedString::HashedString`, 14478ea70/14478fbe0/14478fe20/14478f9f0/1447a4400 `ScatterParams::{initScatter,
getPosConst,getPosMolang,sampleDistribution,evalCoordinate}`, 143b8dcb0/143b7fdc0 `OreFeature::{place,isExposedTo}`,
149d60440 `SandFeature::place`, 148287a70 `ClayFeature::place`, 141602f90/1415f7a50 `BlockSource::{getBiome,
getAboveTopSolidBlock}`, 140a08bb0 `LevelChunk::getAboveTopSolidBlock`.

Milestone 2a labels: 148230a70 `OverworldBiomeBuilder::addBiomes`, 1481f9850/1482009c0/148206880/14820bae0
`::{addMidSlice,addHighSlice,addPeaks,addLowSlice}`, 1481f95f0 `::addSurfaceBiome`, 14805fa80 `Climate::Parameter::span`,
14805fd00 `Climate::parameters`, 1480610f0/148060510/148060320/14805ff10 `RTree::{create,build,Node::sort,
Node::makeSubTree}`, 14808bb60 `RTree::Node::sort::less`, 148089fd0/14808ced0 `std::sort<RTree::Node>` (by params,
by key), 145809ef0/145809370/1487dcb80/1487e4350/1487e4d10 `VanillaSurfaceBuilders::{Overworld,Capped,Mesa,
FrozenOcean,Swamp}SurfaceBuilder::buildSurfaceAt`, 1487dd550/1487e18e0/1487e4870 `::{Mesa,FrozenOcean,Swamp}
SurfaceBuilder::init`, 143907ca0 `SurfaceMaterialAdjustments::evaluate`, 1481f0380 `OreVeinifier::OreVeinifier`,
145e2a730 `PerlinSimplexNoise::addOctave`, 145e27a70/145e27d50 `SimplexNoise::{getValue2D,getValue3D}`,
141188be0/141188db0/141188e50/141188bc0 `Random::{_genRandInt32,nextFloat,nextInt,nextBoolean}`.

Residue labels (2026-10-07): 1409ddfc0/1409e3920/1409e09d0/140a0cbd0 `ChunkSource::{_spawnChunkGenerationTasks,
queue (pending vector +0x70), _checkForUnblockingChunks, std::sort of tasks}`, 140a16cd0 decoration task lambda,
1438df8f0 `PostprocessingManager::tryLock`, 142fe8f60 `TickingAreaView::tick` (influence), 149e845f0/149e867f0
`GeodeFeature::{place,scan row}`, 149e96790/149e8f460/149e90f80/14827ec60 `{LegacyIceFeature,IcebergFeature,
IcebergFeature::carve,BlueIceFeature}::place`, 1411a3e60 `LevelChunk` temperature-noise cache, 140a09950
`LevelChunk::getTopRainBlockPos`, 1408741f0 `IRandom::nextDouble` (MT: raw x 2^-32), 14a7125a0 Random's IRandom vtable.

Liquid labels (2026-10-07): 149e96f20/149e97230 `SpringsFeature::{SpringsFeature,place}` (vtable 14a98e800),
149d65900/149d65e10 `SpringFeature::{place,isRock}` (vtable 14a98ad40), 141603c30/1416042f0/141607780
`BlockTickingQueue::{add,tickPendingTicks,acquireAllTicks}`, 141603fd0/1415f50f0 `{BlockTickingQueue,BlockSource}::
addToTickingQueue`, 1415f6ae0/1415f6fd0 `BlockSource::{setBlock impl,_blockChanged}` (vtable 14a753900, setBlock
slot +0x100), 1415f89a0/1415f87d0 `BlockSource::{updateNeighborsAt,updateNeighbor}`, 1415f46f0
`BlockSource::isInstaticking`, 148263880 loadChunk lambda_11 (fluid ticks), 14393d890 `VanillaBiomeTags` init
(registry tag ids 0x18 apart from ocean at +0x98: frozen +0x128, swamp +0x158, mutated +0x1d0, ice_plains +0x218).

Residue labels (2026-10-08): 148299570 `GrassDiscFeature::place`; block ids 0x14cce09b8 mud, 0x14cce0838 dirt,
0x14cce1af8 tuff, 0x14cce38b0 stone (unnamed in the initialiser, placed by alphabetical runs), 0x14ccf9950 air.

Structure feature labels (2026-10-08, structfeat): 145e137d0 `FeaturePoolElement` ctor (vtable 14a89acf0),
145e141d0/145e144c0 its legacy/v2 place, 145e1aa90 `StructurePoolElement::getBoundingBox`, 14827e1e0/14827ea50
pile feature place/block, 141f79610 `TemplateStructurePiece` box, 148358d80 `PillagerOutpostPiece::postProcess`
(details in worldgen-bds-jigsaw.md and worldgen-bds-templates.md).

Mineshaft labels (2026-10-08, struct3): 141efb070 `StructureFeature::foreachIntersectingStructureStart`, 141ef97c0
`StructureFeature` ctor, 1415f5900 `BlockSource::getChunk`; block globals 0x14ccdafb8 flowing_lava, 0x14ccdfc68
iron_chain, 0x14ccd9718 bedrock (details in worldgen-bds-structures.md, Mineshafts).

Feature residue labels (2026-10-08, feat2): 141882f10 `Dimension::getDefaultBiomeId` (vtables 14a75bd80,
14a75c020, slot 0xc8; nether 14880e9e0 = 8, end 148276900 = 9), 141602f90 `BlockSource::getBiome`, 1438eac70
`TreeHelper::prepareSpawn`, 1438f2ba0 `MegaTreeTrunk::placeTrunk`, 143bd34c0 `AcaciaTreeTrunk::placeTrunk`,
142cf3f80 `BlockDescriptor::matches` (lazy resolve 142cf62c0), 143bc8310 `VegetationPatchFeature::place`,
14828af70/14828b9e0 `CoralFeature::{place,buildShape}`, 148288070 `CoralCrustFeature::place`, 148294070
`CoralHangFeature::place`, 149d60d10 `SeaAnemoneFeature::place`, 149d61020 `SeaPickleFeature::place`; block
globals 0x14ccd5b70 tube_coral_wall_fan, 0x14ccd09a0 brain_coral_wall_fan.

Coral labels (2026-10-08, coral; LeviLamina names): 14828b9e0 `CoralFeature::_placeCoral` (not buildShape),
1482914f0 `_starFormation`, 148291d20 `_buildPlantArm`, 148290ad0 `_getColor`, 148290a30 `_canOverwrite`, 148289370
`CoralCrustFeature::_getCropOffsetFromRot`, 1482895b0 `_placeCoralBase`, 1416027c0 `BlockSource::isUnderWater`
(BlockSource vtable 14a753900, slot 0x50), 1415f69e0 `BlockSource::getLiquidBlock` (0x28), 144c0cc40
`Feature::_setBlock`, 149d99a70 `VinesFeature::place`, 143b7ff60 `TreeFeature::place`, 149e97fc0 the
`LegacyTreeFeature` tree loop; block lists 0x14ccda7e8 (10 coral blocks), 0x14ccd9410/9440/9458/3250 (live coral
blocks, plants, fans, wall fans); 0x14cce2b78 blue ice. IBlockWorldGenAPI slots: 0x20 getBlock, 0x58 setBlock, 0x78
mayPlace, 0x88 canSurvive, 0x98/0xa0 max/min height, 0xb0 getHeightmap, 0xb8 isLegacyLevel.

Liq2 labels (2026-10-09, read in a scratch bdsre, not saved as labels): 1409f59e0 `LevelChunk::setBlock`, 1409f3d60
its heightmap upkeep (returns unless state +0xf0 == 11), 1409f3f50 the column update, 1409e29f0 heightmap rebuild
(+0xb80 light, +0xd80, +0xf88 highest block), 140a16f40 decoration task lambda (`tryLock`, vtable +0x58, rebuild),
140a16cd0 structure task lambda (vtable +0x50), 1415f5aa0 `BlockSource::shouldFireEvents` (vtable +0x98), 141a28450
`BlockType::isLavaBlocking` (vtable +0xc8; `mThickness` +0x158, material +3 blocks motion, +5 solid), 148b5ee50
`LiquidBlock::_isLiquidBlocking`, 148b5eab0 lava fizz, 141f59f90 `StructurePiece::placeBlock` (liquid tick behind
`mAllowTickingChanges`, BlockSource +0x58), 14160c130 template placement, 142cedf20/142ce9ca0
`Block::neighborChanged` and its cannot-survive test, 1415483f0/141548760 a write list's commit (`onPlace`, ticks,
`updateNeighborsAt`), 144c0cc40 `Feature::_setBlock` (transaction at +0x18, else `setBlock` flags 3); block globals
0x14ccdaf58 flowing_water, 0x14ccdaf88 water; BlockSource +0x60 `mPendingConnectionUpdates`, +0xd8/+0xe0 random
and tick queues.

Feat4 labels (2026-10-09): 14478e670 `ScatterParams::initMolangParams` (callers: decorateBiome 14580b462, scatter
feature 143b9b76d, 1438edd8b), 0x14cd56144/46/48 origin x/y/z variable ids and 0x14cd5614a.. world ids,
1411a24c0 `query.noise` lambda (call 1411a25f1), 145e276f0 `PerlinSimplexNoise` ctor, 149d9a560 mushroom shade,
142cedc30 `Block::mayPlace(region, pos, face)`, 1438ebaf0 fallen trunk, `Block` +0xa5 `mLight`, `BlockType` +0x15c
translucency, `LevelChunk` +0xf88 highest block.

Sculk-round labels (2026-10-09, read in a scratch bdsre): 148213acc/1482144e3 the structure and decoration
`tryLock` call sites, 140a16cd0 structure task lambda (requeues when vt+0x50 returns false), 148b4265b the
sculk conversion in the vein behaviour's `attemptUseCharge` (148b42070; replaceable set at spreader +0x30, a
`std::set<const Block*>`), 1411a09d0 world-gen API slot 0x90 (`BlockType` vt+0x298, can be built over),
143b84a00.. `MultifaceFeature` attach's test of the supporting block's chunk (in another chunk of state 2..5,
a block inside a structure of the type at 0x14ccbb900, which `StrongholdFeature`'s constructor also uses, or
0x14ccbb930 refuses the face; not ported, decoration's neighbours are past that state), 145e2d090/145e2d290/145e2d4d0 pool alias
binding resolves, 141eeb8d0 `placeInWorld` (settings: +0 integrity, +4 seed, +8 terrain match, +9 mirror, +10
rotation, +11/+12 skip structure blocks and jigsaws, +0x10 ignored block), 141f797b0
`TemplateStructurePiece::postProcess` (seed = the chunk generator's seed field).
Drip labels (2026-10-09, read in scratch bdsre-drip, not saved as labels): 149d9ab60/149d9ac10 the cluster column's
inside/edge lambdas (`_placeColumn` lambda_12/13), 143beaf00/143beafd0/143beb050 `_findSnapPosInDirection`'s
support/open/solid lambdas (85/86/87), 143900260 `scanColumnWithinPosBiome`'s lambda, 141a24e20
`BlockType::canProvideSupport` (vtable 0x90; leaves 140076010), 148a897e0/148a89b20 the support component's lookup
and its stair/slab test, 1415f65e0/1415f6880/1415f69e0 `BlockSource::{getBlock,getExtraBlock,getLiquidBlock}`
(vtable 0x10/0x20/0x28; world-gen API 0x20/0x30), 1409f7770 `LevelChunk::setPreWorldGenHeightMap` (+0xf80; callers
14821178e loadChunk, 148215a83 decorationPostProcessChunk), 14823028b the store into it in `_prepareHeights`,
149d99710 moss vegetation place, 148299bc0 dripleaf place, 149d9a0f0 single-face vines place, 149d660d0 root
system place (predicate 1438f1bb0, ctor 149d65f70), 1425e2b70 vine block by face, 1465f64a0 pale moss carpet
placement, 144c15200 `GrowingPlantFeature::place`, 143bc8310 `VegetationPatchFeature::place` (accepted cell at
143bc8468); block globals 0x14ccdcc50 pale_moss_carpet, 0x14ccdfb78 invisible_bedrock, 0x14cce2ec0 vine, and by
their place in the id runs (unnamed in the initialiser) 0x14cce0778 clay, 0x14ccdb108 sand, 0x14ccda6b8 fern; lists
0x14ccd2738 (27: leaves and replaceable plants), 0x14ccd2758 (17: root-system replaceable), 0x14ccd94d0 (spikes);
`Material` +0 type (5 water, 6 lava), +3 blocks motion, +5 solid; `LevelChunk` +0xf0 state. The pre-worldgen
heightmap's setter name and `LevelChunk` +0x64 (subtracted from its values: taken as the chunk's minimum height)
are guesses.

Feat5 labels (2026-10-09, read in a scratch bdsre): 149e821d0 `FireflyBushClusterFeature::place` (vtable
14a98e600, +0x20 the water variant; registered at 14830431e and 14830447c), `BlockType` vt+0x210 `mayPlace(region,
pos)`, 14822d4b0 `_computeDensityCells`' arguments (rdx the 5x5x41 output, r8 the chunk, r9 the climate cache
{x0, z0, width, +0x10 entries of 0x28 bytes}; octave objects at this+0x280, +0x2a8, +0x2d0, 0x818 bytes each with
the offsets first) and stack buffers (14822d84a: rsp+0x213c, +0x1138, +0x134), 145e29ee0
`LegacyBlendedNoise::addOctave` (offsets from the random's vt+0x28 `nextFloat`, for Xoroshiro the rounded
`nextDouble`), 1481e10d0 `Cavifier::carve` (`imp:powf` at 1481e171e; 14a922758 the 3D spaghetti's thickness
span 0x3cbc6a80), 1487e4350 the frozen ocean builder (1487e44cb `Biome::getTemperature` call, 1487e4536 the
64-bit position hash, 1487e470c/1487e4713 the truncated bounds), 14580ae5c the overworld builder's sea-level
test (material 5, or block id 0x14cce2ba8 packed ice or 0x14cce2b78 blue ice), 141f69b60 and 144c50eb0 the
other two 64-bit `Mth.getSeed` copies (a structure processor's Java LCG and the block render offset).

Misc labels (2026-10-10, read in a scratch bdsre): 149e82e00 `FossilFeature::place` (vtable 14a928230, slot 4;
ore descriptor +0x20, max empty corners +0xd0), 149ea0080 the fossil template names' initialiser (table
0x14ce6cc10, 0x20 apart), 0x14a98ef20 its rotation switch, 1438f4e80 `PoplarTreeTrunk::placeTrunk` (vtable slot
at 14a98e028; height 1438ec2c0, fields +0x10/+0x14 remaining height, +0x18/+0x1c branches, +0x20 trunk block,
+0xd0 decoration, +0x1b0.. log feature), 1438f43a0 `PoplarTreeCanopy::placeCanopy` (slot at 14a98b910; +8 leaf,
+0xb8 branch block, +0x168 radius pairs, +0x180/+0x184 height, +0x188 side hole chance), 1438f4c10 its layer,
1438f0700 the trunks' attachable decoration, 149e8ca80 `HorizontalTreeDecorationFeature::place` (vtable
14a9283c0; block +0x10, bark_side_only +0xc0, allow_adjacent +0xc1), 0x14a6821c8 facing -> cardinal direction
value (north 2, south 0, west 1, east 3), 0x14a712760 facing offsets, 145e16f50 the v2 jigsaw attach test,
145e13b90 `FeaturePoolElement`'s jigsaw info, `StructurePoolElement` vt+0x68 (+0x138, ends a pool's list when
0) and vt+0x78 (1 for a feature element), 144d13b6e the 2x2x2 box a feature element leaves in its space.
The tree classes' vtables sit between std::function vtables in 14a98b000..14a98f000, three slots each
(destructor, place, height); the functions follow the classes' alphabetical order from 1438ea120 on.
Struct4 labels (2026-10-10, read in scratch bdsre-struct4, not saved as labels): 141a24f10/141a24e50
`BlockType::canProvideMultifaceSupport` and its non-virtual body, 146603600 slab / 1425764f0 trapdoor / 148b220d0
chain `canProvideSupport` (the rest in worldgen-bds-caves.md, "Support"), 148b6adf0 `MultifaceBlock` remove face,
148b41420 sculk vein replaceable (water source, or buildable over and not lava), 148b4a9d0 vein config
`canSpreadInto`, 1465e9f50 the lichen config's, 14196ac20/141efd3b0 `WorldGenerator::isStructureFeatureTypeAt` and
the registry's test (world generator vtable +0x150; +0x148 = 14196ac00 `findStructureFeatureTypeAt`), 141f085e0 the
structure type names' initialiser (0x14ccbb900 stronghold, 0x14ccbb930 mineshaft), 1419be4c0/1419beaf0 block
registry lookups (registry 0x14ccb1608), `BlockType` vtable 0x210 `mayPlace`, 0x220 `mayPlaceOn`, 0x298 buildable
over; `setBlock` call sites of the sculk code: 143b84ccc multiface attach, 1465e946d.. (6) spread toward random,
1465e9bc7.. (6) spread toward all, 148b41132 vein regrow, 148b42676 conversion, 148b41ae2/148b41cee growth,
148b6b1f7 remove face, 143ba17e8 catalyst, 143ba1853 shrieker; 0x14ccd3328 the dirt list, 0x14ce19564 the support
component's id.

Sculk2 labels (2026-10-10, read in scratch bdsre-sculk2, not saved as labels): 1411889d0 the thread-local MT's
constructor (seed: four bytes from 141cbf950/141ccbe30, the system entropy source), 1465e98c0 the in-game random
multiface spread (the other caller of 1465e9360 and of the shuffle 1403747f0), 142fce030 grindstone / 146602890
skull / 148a32230 end rod / 148b5bff0 lightning rod / 148b51e20 shelf `canProvideSupport` (this, block, face,
type); block state ids 0x14cd254f0, 0x14cd26368 and 0x14cd25130, taken as `attachment`, `facing_direction` and
`minecraft:cardinal_direction` from which blocks read them (names not read out of the binary).

Feat3 labels (2026-10-08): 14a73fa30 the decoration's IBlockWorldGenAPI vtable (forwards to its `BlockSource` at
+8; 0xb0 = 1411a0a60 heightmap: `LevelChunk` +0xb80 i16 per column + minimum y), 1438f0d00
`MangroveTreeRoots::placeRoots` (vtable 14a98e1f8), 1438f16f0 its root simulation, 14a681437 its facing table,
1438f24d0 `MangroveTreeTrunk::getTreeHeight`, 1438f4080 `MegaTreeTrunk::getTreeHeight`, 1438ed7d0 block
normaliser before `BlockDescriptor::matches`, 143bcfb10 `WeightedRandomFeature::place`, 1438ef7d0
`MangroveTreeCanopy::placeCanopy`; IRandom vtable 14a7125a0 slots 0x8 nextInt(n), 0x10 nextIntUnbounded, 0x18
two raw draws as a 64-bit value, 0x20 nextBoolean (bit 27), 0x28 nextFloat.

Lush2 labels (2026-10-10, read in scratch bdsre-lush2, not saved): 1411889d0 `Random::Random()` (seed from four
platform random bytes via 141ccbe30; 141188a43 stores it, ecx), 143b845e6 the multiface feature's first-use
initialiser of the thread-local generator (`gs:[0x58]` +0x460 the `Random`, +0xe60 its wrapper, guards +0xe48 and
+0x1860), 143bc8468 the vegetation patch's accepted cell, 143bc8be0 its vegetation loop, 143bc871f the depth-0
shortcut; `VegetationPatchFeature` fields +0xf4/+0xf8 depth, +0xfc extra deep chance, +0x100 vertical range,
+0x104 vegetation chance, +0x108/+0x10c radius, +0x110 extra edge chance, +0x114 waterlogged.
