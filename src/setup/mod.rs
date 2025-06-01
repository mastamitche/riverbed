use std::time::Duration;

use avian3d::{prelude::Gravity, PhysicsPlugins};
use bevy::{
    core_pipeline::experimental::taa::TemporalAntiAliasPlugin,
    image::{ImageAddressMode, ImageFilterMode, ImageSamplerDescriptor},
    prelude::*,
    remote::{http::RemoteHttpPlugin, RemotePlugin},
    render::{
        settings::{Backends, RenderCreation, WgpuSettings},
        RenderPlugin,
    },
    text::FontSmoothing,
};
use bevy_dev_tools::{
    fps_overlay::{FpsOverlayConfig, FpsOverlayPlugin},
    picking_debug::DebugPickingMode,
};
use render::Render;
use sounds::SoundPlugin;
use ui::UIPlugin;
use world::GenPlugin;
use world::VoxelWorld;

use crate::ui;
use crate::world;
use crate::{
    agents::{self, AgentsPlugin},
    controls::ActionMappingPlugin,
};
use crate::{interactions::PlayerInteractionsPlugin, render};
use crate::{scenes::ScenesPlugin, sounds};

pub fn create_app() {
    let mut app = App::new();
    // #[cfg(not(feature = "web"))]
    app.add_plugins(FpsOverlayPlugin {
        config: FpsOverlayConfig {
            text_config: TextFont {
                font_size: 15.0,
                line_height: bevy::text::LineHeight::Px(15.0),
                font: default(),
                font_smoothing: FontSmoothing::default(),
            },
            refresh_interval: Duration::from_millis(500),
            text_color: Color::WHITE,
            enabled: true,
        },
    });
    app.insert_resource(VoxelWorld::new())
        .insert_resource(Gravity(Vec3::NEG_Y * 19.6))
        .add_plugins((
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
            PhysicsPlugins::default(),
        ))
        .add_plugins(AgentsPlugin)
        .add_plugins(UIPlugin)
        .add_plugins(GenPlugin)
        .add_plugins(ActionMappingPlugin)
        .add_plugins(Render)
        .add_plugins(SoundPlugin)
        .add_plugins(PlayerInteractionsPlugin)
        .add_plugins(TemporalAntiAliasPlugin)
        .add_plugins(ScenesPlugin)
        .add_plugins((MeshPickingPlugin)) //, DebugPickingPlugin))
        .add_plugins(RemotePlugin::default())
        .add_plugins(RemoteHttpPlugin::default())
        .insert_resource(DebugPickingMode::Normal)
        .run();
}
