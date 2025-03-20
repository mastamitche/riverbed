use bevy::{
    app::Plugin,
    ecs::{component::Component, system::Resource},
    render::{
        extract_component::ExtractComponent,
        render_resource::{
            BindGroupLayout, CachedComputePipelineId, CachedRenderPipelineId, Sampler,
            TextureFormat,
        },
        texture::CachedTexture,
    },
    state::commands,
};
mod ao_gen_prepass_node;
mod blur_prepass_node;
mod hbao;
mod linear_depth_prepass_node;
mod postprocessing_node;

pub struct EffectsPlugin;

impl Plugin for EffectsPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_plugins(hbao::HBAOPlugin);
    }
}
