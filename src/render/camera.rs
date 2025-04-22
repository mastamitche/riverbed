use crate::agents::PlayerSpawn;
use bevy::prelude::*;
use leafwing_input_manager::prelude::*;

pub struct Camera3dPlugin;

impl Plugin for Camera3dPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(InputManagerPlugin::<CameraMovement>::default())
            .add_systems(
                Startup,
                (cam_setup, apply_deferred)
                    .chain()
                    .in_set(CameraSpawn)
                    .after(PlayerSpawn),
            );
    }
}

#[derive(Clone, Debug, Copy, PartialEq, Eq, Reflect, Hash)]
pub enum CameraMovement {
    Pan,
}

impl Actionlike for CameraMovement {
    fn input_control_kind(&self) -> InputControlKind {
        InputControlKind::DualAxis
    }
}
#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash)]
pub struct CameraSpawn;

pub fn cam_setup(mut commands: Commands) {
    let input_map: InputMap<CameraMovement> =
        InputMap::default().with_dual_axis(CameraMovement::Pan, MouseMove::default());

    let initial_angle_degrees: f32 = 40.0;
    let initial_angle: f32 = initial_angle_degrees.to_radians();
    commands
        .spawn((
            Camera {
                hdr: true,
                ..default()
            },
            Camera3d::default(),
            Transform::from_xyz(0., 0., 0.),
            Projection::Perspective(PerspectiveProjection {
                fov: initial_angle,
                ..Default::default()
            }),
            Msaa::Sample4,
        ))
        .insert(InputManagerBundle::<CameraMovement> {
            input_map,
            ..default()
        });
}
