mod agents;
mod asset_processing;
mod block;
mod gen;
mod render;
mod sounds;
mod ui;
mod world;
include!(concat!(env!("OUT_DIR"), "/blocks.rs"));
use agents::{MovementPlugin, PlayerPlugin};
use bevy::{
    image::{ImageAddressMode, ImageFilterMode, ImageSamplerDescriptor},
    prelude::*,
    render::{
        settings::{Backends, RenderCreation, WgpuSettings},
        RenderPlugin,
    },
    text::FontSmoothing,
};
use bevy_dev_tools::fps_overlay::{FpsOverlayConfig, FpsOverlayPlugin};
use rand_chacha::{rand_core::SeedableRng, ChaCha8Rng};
use render::{Render, TextureLoadPlugin};
use sounds::SoundPlugin;
use ui::UIPlugin;
use world::GenPlugin;
use world::VoxelWorld;
const SEED: u64 = 42;

#[derive(Resource)]
pub struct WorldRng {
    pub seed: u64,
    pub rng: ChaCha8Rng,
}

fn main() {
    let mut app = App::new();

    app.insert_resource(VoxelWorld::new())
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Riverbed".into(),
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
        .add_plugins(FpsOverlayPlugin {
            config: FpsOverlayConfig {
                text_config: TextFont {
                    font_size: 15.0,
                    font: default(),
                    font_smoothing: FontSmoothing::default(),
                },
                text_color: Color::WHITE,
                enabled: true,
            },
        })
        .insert_resource(WorldRng {
            seed: SEED,
            rng: ChaCha8Rng::seed_from_u64(SEED),
        })
        .add_plugins(PlayerPlugin)
        .add_plugins(TextureLoadPlugin)
        .add_plugins(UIPlugin)
        .add_plugins(MovementPlugin)
        .add_plugins(GenPlugin)
        .add_plugins(Render)
        .add_plugins(SoundPlugin)
        .run();
}
