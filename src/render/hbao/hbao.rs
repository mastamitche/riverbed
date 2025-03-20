use bevy::{
    asset::load_internal_asset,
    core_pipeline::{
        core_3d::graph::{Core3d, Node3d},
        fullscreen_vertex_shader::fullscreen_shader_vertex_state,
        prepass::ViewPrepassTextures,
    },
    prelude::*,
    render::{
        camera::ExtractedCamera,
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_graph::{RenderGraphApp, RenderLabel, ViewNodeRunner},
        render_resource::{
            binding_types::{
                sampler, texture_2d, texture_depth_2d, texture_storage_2d, uniform_buffer,
            },
            AddressMode, BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries, Buffer,
            BufferInitDescriptor, BufferUsages, CachedComputePipelineId, CachedRenderPipelineId,
            ColorTargetState, ColorWrites, ComputePipelineDescriptor, Extent3d, FilterMode,
            FragmentState, MultisampleState, PipelineCache, PrimitiveState,
            RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor, ShaderStages,
            ShaderType, StorageTextureAccess, TextureDescriptor, TextureDimension, TextureFormat,
            TextureSampleType, TextureUsages,
        },
        renderer::RenderDevice,
        texture::{CachedTexture, TextureCache},
        view::{ViewTarget, ViewUniform, ViewUniforms},
        RenderApp, RenderSet,
    },
};
use bytemuck::{Pod, Zeroable};

use crate::ui::LightingSettings;

use super::{
    ao_gen_prepass_node::HBAOAoGenPrepassNode, blur_prepass_node::HBAOBlurPrepassNode,
    linear_depth_prepass_node::HBAOLinearDepthPrepassNode,
    postprocessing_node::HBAOApplicationNode,
};

#[derive(Resource, ShaderType, Clone, Copy, Pod, Zeroable, ExtractResource, Reflect)]
#[repr(C)]
pub struct AOGenParams {
    pub radius: f32,
    pub bias: f32,
    pub strength: f32,
    pub num_directions: u32,
    pub num_steps: u32,
    pub max_radius_pixels: f32,
    pub falloff_scale: f32,
    pub denoise_blur: f32,
}
impl Default for AOGenParams {
    fn default() -> Self {
        Self {
            radius: 0.5,             // Typical range: 0.25 to 1.0
            bias: 0.025,             // Typical range: 0.01 to 0.05
            strength: 1.5,           // Typical range: 1.0 to 2.0
            num_directions: 8,       // Common values: 4, 8, or 16
            num_steps: 4,            // Typical range: 3 to 6
            max_radius_pixels: 32.0, // Typical range: 32.0 to 128.0
            falloff_scale: 0.5,      // Typical range: 0.0 to 1.0
            denoise_blur: 1.0,       // Typical range: 0.0 to 2.0
        }
    }
}

#[derive(Resource, ShaderType, Clone, Copy, Pod, Zeroable, ExtractResource, Reflect)]
#[repr(C)]
pub struct BlurParams {
    pub blur_radius: f32,
    pub sharpness: f32,
    pub normal_sensitivity: f32,
    pub depth_sensitivity: f32,
}
impl Default for BlurParams {
    fn default() -> Self {
        Self {
            blur_radius: 2.0,        // Typical range: 1.0 to 4.0
            sharpness: 8.0,          // Typical range: 4.0 to 16.0
            normal_sensitivity: 0.1, // Typical range: 0.05 to 0.5
            depth_sensitivity: 0.1,  // Typical range: 0.05 to 0.5
        }
    }
}

#[derive(Resource, ShaderType, Clone, Copy, Pod, Zeroable, ExtractResource, Reflect)]
#[repr(C)]
pub struct AOApplicationParams {
    pub strength: f32,
    pub power: f32,
    pub distance_falloff_min: f32,
    pub distance_falloff_max: f32,
    pub use_distance_falloff: u32,
    pub multiply_mode: u32,
    pub color_bleed_intensity: f32,
    pub ao_color: [f32; 3],
    pub _padding: [u32; 2],
}

