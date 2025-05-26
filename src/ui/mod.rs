use crate::{
    agents::{AgentState, PlayerControlled},
    render::camera::{MainCamera, Y_CAM_SPEED},
    utils::lerp,
};
use bevy::{
    input::mouse::MouseWheel,
    log::tracing_subscriber::fmt::format,
    prelude::*,
    render::camera::ScalingMode,
    window::{CursorGrabMode, SystemCursorIcon},
};
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use bevy_inspector_egui::quick::WorldInspectorPlugin;

pub struct UIPlugin;

impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin {
            enable_multipass_for_primary_context: true,
        })
        //World inspector for debugging
        //.add_plugins(WorldInspectorPlugin::new())
        //
        ;
        app.add_systems(
            Update,
            (
                // ui_player_system,
                handle_camera_rotation,
                adjust_camera_angle,
                handle_camera_zoom,
                //Debug testing
                // ui_camera_system,
                // update_camera_projection,
            )
                .run_if(in_state(AgentState::Normal)),
        );
    }
}
#[derive(Component)]
pub struct CameraSmoothing {
    pub target_y: f32,
    pub current_y: f32,
    pub smoothing_factor: f32,
    // Define the target box dimensions (as a percentage of screen)
    pub target_box_width: f32,  // e.g., 0.2 means 20% of screen width
    pub target_box_height: f32, // e.g., 0.2 means 20% of screen height
    pub last_player_pos: Vec3,  // Track last position to calculate movement
}

impl Default for CameraSmoothing {
    fn default() -> Self {
        Self {
            target_y: 0.0,
            current_y: 0.0,
            smoothing_factor: 0.1,  // Lower = smoother but slower
            target_box_width: 0.2,  // 20% of screen width
            target_box_height: 0.2, // 20% of screen height
            last_player_pos: Vec3::ZERO,
        }
    }
}

#[derive(Component)]
pub struct CameraOrbit {
    pub angle: f32,            // Current orbital angle (yaw)
    pub pitch: f32,            // Current pitch
    pub dragging: bool,        // Whether we're currently dragging
    pub last_cursor_pos: Vec2, // Last cursor position for delta calculation
}
#[derive(Component)]
pub struct CameraSettings {
    pub fov: f32,
    pub height: f32,
    pub x_z_offset: f32,
}

