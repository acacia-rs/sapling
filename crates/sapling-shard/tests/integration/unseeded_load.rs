//! A load at integrity 0.5 with no seed draws its cells anew: BDS's two such loads in one tick
//! differed (research/captures/bds-q3integ2.txt).

use crate::common;

use common::Harness;
use sapling_api::{Block, ShardCtx, StructureLoad};

fn placed(ctx: &ShardCtx<'_>, z0: i32) -> Vec<bool> {
    (0..4).flat_map(|x| (64..67).flat_map(move |y| (0..4).map(move |z| [x, y, z + z0]))).map(|at| ctx.block(at).is_some_and(|block| !block.is_air())).collect()
}

#[test]
fn two_unseeded_half_loads_in_one_tick_place_different_cells() {
    let mut h = Harness::new();
    h.enter(1, "Steve");
    h.remote.run(|ctx| {
        for at in (0..4).flat_map(|x| (64..67).flat_map(move |y| (0..4).map(move |z| [x, y, z]))) {
            assert!(ctx.set_block(at, &Block::new("oak_planks")));
        }
        assert_eq!(ctx.save_structure("q3s", [0, 64, 0], [3, 66, 3], false, true, true), Ok(()));
        let half = StructureLoad { integrity: 0.5, ..StructureLoad::default() };
        assert_eq!((ctx.load_structure("q3s", [0, 64, 8], &half), ctx.load_structure("q3s", [0, 64, 16], &half)), (Ok(()), Ok(())));
        let (first, second) = (placed(ctx, 8), placed(ctx, 16));
        assert!(first.iter().any(|placed| *placed) && first.iter().any(|placed| !*placed), "{first:?}");
        assert_ne!(first, second, "each unseeded load draws its own cells");
    });
    h.tick_all();
}
