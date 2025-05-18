use crate::{agents::PlayerSpawn, utils::INITIAL_FOV};
use bevy::{
    core_pipeline::experimental::taa::TemporalAntiAliasing, pbr::ScreenSpaceAmbientOcclusion,
    prelude::*,
};

pub struct Camera3dPlugin;

impl Plugin for Camera3dPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Startup,
            (cam_setup, apply_deferred)
                .chain()
                .in_set(CameraSpawn)
                .after(PlayerSpawn),
        );
    }
}
#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash)]
pub struct CameraSpawn;

#[derive(Component)]
pub struct MainCamera;

pub fn cam_setup(mut commands: Commands) {
    commands
        .spawn((
            Camera {
                hdr: true,
                ..default()
            },
            Camera3d::default(),
            Transform::from_xyz(0., 0., 0.),
            Projection::Perspective(PerspectiveProjection {
                fov: INITIAL_FOV,
                ..Default::default()
            }),
            Msaa::Off,
            ScreenSpaceAmbientOcclusion::default(),
            TemporalAntiAliasing::default(),
            MainCamera
        ))
        //b
        ;
}