// Add this system to handle mouse input for camera rotation
fn handle_camera_rotation(
    mut query: Query<(&mut Transform, &mut CameraOrbit, &MainCamera), With<Camera3d>>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    mut cursor_moved_events: EventReader<CursorMoved>,
    windows: Query<&Window>,
) {
    if let Ok(window) = windows.single() {
        let (_, mut camera_orbit, _) = query.single_mut().unwrap();

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
}

pub fn adjust_camera_angle(
    mut query: Query<
        (
            &mut Transform,
            &mut CameraOrbit,
            &mut CameraSmoothing,
            &CameraSettings,
            &MainCamera,
        ),
        With<Camera3d>,
    >,
    player_query: Query<(Entity, &Transform), (With<PlayerControlled>, Without<Camera3d>)>,
    time: Res<Time>,
    windows: Query<&Window>,
) {
    let (mut camera_transform, mut camera_orbit, mut camera_smoothing, camera_settings, _) =
        query.single_mut().unwrap();
    if let Ok((_, player_transform)) = player_query.single() {
        let player_pos = player_transform.translation;

        // Update target Y position - this is what we'll smoothly move toward
        camera_smoothing.target_y = player_pos.y + camera_settings.height;

        // Smooth Y movement using lerp
        camera_smoothing.current_y = lerp(
            camera_smoothing.current_y,
            camera_smoothing.target_y,
            camera_smoothing.smoothing_factor * time.delta_secs() * Y_CAM_SPEED,
        );

        // Calculate base camera position using orbital angle
        let base_camera_pos = Vec3::new(
            player_pos.x + camera_settings.x_z_offset * camera_orbit.angle.cos(),
            camera_smoothing.current_y, // Use smoothed Y value
            player_pos.z + camera_settings.x_z_offset * camera_orbit.angle.sin(),
        );

        // Calculate player movement since last frame
        let player_movement = player_pos - camera_smoothing.last_player_pos;
        camera_smoothing.last_player_pos = player_pos;

        // Project player position onto camera's view plane
        let window = windows.single().unwrap();
        let window_size = Vec2::new(window.width(), window.height());

        // Calculate view direction and right vector
        let view_dir = (player_pos - camera_transform.translation).normalize();
        let right = view_dir.cross(Vec3::Y).normalize();
        let up = right.cross(view_dir).normalize();

        // Calculate the target box size in world units at player distance
        let distance_to_player = (player_pos - camera_transform.translation).length();
        let target_box_half_width =
            window_size.x * camera_smoothing.target_box_width * 0.5 * distance_to_player / 1000.0;
        let target_box_half_height =
            window_size.y * camera_smoothing.target_box_height * 0.5 * distance_to_player / 1000.0;

        // Project player movement onto camera plane
        let right_movement = player_movement.dot(right);
        let up_movement = player_movement.dot(up);

        // Calculate camera adjustment to keep player in target box
        let mut camera_adjustment = Vec3::ZERO;

        // Only adjust camera if player moves outside target box
        if right_movement.abs() > target_box_half_width {
            let excess = right_movement.abs() - target_box_half_width;
            camera_adjustment += right * excess.signum() * right_movement.signum() * excess;
        }

        if up_movement.abs() > target_box_half_height {
            let excess = up_movement.abs() - target_box_half_height;
            camera_adjustment += up * excess.signum() * up_movement.signum() * excess;
        }

        // Apply camera position with adjustment
        camera_transform.translation = base_camera_pos + camera_adjustment;

        // Look at player position
        camera_transform.look_at(player_pos, Vec3::Y);
    }
}

fn ui_camera_system(
    mut contexts: EguiContexts,
    mut query: Query<(&Transform, &mut CameraSettings), With<Camera3d>>,
) {
    if let Ok((_camera_transform, mut camera_settings)) = query.single_mut() {
        egui::Window::new("Camera Settings").show(contexts.ctx_mut(), |ui| {
            ui.add(egui::Slider::new(&mut camera_settings.fov, 5.0..=120.0).text("fov"));
            ui.add(egui::Slider::new(&mut camera_settings.height, 1.0..=500.0).text("Height"));
            ui.add(
                egui::Slider::new(&mut camera_settings.x_z_offset, 1.0..=500.0)
                    .text("Distance off center"),
            );
        });
    }
}
fn ui_player_system(
    mut contexts: EguiContexts,
    player_query: Query<(Entity, &Transform), (With<PlayerControlled>, Without<Camera3d>)>,
    mut query: Query<(&Transform, &mut CameraSettings), With<Camera3d>>,
) {
    if let Ok((camera_transform, _)) = query.single_mut() {
        let player_pos = format!(
            "Player pos: x: {}, y: {}, z: {}",
            player_query.single().unwrap().1.translation.x.floor(),
            player_query.single().unwrap().1.translation.y.floor(),
            player_query.single().unwrap().1.translation.z.floor()
        );
        let camera_pos = format!(
            "Camera pos: x: {}, y: {}, z: {}",
            camera_transform.translation.x.floor(),
            camera_transform.translation.y.floor(),
            camera_transform.translation.z.floor()
        );
        egui::Window::new("Player ").show(contexts.ctx_mut(), |ui| {
            ui.label(player_pos);
            ui.label(camera_pos);
        });
    }
}

fn update_camera_projection(mut query: Query<(&mut Projection, &CameraSettings), With<Camera3d>>) {
    for (mut projection, camera_settings) in query.iter_mut() {
        *projection = Projection::Perspective(PerspectiveProjection {
            fov: camera_settings.fov.to_radians(),
            ..Default::default()
        });
    }
}
fn handle_camera_zoom(
    mut mouse_wheel_events: EventReader<MouseWheel>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    mut query: Query<(&Transform, &mut CameraSettings), (With<Camera3d>, With<MainCamera>)>,
) {
    if let Ok((_, mut camera_settings)) = query.single_mut() {
        // Skip zoom if right mouse button is pressed (to avoid conflict with rotation)
        if mouse_button_input.pressed(MouseButton::Right) {
            return;
        }

        // Process all scroll events
        for event in mouse_wheel_events.read() {
            let zoom_speed = 6.;
            let zoom_delta = event.y * zoom_speed;

            camera_settings.height = (camera_settings.height - zoom_delta).clamp(5.0, 40.0);
        }
    }
}
