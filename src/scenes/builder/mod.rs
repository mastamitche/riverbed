use bevy::prelude::*;
use bevy_egui::EguiContextPass;
use builder_chunk::BuilderChunk;
use systems::*;

pub mod builder_chunk;
pub mod systems;
pub struct BuilderPlugin;
impl Plugin for BuilderPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app
            .insert_resource(BuilderSettings::default())
            .init_resource::<BuilderChunk>()
            .add_systems(Startup, create_area)
            .add_systems(
                EguiContextPass,
                (render_to_image_example_system),
            )
            .add_systems(Update, (adjust_camera_angle,update_chunk_border))
        //b
        ;
    }
}
