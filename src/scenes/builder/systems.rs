use bevy::{
    asset::RenderAssetUsages,
    core_pipeline::experimental::taa::TemporalAntiAliasing,
    pbr::ScreenSpaceAmbientOcclusion,
    prelude::*,
    render::{
        camera::RenderTarget,
        render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
    },
};
use bevy_egui::{egui, EguiContexts, EguiGlobalSettings, EguiUserTextures};

use crate::{
    interactions::place::{PlaceBlockEvent, PlaceDestination},
    render::{
        camera::Y_CAM_SPEED,
        draw_chunks::{BuildingPreview, BuildingState, WorldMesh},
    },
    setup::Block,
    ui::{CameraOrbit, CameraSettings, CameraSmoothing},
    utils::{lerp, INITIAL_FOV},
    world::pos3d::Pos3d,
};

use super::builder_chunk;

pub const BUILDER_Y: f32 = 1054.0;

#[derive(Deref, Resource)]
pub struct EditorRenderTexture(Handle<Image>);
#[derive(Component)]
pub struct BuildCamera;

pub fn create_area(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut egui_user_textures: ResMut<EguiUserTextures>,
    mut egui_global_settings: ResMut<EguiGlobalSettings>,
) {
    egui_global_settings.enable_absorb_bevy_input_system = true;
    let size = Extent3d {
        width: 512,
        height: 512,
        ..default()
    };
    let mut image = Image::new_fill(
        size,
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Bgra8UnormSrgb,
        RenderAssetUsages::default(),
    );
    // You need to set these texture usage flags in order to use the image as a render target
    image.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;

    let image_handle = images.add(image);

    egui_user_textures.add_image(image_handle.clone());
    commands.insert_resource(EditorRenderTexture(image_handle.clone()));
    // Spawn a secondary camera
    commands
        .spawn((
            Camera {
                hdr: true,
                order: -1,
                target: RenderTarget::Image(image_handle.into()),
                clear_color: ClearColorConfig::Custom(Color::srgba(0.0, 0.0, 0.0, 0.0)),
                ..default()
            },
            Camera3d::default(),
            CameraOrbit {
                angle: std::f32::consts::PI / 4.0,
                pitch: 0.,
                dragging: false,
                last_cursor_pos: Vec2::ZERO,
            },
            CameraSmoothing::default(),
            Transform::from_translation(Vec3::new(2.0, BUILDER_Y, 0.0))
                .looking_at(Vec3::new(0.0, BUILDER_Y, 0.0), Vec3::Y),
            Projection::Perspective(PerspectiveProjection {
                fov: INITIAL_FOV,
                ..Default::default()
            }),
            Msaa::Off,
            ScreenSpaceAmbientOcclusion::default(),
            TemporalAntiAliasing::default(),
            BuildCamera,
        ))
        .insert(Name::new("Builder Camera"));

    let black_material = materials.add(StandardMaterial {
        base_color: Color::BLACK,
        unlit: true,
        ..default()
    });

    let horizontal_panel = Mesh3d(meshes.add(Cuboid::new(100.0, 1.0, 100.0)));

    let vertical_panel_xz = Mesh3d(meshes.add(Cuboid::new(100.0, 100.0, 1.0)));
    let vertical_panel_yz = Mesh3d(meshes.add(Cuboid::new(1.0, 100.0, 100.0)));

    let inner_size = 62.0;
    let half_size = inner_size / 2.0;

    // Top face
    commands.spawn((
        horizontal_panel.clone(),
        MeshMaterial3d(black_material.clone()),
        Transform::from_xyz(0.0, BUILDER_Y + half_size, 0.0),
        WorldMesh,
        Pickable {
            should_block_lower: true,
            is_hoverable: true,
        },
    ));

    // Bottom face
    commands.spawn((
        horizontal_panel,
        MeshMaterial3d(black_material.clone()),
        Transform::from_xyz(0.0, BUILDER_Y - half_size, 0.0),
        WorldMesh,
        Pickable {
            should_block_lower: true,
            is_hoverable: true,
        },
    ));

    // Front face (Z+)
    commands.spawn((
        vertical_panel_xz.clone(),
        MeshMaterial3d(black_material.clone()),
        Transform::from_xyz(0.0, BUILDER_Y, half_size),
        WorldMesh,
        Pickable {
            should_block_lower: true,
            is_hoverable: true,
        },
    ));

    // Back face (Z-)
    commands.spawn((
        vertical_panel_xz,
        MeshMaterial3d(black_material.clone()),
        Transform::from_xyz(0.0, BUILDER_Y, -half_size),
        Pickable {
            should_block_lower: true,
            is_hoverable: true,
        },
    ));

    // Left face (X-)
    commands.spawn((
        vertical_panel_yz.clone(),
        MeshMaterial3d(black_material.clone()),
        Transform::from_xyz(-half_size, BUILDER_Y, 0.0),
        WorldMesh,
        Pickable {
            should_block_lower: true,
            is_hoverable: true,
        },
    ));

    // Right face (X+)
    commands.spawn((
        vertical_panel_yz,
        MeshMaterial3d(black_material),
        Transform::from_xyz(half_size, BUILDER_Y, 0.0),
        WorldMesh,
        Pickable {
            should_block_lower: true,
            is_hoverable: true,
        },
    ));

    let red_cube = Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0)));
    commands.spawn((
        red_cube.clone(),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(1.0, 0.0, 0.0, 1.0),
            ..default()
        })),
        WorldMesh,
        Transform::from_xyz(0., BUILDER_Y, 0.),
        Pickable {
            should_block_lower: true,
            is_hoverable: true,
        },
    ));
}

