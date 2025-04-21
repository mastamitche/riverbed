use crate::agents::{PlayerControlled, AABB};
use bevy::{
    log::tracing_subscriber::fmt::format,
    prelude::*,
    render::camera::ScalingMode,
    window::{CursorGrabMode, SystemCursorIcon},
};
use bevy_egui::{egui, EguiContexts, EguiPlugin};

pub struct UIPlugin;

impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin);
        app.insert_resource(CameraSettings {
            fov: 40.0,
            height: 15.0,
            x_z_offset: 6.0,
        })
        .insert_resource(CameraOrbit {
            angle: std::f32::consts::PI / 4.0,
            dragging: false,
            last_cursor_pos: Vec2::ZERO,
        })
        .add_systems(
            Update,
            (
                ui_player_system,
                handle_camera_rotation,
                adjust_camera_angle,
                //Debug testing
                // ui_camera_system,
                // update_camera_projection,
            ),
        );
    }
}
// Add this new resource to track camera orbit state
#[derive(Resource)]
pub struct CameraOrbit {
    pub angle: f32,            // Current orbital angle (yaw)
    pub dragging: bool,        // Whether we're currently dragging
    pub last_cursor_pos: Vec2, // Last cursor position for delta calculation
}
#[derive(Resource)]
pub struct CameraSettings {
    fov: f32,
    height: f32,
    x_z_offset: f32,
}

// Add this system to handle mouse input for camera rotation
fn handle_camera_rotation(
    mut camera_orbit: ResMut<CameraOrbit>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    mut cursor_moved_events: EventReader<CursorMoved>,
    windows: Query<&Window>,
) {
    let window = windows.single();

    // Start dragging when right mouse button is pressed
    if mouse_button_input.just_pressed(MouseButton::Right) {
        camera_orbit.dragging = true;
        if let Some(cursor_position) = window.cursor_position() {
            camera_orbit.last_cursor_pos = cursor_position;
        }
    }

    // Stop dragging when right mouse button is released
    if mouse_button_input.just_released(MouseButton::Right) {
        camera_orbit.dragging = false;
    }

    // If dragging, update the angle based on cursor movement
    if camera_orbit.dragging {
        for event in cursor_moved_events.read() {
            let delta = event.position - camera_orbit.last_cursor_pos;

            // Adjust rotation speed as needed
            let rotation_speed = 0.005;
            camera_orbit.angle += delta.x * rotation_speed;

            // Wrap angle to keep it between 0 and 2π
            camera_orbit.angle %= (2.0 * std::f32::consts::PI);
            if camera_orbit.angle < 0.0 {
                camera_orbit.angle += 2.0 * std::f32::consts::PI;
            }

            camera_orbit.last_cursor_pos = event.position;
        }
    }
}

pub fn adjust_camera_angle(
    camera_settings: Res<CameraSettings>,
    camera_orbit: Res<CameraOrbit>,
    mut query: Query<&mut Transform, With<Camera3d>>,
    player_query: Query<(Entity, &AABB, &Transform), (With<PlayerControlled>, Without<Camera3d>)>,
) {
    let mut camera_transform = query.single_mut();
    let (_, _, player_transform) = player_query.single();

    let player_pos = player_transform.translation;

    let camera_pos = Vec3::new(
        player_pos.x + camera_settings.x_z_offset * camera_orbit.angle.cos(),
        player_pos.y + camera_settings.height,
        player_pos.z + camera_settings.x_z_offset * camera_orbit.angle.sin(),
    );

    camera_transform.translation = camera_pos;

    camera_transform.look_at(player_pos, Vec3::Y);
}

fn ui_camera_system(
    mut contexts: EguiContexts,
    mut camera_settings: ResMut<CameraSettings>,
    player_query: Query<(Entity, &AABB, &Transform), (With<PlayerControlled>, Without<Camera3d>)>,
    query: Query<&Transform, With<Camera3d>>,
) {
    egui::Window::new("Camera Settings").show(contexts.ctx_mut(), |ui| {
        ui.add(egui::Slider::new(&mut camera_settings.fov, 5.0..=120.0).text("fov"));
        ui.add(egui::Slider::new(&mut camera_settings.height, 1.0..=500.0).text("Height"));
        ui.add(
            egui::Slider::new(&mut camera_settings.x_z_offset, 1.0..=500.0)
                .text("Distance off center"),
        );
    });
}
fn ui_player_system(
    mut contexts: EguiContexts,
    mut camera_settings: ResMut<CameraSettings>,
    player_query: Query<(Entity, &AABB, &Transform), (With<PlayerControlled>, Without<Camera3d>)>,
    query: Query<&Transform, With<Camera3d>>,
) {
    let player_pos = format!(
        "Player pos: x: {}, y: {}, z: {}",
        player_query.single().2.translation.x.floor(),
        player_query.single().2.translation.y.floor(),
        player_query.single().2.translation.z.floor()
    );
    let camera_pos = format!(
        "Camera pos: x: {}, y: {}, z: {}",
        query.single().translation.x.floor(),
        query.single().translation.y.floor(),
        query.single().translation.z.floor()
    );
    egui::Window::new("Player ").show(contexts.ctx_mut(), |ui| {
        ui.label(player_pos);
        ui.label(camera_pos);
    });
}

fn update_camera_projection(
    camera_settings: Res<CameraSettings>,
    mut query: Query<&mut Projection, With<Camera3d>>,
) {
    for mut projection in query.iter_mut() {
        *projection = Projection::Perspective(PerspectiveProjection {
            fov: camera_settings.fov.to_radians(),
            ..Default::default()
        });
    }
}
fn grab_cursor(mut windows: Query<&mut Window>) {
    let Ok(mut window) = windows.get_single_mut() else {
        return;
    };
    window.cursor_options.visible = false;
    window.cursor_options.grab_mode = CursorGrabMode::Confined;
}

fn free_cursor(mut windows: Query<&mut Window>) {
    let Ok(mut window) = windows.get_single_mut() else {
        return;
    };
    window.cursor_options.visible = true;
    window.cursor_options.grab_mode = CursorGrabMode::None;
}
