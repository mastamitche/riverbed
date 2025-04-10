mod chunk;
mod load_area;
mod load_orders;
mod pos;
mod realm;
mod utils;
mod voxel_world;

use self::load_orders::{
    assign_load_area, on_render_distance_change, process_unload_orders, update_load_area,
};
use crate::{agents::PlayerSpawn, gen::setup_gen_thread};
use bevy::{
    app::Startup,
    ecs::schedule::{apply_deferred, IntoSystemConfigs, SystemSet},
    prelude::{Plugin, Update},
};
pub use chunk::*;
pub use load_area::{range_around, PlayerArea, RenderDistance};
pub use load_orders::{BlockEntities, ColUnloadEvent, LoadOrders};
pub use pos::*;
pub use realm::*;
pub use voxel_world::*;
pub const CHUNK_S1: usize = 62;
#[allow(dead_code)]
pub const CHUNK_S2: usize = CHUNK_S1.pow(2);
pub const CHUNKP_S1: usize = CHUNK_S1 + 2;
pub const CHUNKP_S2: usize = CHUNKP_S1.pow(2);
pub const CHUNKP_S3: usize = CHUNKP_S1.pow(3);
pub const CHUNK_S1I: i32 = CHUNK_S1 as i32;

pub const MAX_HEIGHT: usize = 496;
pub const MAX_GEN_HEIGHT: usize = 400;
pub const WATER_H: i32 = 61;
pub const Y_CHUNKS: usize = MAX_HEIGHT / CHUNK_S1;

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy, SystemSet)]
pub struct LoadAreaAssigned;

pub struct GenPlugin;

impl Plugin for GenPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.insert_resource(LoadOrders::new())
            .insert_resource(BlockEntities::default())
            .add_event::<ColUnloadEvent>()
            .add_systems(Startup, setup_gen_thread)
            .add_systems(
                Startup,
                (assign_load_area, apply_deferred)
                    .chain()
                    .in_set(LoadAreaAssigned)
                    .after(PlayerSpawn),
            )
            .add_systems(Update, update_load_area)
            .add_systems(Update, on_render_distance_change)
            .add_systems(Update, process_unload_orders);
    }
}
