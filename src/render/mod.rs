pub mod binary_greedy_meshing;
pub mod camera;
mod chunk_culling;
pub mod draw_chunks;
mod effects;
mod mesh_chunks;
mod mesh_utils;
mod shared_load_area;
pub mod sky;
pub mod texture_array;

mod texture_load;
use bevy::prelude::Plugin;
pub use texture_load::*;

pub struct Render;

impl Plugin for Render {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_plugins(draw_chunks::Draw3d)
            .add_plugins(sky::SkyPlugin)
            .add_plugins(camera::Camera3dPlugin)
            .add_plugins(effects::EffectsPlugin);
    }
}
