//! Blocks BDS saves with a second layer: the water of a waterlogged block, or the small plant a
//! snow layer fell on (`snow_layer` with the plant in the layer liquids use; oracle: short grass,
//! ferns, poppies and dandelions under the freeze pass's snow, never a double plant or a mushroom).

use super::Block;

/// A plant and the snow layer holding it, in table order of the snow layers.
const SNOWED: [(Block, Block); 14] = [
    (Block::ShortGrass, Block::SnowOverShortGrass),
    (Block::Fern, Block::SnowOverFern),
    (Block::Poppy, Block::SnowOverPoppy),
    (Block::Dandelion, Block::SnowOverDandelion),
    (Block::BlueOrchid, Block::SnowOverBlueOrchid),
    (Block::AzureBluet, Block::SnowOverAzureBluet),
    (Block::RedTulip, Block::SnowOverRedTulip),
    (Block::OrangeTulip, Block::SnowOverOrangeTulip),
    (Block::WhiteTulip, Block::SnowOverWhiteTulip),
    (Block::PinkTulip, Block::SnowOverPinkTulip),
    (Block::OxeyeDaisy, Block::SnowOverOxeyeDaisy),
    (Block::Cornflower, Block::SnowOverCornflower),
    (Block::LilyOfTheValley, Block::SnowOverLilyOfTheValley),
    (Block::Allium, Block::SnowOverAllium),
];

impl Block {
    /// A snow layer, bare or over a plant.
    pub fn is_snow_layer(self) -> bool {
        (Block::SnowLayer..=Block::SnowOverAllium).contains(&self)
    }

    /// The snow layer that keeps this plant in its second layer, for the plants snow covers.
    pub fn snowed(self) -> Option<Block> {
        SNOWED.iter().find(|(plant, _)| *plant == self).map(|&(_, snow)| snow)
    }

    /// What BDS saves in the cell's second layer: water, or the plant under a snow layer.
    pub fn second_layer(self) -> Option<Block> {
        if self.is_waterlogged() {
            return Some(Block::Water);
        }
        SNOWED.iter().find(|(_, snow)| *snow == self).map(|&(plant, _)| plant)
    }
}