pub fn render_to_image_example_system(
    mesh_pickable_query: Query<(), With<WorldMesh>>,
    cube_preview_image: Res<EditorRenderTexture>,
    mut query: Query<(&GlobalTransform, &mut CameraOrbit, &BuildCamera, &Camera)>,
    mut contexts: EguiContexts,
    mut building_state: ResMut<BuildingState>,
    mut ray_cast: MeshRayCast,
    mut preview_query: Query<(&mut Transform, &mut Visibility), With<BuildingPreview>>,
    mut place_events: EventWriter<PlaceBlockEvent>,
) -> Result {
    let cube_preview_texture_id = contexts.image_id(&cube_preview_image).unwrap();

    let ctx = contexts.ctx_mut();
    egui::Window::new("Cube material preview")
        .collapsible(false)
        .movable(false)
        .title_bar(false)
        .resizable(false)
        .show(ctx, |ui| {
            let image_size = egui::vec2(512., 512.);
            let response = ui.image(egui::load::SizedTexture::new(
                cube_preview_texture_id,
                image_size,
            ));
            if ui.ui_contains_pointer() {
                ui.input(|i| {
                    let (camera_global_transform, mut camera_orbit, _, camera) =
                        query.single_mut().unwrap();
                    if i.pointer.button_down(egui::PointerButton::Secondary) {
                        camera_orbit.dragging = true;
                        let latest_pos = i.pointer.latest_pos().unwrap();
                        let delta = i.pointer.delta();
                        let rotation_speed = 0.005;

                        // Horizontal rotation (yaw)
                        camera_orbit.angle += delta.x * rotation_speed;

                        // Vertical rotation (pitch)
                        camera_orbit.pitch -= delta.y * rotation_speed; // Note: negative to match expected behavior

                        // Wrap angle to keep it between 0 and 2π
                        camera_orbit.angle %= (2.0 * std::f32::consts::PI);
                        if camera_orbit.angle < 0.0 {
                            camera_orbit.angle += 2.0 * std::f32::consts::PI;
                        }

                        // Clamp pitch to prevent camera flipping
                        let pitch_limit = std::f32::consts::PI / 2.0 - 0.01; // Slightly less than 90 degrees
                        camera_orbit.pitch = camera_orbit.pitch.clamp(-pitch_limit, pitch_limit);

                        camera_orbit.last_cursor_pos = Vec2::new(latest_pos.x, latest_pos.y);
                    } else {
                        camera_orbit.dragging = false;
                    }
                    if let Some(pos) = i.pointer.hover_pos() {
                        // Calculate position relative to the image
                        let image_rect = response.rect;
                        let camera_viewport_rect = camera.logical_viewport_rect().unwrap();
                        // First convert to normalized coordinates (0 to 1) within the image
                        let normalized_x = (pos.x - image_rect.min.x) / image_rect.width()
                            * camera_viewport_rect.width();
                        let normalized_y = (pos.y - image_rect.min.y) / image_rect.height()
                            * camera_viewport_rect.height();

                        if let Ok(ray) = camera.viewport_to_world(
                            camera_global_transform,
                            Vec2::new(normalized_x, normalized_y),
                        ) {
                            let visibility = RayCastVisibility::Any;
                            let filter = |entity| mesh_pickable_query.contains(entity);
                            let settings = MeshRayCastSettings::default()
                                .with_filter(&filter)
                                .with_visibility(visibility);

                            // Cast the ray with the settings
                            if let Some(hit) = ray_cast.cast_ray(ray, &settings).first() {
                                //println!("Hit: {:?}", hit);

                                let voxel_size = 0.125;
                                let half_voxel_size = voxel_size / 2.0;
                                let world_position = hit.1.point;

                                let target_voxel_pos = Vec3::new(
                                    (world_position.x / voxel_size).floor() * voxel_size
                                        + half_voxel_size,
                                    (world_position.y / voxel_size).floor() * voxel_size
                                        + half_voxel_size,
                                    (world_position.z / voxel_size).floor() * voxel_size
                                        + half_voxel_size,
                                );

                                if let Ok((mut transform, mut visibility)) =
                                    preview_query.single_mut()
                                {
                                    transform.translation = world_position;
                                    building_state.current_position = Some(target_voxel_pos);
                                    *visibility = Visibility::Visible;
                                }
                            }
                        }
                    }

                    if i.pointer.primary_released() {
                        if let Some(pos) = building_state.current_position {
                            println!("Placing block at {pos:?}");
                            let p: Pos3d<1> = Pos3d {
                                x: (pos.x * 8.) as i32,
                                y: (pos.y * 8.) as i32,
                                z: (pos.z * 8.) as i32,
                            };
                            place_events.write(PlaceBlockEvent {
                                pos: p,
                                block: Block::AcaciaLeaves,
                                destination: PlaceDestination::Builder,
                            });
                        }
                    }
                });
            }
        });

    Ok(())
}

