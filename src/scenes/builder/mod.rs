use bevy::{
    asset::RenderAssetUsages,
    core_pipeline::experimental::taa::TemporalAntiAliasing,
    pbr::ScreenSpaceAmbientOcclusion,
    prelude::*,
    render::{
        camera::RenderTarget,
        render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
        view::RenderLayers,
    },
};
use bevy_egui::{egui, EguiContextPass, EguiContexts, EguiGlobalSettings, EguiUserTextures};

use crate::utils::INITIAL_FOV;

pub const BUILDER_Y: f32 = 1000.0;
pub struct BuilderPlugin;
impl Plugin for BuilderPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app
            .add_systems(Startup, create_area)
            .add_systems(
                EguiContextPass,
                (render_to_image_example_system),
            )
        //b
        ;
    }
}

#[derive(Deref, Resource)]
struct EditorRenderTexture(Handle<Image>);

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
    let first_pass_layer = RenderLayers::layer(1);

    egui_user_textures.add_image(image_handle.clone());
    commands.insert_resource(EditorRenderTexture(image_handle.clone()));
    let preview_pass_layer = RenderLayers::layer(1);
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
            Transform::from_translation(Vec3::new(2.0, BUILDER_Y, 0.0))
                .looking_at(Vec3::new(0.0, BUILDER_Y, 0.0), Vec3::Y),
            Projection::Perspective(PerspectiveProjection {
                fov: INITIAL_FOV,
                ..Default::default()
            }),
            Msaa::Off,
            ScreenSpaceAmbientOcclusion::default(),
            TemporalAntiAliasing::default(),
            first_pass_layer,
        ))
        .insert(Name::new("Builder Camera"))
        .insert(preview_pass_layer.clone());

    let black_material = materials.add(StandardMaterial {
        base_color: Color::BLACK,
        unlit: true,
        ..default()
    });

    let horizontal_panel = Mesh3d(meshes.add(Cuboid::new(100.0, 10.0, 100.0)));

    let vertical_panel_xz = Mesh3d(meshes.add(Cuboid::new(100.0, 100.0, 10.0)));
    let vertical_panel_yz = Mesh3d(meshes.add(Cuboid::new(10.0, 100.0, 100.0)));

    let inner_size = 62.0;
    let half_size = inner_size / 2.0;

    // Top face
    commands
        .spawn((
            horizontal_panel.clone(),
            MeshMaterial3d(black_material.clone()),
            Transform::from_xyz(0.0, BUILDER_Y + half_size, 0.0),
            Pickable {
                should_block_lower: true,
                is_hoverable: false,
            },
        ))
        .insert(preview_pass_layer.clone());

    // Bottom face
    commands
        .spawn((
            horizontal_panel,
            MeshMaterial3d(black_material.clone()),
            Transform::from_xyz(0.0, BUILDER_Y - half_size, 0.0),
            Pickable {
                should_block_lower: true,
                is_hoverable: false,
            },
        ))
        .insert(preview_pass_layer.clone());

    // Front face (Z+)
    commands
        .spawn((
            vertical_panel_xz.clone(),
            MeshMaterial3d(black_material.clone()),
            Transform::from_xyz(0.0, BUILDER_Y, half_size),
            Pickable {
                should_block_lower: true,
                is_hoverable: false,
            },
        ))
        .insert(preview_pass_layer.clone());

    // Back face (Z-)
    commands
        .spawn((
            vertical_panel_xz,
            MeshMaterial3d(black_material.clone()),
            Transform::from_xyz(0.0, BUILDER_Y, -half_size),
            Pickable {
                should_block_lower: true,
                is_hoverable: false,
            },
        ))
        .insert(preview_pass_layer.clone());

    // Left face (X-)
    commands
        .spawn((
            vertical_panel_yz.clone(),
            MeshMaterial3d(black_material.clone()),
            Transform::from_xyz(-half_size, BUILDER_Y, 0.0),
            Pickable {
                should_block_lower: true,
                is_hoverable: false,
            },
        ))
        .insert(preview_pass_layer.clone());

    // Right face (X+)
    commands
        .spawn((
            vertical_panel_yz,
            MeshMaterial3d(black_material),
            Transform::from_xyz(half_size, BUILDER_Y, 0.0),
            Pickable {
                should_block_lower: true,
                is_hoverable: false,
            },
        ))
        .insert(preview_pass_layer.clone());

    let red_cube = Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0)));
    commands
        .spawn((
            red_cube.clone(),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgba(1.0, 0.0, 0.0, 1.0),
                ..default()
            })),
            Transform::from_xyz(0., BUILDER_Y, 0.),
            Pickable {
                should_block_lower: true,
                is_hoverable: true,
            },
        ))
        .insert(preview_pass_layer.clone());
}

fn render_to_image_example_system(
    cube_preview_image: Res<EditorRenderTexture>,
    mut contexts: EguiContexts,
) -> Result {
    let cube_preview_texture_id = contexts.image_id(&cube_preview_image).unwrap();

    let ctx = contexts.ctx_mut();
    egui::Window::new("Cube material preview")
        .collapsible(false)
        .movable(false)
        .title_bar(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.image(egui::load::SizedTexture::new(
                cube_preview_texture_id,
                egui::vec2(300., 300.),
            ));
            if ui.ui_contains_pointer() {
                ui.input(|i| {
                    if i.pointer.button_down(egui::PointerButton::Secondary) {
                        println!("Right click down")
                    }
                });
            }
        });

    Ok(())
}
