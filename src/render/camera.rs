use crate::agents::{PlayerControlled, PlayerSpawn, AABB};
use bevy::core_pipeline::experimental::taa::TemporalAntiAliasing;
use bevy::pbr::ScreenSpaceAmbientOcclusion;
use bevy::render::camera::ScalingMode;
use bevy::window::CursorGrabMode;
use bevy::{pbr::VolumetricFog, prelude::*};
use leafwing_input_manager::prelude::*;
use std::f32::consts::PI;

const CAMERA_PAN_RATE: f32 = 0.06;

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

pub fn cam_setup(
    mut commands: Commands,
    player_query: Query<(Entity, &AABB, &Transform), With<PlayerControlled>>,
) {
    let input_map = InputMap::default().with_dual_axis(CameraMovement::Pan, MouseMove::default());
    let (player, aabb, transform) = player_query.get_single().unwrap();

    let initial_angle_degrees: f32 = 15.0; // Adjust this value as needed (0 is directly top-down)
    let initial_angle: f32 = initial_angle_degrees.to_radians();
    // Calculate camera position based on the angle
    let camera_height = 100.0; // Adjust this value to change the camera's height
    let camera_offset_z = camera_height * initial_angle.sin();

    let cam = commands
        .spawn((
            Camera {
                hdr: true,
                ..default()
            },
            Camera3d::default(),
            Transform::from_xyz(
                transform.translation.x,
                transform.translation.y + camera_height,
                transform.translation.z,
            )
            .looking_at(transform.translation, Vec3::Y),
            Projection::Perspective(PerspectiveProjection {
                fov: 15.,
                ..Default::default()
            }),
            DistanceFog {
                color: Color::linear_rgba(0.70, 0.85, 0.95, 1.0),
                falloff: FogFalloff::Linear {
                    start: 100.0,
                    end: 10000.0,
                },
                ..default()
            },
            Msaa::Sample4,
            VolumetricFog::default(),
        ))
        .insert(InputManagerBundle::<CameraMovement> {
            input_map,
            ..default()
        })
        .id();
    commands.entity(player).add_child(cam);
}
