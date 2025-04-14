use crate::agents::{PlayerControlled, AABB};
use bevy::{
    prelude::*,
    render::camera::ScalingMode,
    window::{CursorGrabMode, SystemCursorIcon},
};
use bevy_egui::{egui, EguiContexts, EguiPlugin};

pub struct UIPlugin;

impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin);
        let initial_pitch_degrees: f32 = 15.0; // Adjust this value as needed (0 is directly top-down)
        let initial_pitch: f32 = initial_pitch_degrees.to_radians();
        app.insert_resource(CameraSettings {
            projection_type: ProjectionType::Perspective,
            fov: 60.0,
            scale: 5.0,
            near: 0.1,
            far: 10000.0,
            distance: 100.0,
            pitch: initial_pitch,
            yaw: 0.0,
        })
        .add_systems(
            Update,
            (ui_system, adjust_camera_angle, update_camera_projection),
        );
    }
}
#[derive(PartialEq)]
enum ProjectionType {
    Perspective,
    Orthographic,
}

#[derive(Resource)]
pub struct CameraSettings {
    projection_type: ProjectionType,
    // Perspective settings
    fov: f32,
    // Orthographic settings
    scale: f32,
    // Shared settings
    near: f32,
    far: f32,
    distance: f32,
    pitch: f32,
    yaw: f32,
}

pub fn adjust_camera_angle(
    camera_settings: Res<CameraSettings>,
    mut query: Query<&mut Transform, With<Camera3d>>,
    player_query: Query<(Entity, &AABB, &Transform), (With<PlayerControlled>, Without<Camera3d>)>,
) {
    let mut camera_transform = query.single_mut();
    let (_, _, player_transform) = player_query.single();

    // Get the player's position
    let player_pos = player_transform.translation;

    // Calculate the new camera position
    let pitch = camera_settings.pitch.to_radians();
    let yaw = camera_settings.yaw.to_radians();

    // Calculate the new camera position
    let offset = Vec3::new(
        camera_settings.distance * yaw.sin() * pitch.cos(),
        camera_settings.distance * pitch.sin(),
        -camera_settings.distance * yaw.cos() * pitch.cos(),
    );

    // Set the new camera position relative to the player
    camera_transform.translation = player_pos + offset;

    // Make the camera look at the player
    camera_transform.look_at(player_pos, Vec3::Y);
}

fn ui_system(mut contexts: EguiContexts, mut camera_settings: ResMut<CameraSettings>) {
    egui::Window::new("Camera Settings").show(contexts.ctx_mut(), |ui| {
        ui.radio_value(
            &mut camera_settings.projection_type,
            ProjectionType::Perspective,
            "Perspective",
        );
        ui.radio_value(
            &mut camera_settings.projection_type,
            ProjectionType::Orthographic,
            "Orthographic",
        );

        ui.add(egui::Slider::new(&mut camera_settings.distance, 1.0..=3000.0).text("Height"));
        ui.add(egui::Slider::new(&mut camera_settings.pitch, 0.0..=90.0).text("Pitch"));
        ui.add(egui::Slider::new(&mut camera_settings.yaw, 0.0..=360.0).text("Yaw"));
        // ui.add(egui::Slider::new(&mut camera_settings.near, 0.1..=100.0).text("Near"));
        // ui.add(egui::Slider::new(&mut camera_settings.far, 100.0..=10000.0).text("Far"));

        match camera_settings.projection_type {
            ProjectionType::Perspective => {
                ui.add(egui::Slider::new(&mut camera_settings.fov, 0.0..=120.0).text("FOV"));
            }
            ProjectionType::Orthographic => {
                ui.add(egui::Slider::new(&mut camera_settings.scale, 0.1..=20.0).text("Scale"));
            }
        }
    });
}

fn update_camera_projection(
    camera_settings: Res<CameraSettings>,
    mut query: Query<&mut Projection, With<Camera3d>>,
) {
    for mut projection in query.iter_mut() {
        match camera_settings.projection_type {
            ProjectionType::Perspective => {
                *projection = Projection::Perspective(PerspectiveProjection {
                    fov: camera_settings.fov.to_radians(),
                    aspect_ratio: 1.0, // This should be set correctly based on window size
                    near: camera_settings.near,
                    far: camera_settings.far,
                });
            }
            ProjectionType::Orthographic => {
                *projection = Projection::Orthographic(OrthographicProjection {
                    scale: camera_settings.scale,
                    near: camera_settings.near,
                    far: camera_settings.far,
                    viewport_origin: Vec2::new(0.5, 0.5),
                    scaling_mode: ScalingMode::WindowSize,
                    area: Rect::new(-1.0, -1.0, 1.0, 1.0),
                });
            }
        }
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
