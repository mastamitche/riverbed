use crate::world::{unchunked, ColPos, VoxelWorld, CHUNK_S1, MAX_GEN_HEIGHT};
use crate::{gen::Soils, Block};
use itertools::iproduct;
use riverbed_closest::{points, ranges, ClosestTrait};
use std::{collections::HashMap, path::Path};

pub struct DebugGen {
    seed: u32,
    config: HashMap<String, f32>,
    soils: Soils,
}

impl Clone for DebugGen {
    fn clone(&self) -> Self {
        DebugGen::new(self.seed, self.config.clone())
    }
}
impl DebugGen {
    pub fn new(seed: u32, config: std::collections::HashMap<String, f32>) -> Self
    where
        Self: Sized + Clone,
    {
        DebugGen {
            seed,
            config,
            soils: ranges::from_csv("assets/gen/soils_condition.csv").unwrap(),
        }
    }

    pub fn gen(&self, world: &VoxelWorld, col: ColPos) {
        for (dx, dz) in iproduct!(0..CHUNK_S1, 0..CHUNK_S1) {
            let (x, z) = (unchunked(col.x, dx), unchunked(col.z, dz));
            let (y, t, h) = (0.5, 0.5, 0.5);
            let y = (y * MAX_GEN_HEIGHT as f32) as i32;
            assert!(y >= 0);
            let block = *self.soils.closest([t as f32, h as f32]).0;
            world.set_yrange(col, (dx, dz), y, 3, block);
        }
        // this is a bit too slow so we don't bother with it for now
        // col.fill_up(Block::Stone);
    }
}
