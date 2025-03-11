use bevy::{
    prelude::*,
};

pub struct TextureArrayPlugin;

impl Plugin for TextureArrayPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, build_base_mat);
    }
}


fn build_base_mat(mut commands: Commands, mut materials: ResMut<Assets<StandardMaterial>>) {
    let handle = materials.add(StandardMaterial {
        perceptual_roughness: 1.,
        reflectance: 0.1,
        alpha_mode: AlphaMode::AlphaToCoverage,
        ..Default::default()
    });
    commands.insert_resource(BlockTextureArray(handle));
}

#[derive(Resource)]
pub struct BlockTextureArray(pub Handle<StandardMaterial>);