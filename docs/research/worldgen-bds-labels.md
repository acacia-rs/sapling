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

Feat3 labels (2026-10-08): 14a73fa30 the decoration's IBlockWorldGenAPI vtable (forwards to its `BlockSource` at
+8; 0xb0 = 1411a0a60 heightmap: `LevelChunk` +0xb80 i16 per column + minimum y), 1438f0d00
`MangroveTreeRoots::placeRoots` (vtable 14a98e1f8), 1438f16f0 its root simulation, 14a681437 its facing table,
1438f24d0 `MangroveTreeTrunk::getTreeHeight`, 1438f4080 `MegaTreeTrunk::getTreeHeight`, 1438ed7d0 block
normaliser before `BlockDescriptor::matches`, 143bcfb10 `WeightedRandomFeature::place`, 1438ef7d0
`MangroveTreeCanopy::placeCanopy`; IRandom vtable 14a7125a0 slots 0x8 nextInt(n), 0x10 nextIntUnbounded, 0x18
two raw draws as a 64-bit value, 0x20 nextBoolean (bit 27), 0x28 nextFloat.
