use crate::gen::earth_gen::Earth;
use crate::world::pos2d::Pos2d;
use crate::world::ColPos;
use crate::world::LoadOrders;
use crate::world::VoxelWorld;
use bevy::prelude::*;
use std::collections::HashMap;

pub const MAX_GEN_TIME_MS: u32 = 20;

#[derive(Resource, Default)]
pub struct TerrainGenerationQueue {
    pub queue: Vec<(Pos2d<62>, u32)>, // col_pos and priority/distance
    pub in_progress: Option<Pos2d<62>>,
    pub generator: Option<Earth>,
    // Generation state
    pub gen_state: Option<GenerationState>,
}
#[derive(Default, Copy, Clone)]
pub struct GenerationState {
    pub col_pos: ColPos,
    pub current_x: usize,
    pub current_z: usize,
    pub top_height: Option<i32>, // Store the calculated height when processing blocks
    pub base_height: i32,
    pub hill_height: i32,
    pub phase: GenerationPhase,
}
#[derive(Default, Copy, Clone)]
pub enum GenerationPhase {
    #[default]
    CalculatingHeights,
    MarkingChunks,
    Completed,
}

pub fn setup_gen_system(mut commands: Commands) {
    // Initialize the terrain generator
    let generator = Earth::new(HashMap::new());
    commands.insert_resource(TerrainGenerationQueue {
        queue: Vec::new(),
        in_progress: None,
        generator: Some(generator),
        gen_state: None,
    });
}
pub fn queue_terrain_generation(
    mut terrain_queue: ResMut<TerrainGenerationQueue>,
    load_orders: Res<LoadOrders>,
) {
    // Only add to the queue if we're not currently processing a chunk
    if terrain_queue.in_progress.is_none() {
        if let Some(mut orders) = load_orders.to_generate.try_write_arc() {
            if let Some((col_pos, priority)) = orders.pop() {
                terrain_queue.queue.push((col_pos, priority));
            }
        }
    }
}
pub fn process_terrain_generation(
    mut terrain_queue: ResMut<TerrainGenerationQueue>,
    mut world: ResMut<VoxelWorld>,
) {
    let start_time = std::time::Instant::now();
    let col_pos_wrapped = terrain_queue.in_progress;
    let mut gen_state_wrapped = terrain_queue.gen_state;
    let gen_unwrapped = &terrain_queue.generator;
    if gen_unwrapped.is_none() {
        return;
    }
    if col_pos_wrapped.is_some() {
        let col_pos = col_pos_wrapped.unwrap();
        let gen = gen_unwrapped.as_ref().unwrap();
        // If we don't have a generation state, initialize one
        if gen_state_wrapped.is_none() {
            gen_state_wrapped = Some(GenerationState {
                col_pos,
                current_x: 0,
                current_z: 0,
                top_height: None,
                base_height: 40, // Constants from the original function
                hill_height: 20,
                phase: GenerationPhase::CalculatingHeights,
            });
        }

        let mut gen_state = gen_state_wrapped.unwrap();

        let completed =
            gen.process_generation_chunk(&mut gen_state, &world, MAX_GEN_TIME_MS, start_time);

        // If generation is complete, clean up
        terrain_queue.gen_state = Some(gen_state);
        if completed {
            terrain_queue.gen_state = None;
            terrain_queue.in_progress = None;
        }
    }
    // Get next chunk to process if we're not currently processing one
    if terrain_queue.in_progress.is_none() && !terrain_queue.queue.is_empty() {
        // Sort by priority if needed
        terrain_queue
            .queue
            .sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        // Take the highest priority chunk
        if let Some((pos, _)) = terrain_queue.queue.pop() {
            terrain_queue.in_progress = Some(pos);
        }
    }
}
