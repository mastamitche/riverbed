use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;

use crate::render::camera::MainCamera;

use super::AgentState;

pub struct FreeFlyPlugin;

const SUPER_SPEED: f32 = 5.0;

impl Plugin for FreeFlyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (free_fly_camera_movement, free_fly_camera_rotation)
                .run_if(in_state(AgentState::FreeFly)),
        )
        .insert_resource(FreeFlySettings::default());
    }
}

// Resource to store settings
#[derive(Resource)]
pub struct FreeFlySettings {
    pub movement_speed: f32,
    pub rotation_speed: f32,
    pub enabled: bool,
}

impl Default for FreeFlySettings {
    fn default() -> Self {
        Self {
            movement_speed: 5.0,
            rotation_speed: 0.1,
            enabled: true,
        }
    }
}

// System to handle camera movement
fn free_fly_camera_movement(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    settings: Res<FreeFlySettings>,
    mut query: Query<(&mut Transform, &MainCamera), With<Camera>>,
) {
    if !settings.enabled {
        return;
    }

    let mut direction = Vec3::ZERO;

    if keys.pressed(KeyCode::KeyW) {
        direction.z -= 1.0;
    }
    if keys.pressed(KeyCode::KeyS) {
        direction.z += 1.0;
    }
    if keys.pressed(KeyCode::KeyA) {
        direction.x -= 1.0;
    }
    if keys.pressed(KeyCode::KeyD) {
        direction.x += 1.0;
    }
    if keys.pressed(KeyCode::KeyE) {
        direction.y += 1.0;
    }
    if keys.pressed(KeyCode::KeyQ) {
        direction.y -= 1.0;
    }
    if direction.length_squared() > 0.0 {
        direction = direction.normalize();
    }
    if keys.pressed(KeyCode::ShiftLeft) {
        direction *= SUPER_SPEED;
    }

    if let Ok((mut cam, _)) = query.single_mut() {
        let cam_rotation = cam.rotation;
        let movement = direction * settings.movement_speed * time.delta_secs();
        cam.translation += cam_rotation * movement;
    }
}

// System to handle camera rotation
fn free_fly_camera_rotation(
    mut motion_events: EventReader<MouseMotion>,
    settings: Res<FreeFlySettings>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut query: Query<(&mut Transform, &MainCamera), With<Camera>>,
) {
    if !settings.enabled || !mouse_buttons.pressed(MouseButton::Right) {
        return;
    }

    let mut rotation = Vec2::ZERO;
    for event in motion_events.read() {
        rotation += event.delta;
    }

    if rotation.length_squared() > 0.0 {
        if let Ok((mut transform, _)) = query.single_mut() {
            let pitch = (rotation.y * settings.rotation_speed).to_radians();
            let yaw = (rotation.x * settings.rotation_speed).to_radians();

            // Apply yaw rotation
            transform.rotate_y(-yaw);

            // Apply pitch rotation (with local x axis)
            let right = transform.right();
            transform.rotate_axis(right, -pitch);
        }
    }
}
