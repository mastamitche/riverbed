use crate::{
    agents::{PlayerControlled, AABB},
    render::hbao::hbao::{AOApplicationParams, AOGenParams, BlurParams},
};
use bevy::{
    prelude::*,
    render::{camera::ScalingMode, extract_resource::ExtractResource},
    window::{CursorGrabMode, SystemCursorIcon},
};
use bevy_egui::{egui, EguiContexts, EguiPlugin};

use crate::render::sky::sky::Sun;

#[derive(Resource)]
pub struct CameraSettings {
    // Perspective settings
    fov: f32,
    // Shared settings
    near: f32,
    far: f32,
    distance: f32,
    pitch: f32,
    yaw: f32,
}
#[derive(Resource, ExtractResource, Clone, Copy)]
pub struct LightingSettings {
    pub ao_gen_params: AOGenParams,
    pub blur_params: BlurParams,
    pub ao_application_params: AOApplicationParams,
    ambient_strength: f32,
    directional_strength: f32,
}
impl Default for LightingSettings {
    fn default() -> Self {
        Self {
            ao_gen_params: AOGenParams::default(),
            blur_params: BlurParams::default(),
            ao_application_params: AOApplicationParams::default(),
            ambient_strength: 800.0,
            directional_strength: 800.0,
        }
    }
}

pub struct UIPlugin;

impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin);
        let initial_pitch_degrees: f32 = 15.0; // Adjust this value as needed (0 is directly top-down)
        let initial_pitch: f32 = initial_pitch_degrees.to_radians();
        app.insert_resource(CameraSettings {
            fov: 10.0,
            near: 0.1,
            far: 10000.0,
            distance: 5.0,
            pitch: initial_pitch,
            yaw: 0.0,
        })
        .init_resource::<LightingSettings>()
        .add_systems(
            Update,
            (
                ui_system,
                adjust_camera_angle,
                update_camera_projection,
                update_lighting,
                lighting_ui_system,
            ),
        );
    }
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