impl Default for AOApplicationParams {
    fn default() -> Self {
        Self {
            strength: 1.0, // Typical range: 0.5 to 2.0
            power: 1.5,    // Typical range: 1.0 to 3.0
            distance_falloff_min: 0.0,
            distance_falloff_max: 300.0, // Adjust based on your scene scale
            use_distance_falloff: 1,     // Enable distance falloff
            multiply_mode: 0,            // 0 for multiply, 1 for overlay
            color_bleed_intensity: 0.0,  // Typically 0.0 unless color bleeding is desired
            ao_color: [0.0, 0.0, 0.0],   // Black for standard AO
            _padding: [0, 0],
        }
    }
}

#[derive(Component)]
pub struct HBAOBindGroups {
    pub common_bind_group: bevy::render::render_resource::BindGroup,
    pub linear_depth_bind_group: bevy::render::render_resource::BindGroup,
    pub ao_gen_bind_group: bevy::render::render_resource::BindGroup,
    pub blur_bind_group: bevy::render::render_resource::BindGroup,
    pub application_bind_group: bevy::render::render_resource::BindGroup,

    pub random_data: Vec<f32>,
}

#[derive(Component)]
pub struct HBAOSharedPipelineResources {
    pub linear_depth_texture: CachedTexture,
    pub normal_texture: CachedTexture,
    pub random_rotation_texture: CachedTexture,
    pub raw_ao_texture: CachedTexture,
    pub blur_ao_texture: CachedTexture,

    pub blur_params: Buffer,
    pub ao_gen_params: Buffer,
    pub ao_application_params: Buffer,
}
#[derive(Resource)]
pub struct HBAOPipelines {
    pub linear_depth_pipeline: CachedComputePipelineId,
    pub ao_gen_pipeline: CachedComputePipelineId,
    pub blur_pipeline: CachedComputePipelineId,
    pub application_pipeline: CachedRenderPipelineId,

    pub common_bind_group_layout: BindGroupLayout,

    pub linear_depth_bind_group_layout: BindGroupLayout,
    pub ao_gen_bind_group_layout: BindGroupLayout,
    pub blur_bind_group_layout: BindGroupLayout,
    pub application_bind_group_layout: BindGroupLayout,

    pub point_clamp_sampler: Sampler,
    pub linear_clamp_sampler: Sampler,
    pub linear_wrap_sampler: Sampler,
}

const HBAO_LINEAR_DEPTH_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(1874948457211004181);
const HBAO_AO_GEN_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(1874948457211004183);
const HBAO_BLUR_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(1874948457211004184);
const HBAO_APPLY_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(1874948457211004185);

#[derive(RenderLabel, Debug, Clone, Hash, PartialEq, Eq)]
struct HBAOLinearDepthPrepassLabel;
#[derive(RenderLabel, Debug, Clone, Hash, PartialEq, Eq)]
struct HBAOAoGenPrepassLabel;
#[derive(RenderLabel, Debug, Clone, Hash, PartialEq, Eq)]
struct HBAOBlurPrepassLabel;
#[derive(RenderLabel, Debug, Clone, Hash, PartialEq, Eq)]
struct HBAOApplicationLabel;

/// Plugins

pub struct HBAOPlugin;

impl Plugin for HBAOPlugin {
    fn build(&self, app: &mut App) {
        info!("Building HBAOPlugin");

        // Load the shaders as internal assets
        load_internal_asset!(
            app,
            HBAO_LINEAR_DEPTH_SHADER_HANDLE,
            "shaders/linear_depth_compute.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            HBAO_AO_GEN_SHADER_HANDLE,
            "shaders/ao_gen_compute.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            HBAO_BLUR_SHADER_HANDLE,
            "shaders/blur_compute.wgsl",
            Shader::from_wgsl
        );

        load_internal_asset!(
            app,
            HBAO_APPLY_SHADER_HANDLE,
            "shaders/apply_ao_frag.wgsl",
            Shader::from_wgsl
        );
        app.add_plugins(ExtractResourcePlugin::<LightingSettings>::default());
    }
    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            error!("RenderApp not found");
            return;
        };
        render_app
            .init_resource::<HBAOPipelines>()
            .add_systems(
                bevy::render::Render,
                (
                    prepare_pipeline_textures.in_set(RenderSet::PrepareResources),
                    prepare_pipeline_bind_groups.in_set(RenderSet::PrepareBindGroups),
                ),
            )
            .add_render_graph_node::<ViewNodeRunner<HBAOLinearDepthPrepassNode>>(
                Core3d,
                HBAOLinearDepthPrepassLabel,
            )
            .add_render_graph_node::<ViewNodeRunner<HBAOAoGenPrepassNode>>(
                Core3d,
                HBAOAoGenPrepassLabel,
            )
            .add_render_graph_node::<ViewNodeRunner<HBAOBlurPrepassNode>>(
                Core3d,
                HBAOBlurPrepassLabel,
            )
            .add_render_graph_node::<ViewNodeRunner<HBAOApplicationNode>>(
                Core3d,
                HBAOApplicationLabel,
            )
            .add_render_graph_edges(
                Core3d,
                (
                    Node3d::EndPrepasses,
                    HBAOLinearDepthPrepassLabel,
                    HBAOAoGenPrepassLabel,
                    HBAOBlurPrepassLabel,
                    Node3d::StartMainPass,
                    Node3d::EndMainPass,
                    HBAOApplicationLabel,
                ),
            );
    }
}

