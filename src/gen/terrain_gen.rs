use crate::gen::earth_gen::Earth;
use crate::world::pos2d::Pos2d;
use crate::world::LoadOrders;
use crate::world::VoxelWorld;
use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Resource, Default)]
pub struct TerrainGenerationQueue {
    queue: Vec<(Pos2d<62>, u32)>, // col_pos and priority/distance
    in_progress: Option<Pos2d<62>>,
    generator: Option<Earth>,
}

pub fn setup_gen_system(mut commands: Commands) {
    // Initialize the terrain generator
    let generator = Earth::new(HashMap::new());
    commands.insert_resource(TerrainGenerationQueue {
        queue: Vec::new(),
        in_progress: None,
        generator: Some(generator),
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
    // Process current terrain chunk if there is one
    if let Some(col_pos) = terrain_queue.in_progress.take() {
        if let Some(generator) = &terrain_queue.generator {
            // Generate the terrain for this column
            generator.gen(&world, col_pos);

            // Mark the column as changed so it will be meshed
            // world.mark_change_col(col_pos);
        }
    }

    // Get next chunk to process
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
