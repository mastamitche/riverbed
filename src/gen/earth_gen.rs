use crate::world::{
    BlockPos, BlockPos2d, ChunkPos, ColPos, VoxelWorld, CHUNK_S1, CHUNK_S1I, MAX_GEN_HEIGHT,
    WATER_H,
};
use crate::{gen::Soils, Block};
use bevy::prelude::info_span;
use riverbed_closest::{ranges, ClosestTrait};
use std::{collections::HashMap, ops::RangeInclusive};

pub const CONT_R: f32 = (WATER_H + 2) as f32 / MAX_GEN_HEIGHT as f32;
pub const CONT_COMPL: f32 = 1. - CONT_R;

pub struct Earth {
    config: HashMap<String, f32>,
}

fn pos_to_range(pos: ColPos) -> [RangeInclusive<i32>; 2] {
    let x = pos.z * CHUNK_S1I;
    let y = pos.x * CHUNK_S1I;
    [x..=(x + CHUNK_S1I - 1), y..=(y + CHUNK_S1I - 1)]
}

impl Earth {
    pub fn new(config: HashMap<String, f32>) -> Self {
        Earth { config }
    }
    pub fn gen(&self, world: &VoxelWorld, col: ColPos) {
        let fill_span = info_span!("chunk filling", name = "chunk filling").entered();

        // Constants for hill generation
        const BASE_HEIGHT: i32 = 40;
        const HILL_HEIGHT: i32 = 20;
        const HILL_SCALE_X: f32 = 0.05;
        const HILL_SCALE_Z: f32 = 0.05;

        // Fill each column with dirt and top with grass
        for x in 0..CHUNK_S1 {
            for z in 0..CHUNK_S1 {
                // Calculate absolute block positions
                let abs_x = col.x * CHUNK_S1I + x as i32;
                let abs_z = col.z * CHUNK_S1I + z as i32;

                // Generate rolling hills using sine waves
                let x_factor = (abs_x as f32 * HILL_SCALE_X).sin();
                let z_factor = (abs_z as f32 * HILL_SCALE_Z).sin();
                let diagonal_factor = ((abs_x as f32 + abs_z as f32) * HILL_SCALE_X * 0.7).sin();

                // Combine waves for more natural looking hills
                let height_factor = (x_factor + z_factor + diagonal_factor) / 3.0;

                // Calculate final height (base + hills)
                let top_height = BASE_HEIGHT + (height_factor * HILL_HEIGHT as f32) as i32;

                // Fill with dirt from y=0 to top_height-1
                for y in 0..top_height {
                    let pos = BlockPos {
                        x: abs_x,
                        y,
                        z: abs_z,
                        realm: col.realm,
                    };
                    world.set_block(pos, Block::AcaciaLeaves);
                }

                // Add grass at the top
                let grass_pos = BlockPos {
                    x: abs_x,
                    y: top_height,
                    z: abs_z,
                    realm: col.realm,
                };
                world.set_block(grass_pos, Block::AcaciaLeaves);
            }
        }

        // Mark chunks as loaded - now need to go higher for the hills
        let max_chunk_height = (BASE_HEIGHT + HILL_HEIGHT + 10) / CHUNK_S1 as i32;
        for y in 0..max_chunk_height {
            let chunk_pos = ChunkPos {
                x: col.x,
                y,
                z: col.z,
                realm: col.realm,
            };
            world.set_loaded(chunk_pos);
        }

        fill_span.exit();
    }
}