///Pipeline
///

impl FromWorld for HBAOPipelines {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        println!("Creating HBAOPipelines");

        // Point/Nearest sampling with clamp to edge (for depth and normal textures)
        let point_clamp_sampler = render_device.create_sampler(&SamplerDescriptor {
            min_filter: FilterMode::Nearest,
            mag_filter: FilterMode::Nearest,
            mipmap_filter: FilterMode::Nearest,
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            ..Default::default()
        });

        // Linear sampling with clamp to edge (for filtered textures like AO and random rotation)
        let linear_clamp_sampler = render_device.create_sampler(&SamplerDescriptor {
            min_filter: FilterMode::Linear,
            mag_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Nearest,
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            ..Default::default()
        });

        // Linear sampling with repeat/wrap (specifically for random rotation/noise textures)
        let linear_wrap_sampler = render_device.create_sampler(&SamplerDescriptor {
            min_filter: FilterMode::Linear,
            mag_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Nearest,
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            ..Default::default()
        });

        let common_bind_group_layout = render_device.create_bind_group_layout(
            "hbao_common_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                (
                    sampler(SamplerBindingType::NonFiltering),
                    sampler(SamplerBindingType::Filtering),
                    sampler(SamplerBindingType::Filtering),
                    uniform_buffer::<ViewUniform>(true),
                ),
            ),
        );

        let linear_depth_group_layout = render_device.create_bind_group_layout(
            "linear_depth_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    texture_depth_2d(),
                    texture_storage_2d(TextureFormat::R32Float, StorageTextureAccess::WriteOnly),
                ),
            ),
        );

        let ao_gen_group_layout = render_device.create_bind_group_layout(
            "ao_gen_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    texture_2d(TextureSampleType::Float { filterable: false }),
                    texture_2d(TextureSampleType::Float { filterable: false }),
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    texture_storage_2d(TextureFormat::R32Float, StorageTextureAccess::WriteOnly),
                    uniform_buffer::<AOGenParams>(false),
                ),
            ),
        );

        let blur_group_layout = render_device.create_bind_group_layout(
            "blur_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    texture_2d(TextureSampleType::Float { filterable: false }),
                    texture_2d(TextureSampleType::Float { filterable: false }),
                    texture_storage_2d(TextureFormat::R32Uint, StorageTextureAccess::WriteOnly),
                    uniform_buffer::<BlurParams>(false),
                ),
            ),
        );

        let application_group_layout = render_device.create_bind_group_layout(
            "application_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    uniform_buffer::<AOApplicationParams>(false),
                    texture_2d(TextureSampleType::Uint),
                    texture_2d(TextureSampleType::Float { filterable: false }),
                ),
            ),
        );
        //Start Creating pipelines

        let pipeline_cache = world.resource::<PipelineCache>();

        let linear_depth_pipeline =
            pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
                label: Some("linear_depth_compute_pipeline".into()),
                layout: vec![
                    linear_depth_group_layout.clone(),
                    common_bind_group_layout.clone(),
                ],
                push_constant_ranges: vec![],
                shader: HBAO_LINEAR_DEPTH_SHADER_HANDLE,
                shader_defs: Vec::new(),
                entry_point: "linearize_depth_main".into(),
                zero_initialize_workgroup_memory: false,
            });

        let ao_gen_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("ao_gen_compute_pipeline".into()),
            layout: vec![
                ao_gen_group_layout.clone(),
                common_bind_group_layout.clone(),
            ],
            push_constant_ranges: vec![],
            shader: HBAO_AO_GEN_SHADER_HANDLE,
            shader_defs: Vec::new(),
            entry_point: "ao_gen_main".into(),
            zero_initialize_workgroup_memory: false,
        });

        let blur_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("blur_compute_pipeline".into()),
            layout: vec![blur_group_layout.clone(), common_bind_group_layout.clone()],
            push_constant_ranges: vec![],
            shader: HBAO_BLUR_SHADER_HANDLE,
            shader_defs: Vec::new(),
            entry_point: "blur_combined".into(),
            zero_initialize_workgroup_memory: false,
        });

        let ao_apply_pipeline = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("ao_apply_frag_pipeline".into()),
            layout: vec![
                application_group_layout.clone(),
                common_bind_group_layout.clone(),
            ],

            // Fragment shader configuration
            vertex: fullscreen_shader_vertex_state(), // Use fullscreen vertex shader
            fragment: Some(FragmentState {
                shader: HBAO_APPLY_SHADER_HANDLE, // Use the apply shader handle, not linear depth
                shader_defs: vec![],
                entry_point: "apply_ao_fragment".into(), // Match the entry point in your shader
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::Rgba8UnormSrgb, // Choose appropriate format for your needs
                    blend: None,                           // Or specify a blend state if needed
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            push_constant_ranges: Vec::new(),
            zero_initialize_workgroup_memory: false,
        });
        Self {
            linear_depth_pipeline,
            ao_gen_pipeline,
            blur_pipeline,
            application_pipeline: ao_apply_pipeline,

            common_bind_group_layout,

            linear_depth_bind_group_layout: linear_depth_group_layout,
            ao_gen_bind_group_layout: ao_gen_group_layout,
            blur_bind_group_layout: blur_group_layout,
            application_bind_group_layout: application_group_layout,

            point_clamp_sampler,
            linear_clamp_sampler,
            linear_wrap_sampler,
        }
    }
}
fn prepare_pipeline_textures(
    mut commands: Commands,
    mut texture_cache: ResMut<TextureCache>,
    lighting_settings: Res<LightingSettings>,
    render_device: Res<RenderDevice>,
    views: Query<(Entity, &ExtractedCamera), Without<HBAOSharedPipelineResources>>,
) {
    for (entity, camera) in &views {
        let Some(physical_viewport_size) = camera.physical_viewport_size else {
            println!("physical_viewport_size is None");
            continue;
        };
        let size = Extent3d {
            width: physical_viewport_size.x,
            height: physical_viewport_size.y,
            depth_or_array_layers: 1,
        };

        let hbao_linear_depth_texture = texture_cache.get(
            &render_device,
            TextureDescriptor {
                label: Some("hbao_linear_depth_texture"),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::R32Float,
                usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
        );
        let hbao_normal_texture = texture_cache.get(
            &render_device,
            TextureDescriptor {
                label: Some("hbao_normal_texture"),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::R32Float,
                usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
        );
        let hbao_random_rotation_texture = texture_cache.get(
            &render_device,
            TextureDescriptor {
                label: Some("hbao_random_rotation_texture"),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::R32Float,
                usage: TextureUsages::COPY_DST
                    | TextureUsages::STORAGE_BINDING
                    | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
        );
        let hbao_raw_ao_texture = texture_cache.get(
            &render_device,
            TextureDescriptor {
                label: Some("hbao_raw_ao_texture"),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::R32Float,
                usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
        );
        let hbao_blur_ao_texture = texture_cache.get(
            &render_device,
            TextureDescriptor {
                label: Some("hbao_blur_ao_texture"),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::R32Uint,
                usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
        );
        // Create the buffer for AOGenParams
        let ao_gen_params_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("ao_gen_params_buffer"),
            contents: bytemuck::cast_slice(&[lighting_settings.ao_gen_params]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        // Create the buffer for BlurParams
        let blur_params_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("blur_params_buffer"),
            contents: bytemuck::cast_slice(&[lighting_settings.blur_params]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        let params_clone = lighting_settings.ao_application_params;
        let bytes = bytemuck::bytes_of(&params_clone).to_vec();

        // Create the buffer for AOApplicationParams
        let ao_application_params_buffer =
            render_device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("ao_application_params_buffer"),
                contents: &bytes,
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            });

        commands.entity(entity).insert(HBAOSharedPipelineResources {
            linear_depth_texture: hbao_linear_depth_texture,
            normal_texture: hbao_normal_texture,
            random_rotation_texture: hbao_random_rotation_texture,
            raw_ao_texture: hbao_raw_ao_texture,
            blur_ao_texture: hbao_blur_ao_texture,

            ao_gen_params: ao_gen_params_buffer,
            blur_params: blur_params_buffer,
            ao_application_params: ao_application_params_buffer,
        });
    }
}
fn prepare_pipeline_bind_groups(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    pipelines: Res<HBAOPipelines>,
    view_uniforms: Res<ViewUniforms>,
    views: Query<(
        Entity,
        &HBAOSharedPipelineResources,
        &ViewTarget,
        &ViewPrepassTextures, // Add this
    )>,
) {
    let Some(view_uniforms) = view_uniforms.uniforms.binding() else {
        println!("view_uniforms is None");
        return;
    };

    for (entity, hbao_resources, view_target, prepass_textures) in &views {
        // Create common bind group (used by multiple passes)
        let common_bind_group = render_device.create_bind_group(
            "hbao_common_bind_group",
            &pipelines.common_bind_group_layout,
            &BindGroupEntries::sequential((
                &pipelines.point_clamp_sampler,
                &pipelines.linear_clamp_sampler,
                &pipelines.linear_wrap_sampler,
                view_uniforms.clone(),
            )),
        );

        // Linear depth pass bind group
        let linear_depth_bind_group = render_device.create_bind_group(
            "hbao_linear_depth_bind_group",
            &pipelines.linear_depth_bind_group_layout,
            &BindGroupEntries::sequential((
                prepass_textures.depth_view().unwrap(),
                &hbao_resources.linear_depth_texture.default_view,
            )),
        );

        // AO generation bind group
        let ao_gen_bind_group = render_device.create_bind_group(
            "hbao_ao_gen_bind_group",
            &pipelines.ao_gen_bind_group_layout,
            &BindGroupEntries::sequential((
                &hbao_resources.linear_depth_texture.default_view,
                prepass_textures.normal_view().unwrap(),
                &hbao_resources.random_rotation_texture.default_view,
                &hbao_resources.raw_ao_texture.default_view,
                hbao_resources.ao_gen_params.as_entire_binding(),
            )),
        );

        // Blur pass bind group
        let blur_bind_group = render_device.create_bind_group(
            "hbao_blur_bind_group",
            &pipelines.blur_bind_group_layout,
            &BindGroupEntries::sequential((
                &hbao_resources.raw_ao_texture.default_view,
                &hbao_resources.linear_depth_texture.default_view,
                prepass_textures.normal_view().unwrap(),
                &hbao_resources.blur_ao_texture.default_view,
                hbao_resources.blur_params.as_entire_binding(),
            )),
        );

        // Application pass bind group
        let application_bind_group = render_device.create_bind_group(
            "hbao_application_bind_group",
            &pipelines.application_bind_group_layout,
            &BindGroupEntries::sequential((
                hbao_resources.ao_application_params.as_entire_binding(),
                &hbao_resources.blur_ao_texture.default_view,
                &hbao_resources.linear_depth_texture.default_view,
            )),
        );
        use rand::Rng;
        let size = view_target.main_texture().size();

        let mut rng = rand::rng();
        let mut random_data = vec![0.0f32; (size.width * size.height) as usize];
        for value in random_data.iter_mut() {
            *value = rng.random_range(0.0..1.0);
        }
        // Insert all bind groups as a component on the entity
        commands.entity(entity).insert(HBAOBindGroups {
            common_bind_group,
            linear_depth_bind_group,
            ao_gen_bind_group,
            blur_bind_group,
            application_bind_group,

            random_data,
        });
    }
}
