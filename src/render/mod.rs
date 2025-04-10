pub mod camera;
mod chunk_culling;
mod draw_chunks;
pub mod hbao;
mod mesh_chunks;
mod mesh_utils;
mod shared_load_area;
pub mod sky;
pub mod texture_array;
use bevy::prelude::Plugin;

pub struct Render;

impl Plugin for Render {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_plugins(draw_chunks::Draw3d)
            .add_plugins(sky::SkyPlugin)
            .add_plugins(camera::Camera3dPlugin)
            .add_plugins(hbao::EffectsPlugin);
    }
}