fn ui_system(
    mut contexts: EguiContexts,
    mut camera_settings: ResMut<CameraSettings>,
    mut lighting_settings: ResMut<LightingSettings>,
) {
    egui::Window::new("Camera Settings").show(contexts.ctx_mut(), |ui| {
        ui.add(egui::Slider::new(&mut camera_settings.distance, 1.0..=100.0).text("Distance"));
        ui.add(egui::Slider::new(&mut camera_settings.pitch, 0.0..=90.0).text("Pitch"));
        ui.add(egui::Slider::new(&mut camera_settings.yaw, 0.0..=360.0).text("Yaw"));
        ui.add(egui::Slider::new(&mut camera_settings.fov, 5.0..=90.0).text("FOV"));
    });
}
fn update_lighting(
    mut ambient_light: ResMut<AmbientLight>,
    mut d_l: Query<&mut DirectionalLight, With<Sun>>,
    mut lighting_settings: ResMut<LightingSettings>,
) {
    ambient_light.brightness = lighting_settings.ambient_strength;
    for mut directional_light in d_l.iter_mut() {
        directional_light.illuminance = lighting_settings.directional_strength;
    }
}
fn update_camera_projection(
    camera_settings: Res<CameraSettings>,
    mut query: Query<&mut Projection, With<Camera3d>>,
) {
    for mut projection in query.iter_mut() {
        *projection = Projection::Perspective(PerspectiveProjection {
            fov: camera_settings.fov.to_radians(),
            aspect_ratio: 1.0, // This should be set correctly based on window size
            near: camera_settings.near,
            far: camera_settings.far,
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

pub fn lighting_ui_system(
    mut contexts: EguiContexts,
    mut lighting_settings: ResMut<LightingSettings>,
) {
    egui::Window::new("HBAO Settings").show(contexts.ctx_mut(), |ui| {
        ui.collapsing("AO Generation", |ui| {
            ui.add(
                egui::Slider::new(&mut lighting_settings.ao_gen_params.radius, 0.2..=1.0)
                    .text("Radius"),
            );
            ui.add(
                egui::Slider::new(&mut lighting_settings.ao_gen_params.bias, 0.0001..=0.05)
                    .text("Bias"),
            );
            ui.add(
                egui::Slider::new(&mut lighting_settings.ao_gen_params.strength, 1.0..=8.0)
                    .text("Strength"),
            );
            ui.add(
                egui::Slider::new(&mut lighting_settings.ao_gen_params.num_directions, 4..=16)
                    .text("Num Directions"),
            );
            ui.add(
                egui::Slider::new(&mut lighting_settings.ao_gen_params.num_steps, 3..=6)
                    .text("Num Steps"),
            );
            ui.add(
                egui::Slider::new(
                    &mut lighting_settings.ao_gen_params.max_radius_pixels,
                    5.0..=128.0,
                )
                .text("Max Radius Pixels"),
            );
            ui.add(
                egui::Slider::new(
                    &mut lighting_settings.ao_gen_params.falloff_scale,
                    0.0..=1.0,
                )
                .text("Falloff Scale"),
            );
            ui.add(
                egui::Slider::new(&mut lighting_settings.ao_gen_params.denoise_blur, 0.0..=2.0)
                    .text("Denoise Blur"),
            );
        });

        ui.collapsing("Blur", |ui| {
            ui.add(
                egui::Slider::new(&mut lighting_settings.blur_params.blur_radius, 1.0..=4.0)
                    .text("Blur Radius"),
            );
            ui.add(
                egui::Slider::new(&mut lighting_settings.blur_params.sharpness, 4.0..=16.0)
                    .text("Sharpness"),
            );
            ui.add(
                egui::Slider::new(
                    &mut lighting_settings.blur_params.normal_sensitivity,
                    0.05..=0.5,
                )
                .text("Normal Sensitivity"),
            );
            ui.add(
                egui::Slider::new(
                    &mut lighting_settings.blur_params.depth_sensitivity,
                    0.05..=0.5,
                )
                .text("Depth Sensitivity"),
            );
        });

        ui.collapsing("AO Application", |ui| {
            ui.add(
                egui::Slider::new(
                    &mut lighting_settings.ao_application_params.strength,
                    0.5..=2.0,
                )
                .text("Strength"),
            );
            ui.add(
                egui::Slider::new(
                    &mut lighting_settings.ao_application_params.power,
                    1.0..=3.0,
                )
                .text("Power"),
            );
            ui.add(
                egui::Slider::new(
                    &mut lighting_settings.ao_application_params.distance_falloff_min,
                    0.0..=100.0,
                )
                .text("Distance Falloff Min"),
            );
            ui.add(
                egui::Slider::new(
                    &mut lighting_settings.ao_application_params.distance_falloff_max,
                    100.0..=500.0,
                )
                .text("Distance Falloff Max"),
            );
            ui.add(
                egui::Slider::new(
                    &mut lighting_settings.ao_application_params.use_distance_falloff,
                    0..=1,
                )
                .text("Color Bleed Intensity"),
            );
            ui.radio_value(
                &mut lighting_settings.ao_application_params.multiply_mode,
                0,
                "Multiply Mode",
            );
            ui.radio_value(
                &mut lighting_settings.ao_application_params.multiply_mode,
                1,
                "Overlay Mode",
            );
            ui.add(
                egui::Slider::new(
                    &mut lighting_settings
                        .ao_application_params
                        .color_bleed_intensity,
                    0.0..=1.0,
                )
                .text("Color Bleed Intensity"),
            );
            ui.color_edit_button_rgb(&mut lighting_settings.ao_application_params.ao_color);
        });
        ui.collapsing("General Lighting", |ui| {
            ui.add(
                egui::Slider::new(&mut lighting_settings.ambient_strength, 0.0..=2000.0)
                    .text("Ambient Strength"),
            );
            ui.add(
                egui::Slider::new(&mut lighting_settings.directional_strength, 0.0..=20000.0)
                    .text("Directional Strength"),
            );
        });
    });
}