pub fn adjust_camera_angle(
    camera_settings: Res<CameraSettings>,
    mut query: Query<
        (
            &mut Transform,
            &mut CameraOrbit,
            &mut CameraSmoothing,
            &BuildCamera,
        ),
        With<Camera3d>,
    >,
    time: Res<Time>,
    windows: Query<&Window>,
) {
    let (mut camera_transform, mut camera_orbit, mut camera_smoothing, _) =
        query.single_mut().unwrap();
    let player_pos = Vec3::new(0., BUILDER_Y, 0.);

    // Update target Y position - this is what we'll smoothly move toward
    camera_smoothing.target_y = player_pos.y + camera_settings.height;

    // Smooth Y movement using lerp
    camera_smoothing.current_y = lerp(
        camera_smoothing.current_y,
        camera_smoothing.target_y,
        camera_smoothing.smoothing_factor * time.delta_secs() * Y_CAM_SPEED,
    );

    let base_camera_pos = Vec3::new(
        player_pos.x
            + camera_settings.x_z_offset * camera_orbit.angle.cos() * camera_orbit.pitch.cos(),
        player_pos.y - camera_settings.x_z_offset * camera_orbit.pitch.sin(), // Inverted Y position
        player_pos.z
            + camera_settings.x_z_offset * camera_orbit.angle.sin() * camera_orbit.pitch.cos(),
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
