include!(concat!(env!("OUT_DIR"), "/blocks.rs"));
use agents::{MovementPlugin, PlayerPlugin};
use bevy::{
    core_pipeline::experimental::taa::TemporalAntiAliasPlugin,
    image::{ImageAddressMode, ImageFilterMode, ImageSamplerDescriptor},
    prelude::*,
    render::{
        settings::{Backends, RenderCreation, WgpuSettings},
        RenderPlugin,
    },
    text::FontSmoothing,
};
use bevy_dev_tools::fps_overlay::{FpsOverlayConfig, FpsOverlayPlugin};
use render::{Render, TextureLoadPlugin};
use sounds::SoundPlugin;
use ui::UIPlugin;
use world::GenPlugin;
use world::VoxelWorld;

use crate::agents;
use crate::render;
use crate::sounds;
use crate::ui;
use crate::world;

const SEED: u64 = 42;

pub fn create_app() {
    let mut app = App::new();
    // #[cfg(not(feature = "web"))]
    // app.add_plugins(FpsOverlayPlugin {
    //     config: FpsOverlayConfig {
    //         text_config: TextFont {
    //             font_size: 15.0,
    //             font: default(),
    //             font_smoothing: FontSmoothing::default(),
    //         },
    //         text_color: Color::WHITE,
    //         enabled: true,
    //     },
    // });
    app.insert_resource(VoxelWorld::new())
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        fit_canvas_to_parent: true,
                        //present_mode: bevy::window::PresentMode::Immediate,
                        resolution: (1920.0, 1080.0).into(),
                        prevent_default_event_handling: false,
                        ..default()
                    }),
                    ..default()
                })
                .set(RenderPlugin {
                    render_creation: RenderCreation::Automatic(WgpuSettings {
                        backends: Some(
                            Backends::BROWSER_WEBGPU
                                | Backends::GL
                                | Backends::VULKAN
                                | Backends::METAL,
                        ),
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin {
                    default_sampler: ImageSamplerDescriptor {
                        address_mode_u: ImageAddressMode::Repeat,
                        address_mode_v: ImageAddressMode::Repeat,
                        mag_filter: ImageFilterMode::Nearest,
                        min_filter: ImageFilterMode::Nearest,
                        mipmap_filter: ImageFilterMode::Nearest,
                        ..default()
                    },
                }),
        )
        // .add_plugins(TemporalAntiAliasPlugin)
        .add_plugins(PlayerPlugin)
        .add_plugins(UIPlugin)
        .add_plugins(MovementPlugin)
        .add_plugins(GenPlugin)
        .add_plugins(Render)
        .add_plugins(SoundPlugin)
        .run();
}
