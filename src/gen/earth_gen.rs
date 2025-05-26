use crate::r#gen::terrain_gen::GenerationPhase;
use crate::world::{
    BlockPos, BlockPos2d, ChunkPos, ColPos, VoxelWorld, CHUNK_S1, CHUNK_S1I, MAX_GEN_HEIGHT,
    WATER_H,
};
use crate::{gen::Soils, Block};
use bevy::prelude::info_span;
use riverbed_closest::{ranges, ClosestTrait};
use std::{collections::HashMap, ops::RangeInclusive};

use super::terrain_gen::GenerationState;

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
    pub fn process_generation_chunk(
        &self,
        state: &mut GenerationState,
        world: &VoxelWorld,
        max_time_ms: u32,
        start_time: std::time::Instant,
    ) -> bool {
        const HILL_SCALE_X: f32 = 0.00;
        const HILL_SCALE_Z: f32 = 0.00;

        loop {
            match state.phase {
                GenerationPhase::CalculatingHeights => {
                    // Process a batch of height calculations
                    while state.current_x < CHUNK_S1 {
                        while state.current_z < CHUNK_S1 {
                            // Calculate absolute block positions
                            let abs_x = state.col_pos.x * CHUNK_S1I + state.current_x as i32;
                            let abs_z = state.col_pos.z * CHUNK_S1I + state.current_z as i32;

                            // Generate rolling hills using sine waves
                            let x_factor = (abs_x as f32 * HILL_SCALE_X).sin();
                            let z_factor = (abs_z as f32 * HILL_SCALE_Z).sin();
                            let diagonal_factor =
                                ((abs_x as f32 + abs_z as f32) * HILL_SCALE_X * 0.7).sin();

                            // Combine waves for more natural looking hills
                            let height_factor = (x_factor + z_factor + diagonal_factor) / 3.0;

                            // Calculate final height (base + hills)
                            let top_height = state.base_height
                                + (height_factor * state.hill_height as f32) as i32;
                            state.top_height = Some(top_height);

                            // Fill with dirt from y=0 to top_height-1
                            for y in 0..top_height {
                                let pos = BlockPos {
                                    x: abs_x,
                                    y,
                                    z: abs_z,
                                };
                                world.set_block(pos, Block::AcaciaLeaves, false);
                            }

                            // Add grass at the top
                            let grass_pos = BlockPos {
                                x: abs_x,
                                y: top_height,
                                z: abs_z,
                            };
                            world.set_block(grass_pos, Block::AcaciaLeaves, false);

                            // Move to the next position
                            state.current_z += 1;

                            // Check if we've spent too much time
                            if start_time.elapsed().as_millis() > max_time_ms as u128 {
                                return false; // Not completed yet
                            }
                        }
                        state.current_z = 0;
                        state.current_x += 1;

                        // Check time again after completing a row
                        if start_time.elapsed().as_millis() > max_time_ms as u128 {
                            return false; // Not completed yet
                        }
                    }

                    // Move to marking chunks
                    state.phase = GenerationPhase::MarkingChunks;
                    state.current_x = 0; // Reuse for chunk y index

                    // Check if we should continue to the next phase in this frame
                    if start_time.elapsed().as_millis() > max_time_ms as u128 {
                        return false; // Not completed yet
                    }

                    // Continue to the next phase in this frame
                    continue;
                }

                GenerationPhase::MarkingChunks => {
                    let max_chunk_height =
                        (state.base_height + state.hill_height + 10) / CHUNK_S1 as i32;

                    while state.current_x < max_chunk_height as usize {
                        let chunk_pos = ChunkPos {
                            x: state.col_pos.x,
                            y: state.current_x as i32,
                            z: state.col_pos.z,
                        };

                        let loaded = world.set_loaded(chunk_pos);

                        state.current_x += 1;

                        // Check if we've spent too much time
                        if start_time.elapsed().as_millis() > max_time_ms as u128 {
                            return false; // Not completed yet
                        }
                    }

                    // All done!
                    state.phase = GenerationPhase::Completed;
                    return true;
                }

                GenerationPhase::Completed => {
                    return true; // Already completed
                }
            }
        }
    }
}
