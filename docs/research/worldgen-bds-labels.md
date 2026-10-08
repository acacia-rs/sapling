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
