use bevy::{
    asset::RenderAssetUsages,
    core_pipeline::experimental::taa::TemporalAntiAliasing,
    pbr::ScreenSpaceAmbientOcclusion,
    prelude::*,
    render::{
        camera::RenderTarget,
        render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
    },
};
use bevy_egui::{egui, EguiContextPass, EguiContexts, EguiGlobalSettings, EguiUserTextures};
use builder_chunk::BuilderChunk;
use systems::*;

use crate::{
    interactions::place::{PlaceBlockEvent, PlaceDestination},
    render::{
        camera::Y_CAM_SPEED,
        draw_chunks::{BuildingPreview, BuildingState, WorldMesh},
    },
    setup::Block,
    ui::{CameraOrbit, CameraSettings, CameraSmoothing},
    utils::{lerp, INITIAL_FOV},
    world::pos3d::Pos3d,
};

pub mod builder_chunk;
pub mod systems;
pub struct BuilderPlugin;
impl Plugin for BuilderPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app
            .init_resource::<BuilderChunk>()
            .add_systems(Startup, create_area)
            .add_systems(
                EguiContextPass,
                (render_to_image_example_system),
            )
            .add_systems(PostUpdate, adjust_camera_angle)
        //b
        ;
    }
}
