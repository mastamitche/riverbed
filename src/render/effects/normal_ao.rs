use bevy::{
    asset::load_internal_asset, core_pipeline::{
        core_3d::graph::{Core3d, Node3d},
        fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    }, prelude::*, render::{
        camera::ExtractedCamera, extract_component::{ExtractComponent, ExtractComponentPlugin}, render_graph::{
            NodeRunError, RenderGraphApp, RenderGraphContext, RenderLabel, ViewNode, ViewNodeRunner,
        }, render_resource::{
            BindGroupEntry, BindGroupLayout, BindingResource, BindingType, CachedRenderPipelineId, 
            ColorTargetState, ColorWrites, FragmentState, MultisampleState, Operations, PipelineCache, 
            PrimitiveState, RenderPassColorAttachment, RenderPassDescriptor, RenderPipelineDescriptor, 
            Sampler, SamplerBindingType, SamplerDescriptor, ShaderStages, ShaderType, TextureFormat, 
            TextureSampleType, TextureViewDimension, Shader
        }, renderer::{RenderContext, RenderDevice}, view::{ViewTarget, ViewUniformOffset, ViewUniforms}, RenderApp
    }
};
use bytemuck::{Pod, Zeroable};

#[derive(RenderLabel, Debug, Clone, Hash, PartialEq, Eq)]
struct NormalAoLabel;

const AO_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(1874948457211004181);
pub struct NormalAoPlugin;

impl Plugin for NormalAoPlugin {
    fn build(&self, app: &mut App) {
        info!("Building NormalAoPlugin");
        
        // Load the shader as an internal asset
        load_internal_asset!(
            app,
            AO_SHADER_HANDLE,
            "shaders/normal_ao.wgsl",
            Shader::from_wgsl
        );
        
        app.add_plugins(ExtractComponentPlugin::<NormalAoSettings>::default());

        info!("Adding render graph node");
        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .add_render_graph_node::<ViewNodeRunner<NormalAoNode>>(
                Core3d,
                NormalAoLabel,
            )
            .add_render_graph_edges(
                Core3d,
                (
                     Node3d::EndMainPass,
                    NormalAoLabel,                    
                     Node3d::Taa,
                )            
            );
    }

    fn finish(&self, app: &mut App) {
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.init_resource::<NormalAoPipeline>();
            info!("Initialized normal_ao pipeline");
        }
    }
}


// This component allows configuring the effect
#[derive(Component, Clone, ExtractComponent)]
pub struct NormalAoSettings {
    pub intensity: f32,
    pub radius: f32,
    pub enabled: bool,
}

impl Default for NormalAoSettings {
    fn default() -> Self {
        Self {
            intensity: 1.0,
            radius: 0.1,
            enabled: true,
        }
    }
}

// Resource to hold our pipeline
#[derive(Resource)]
struct NormalAoPipeline {
    pipeline: CachedRenderPipelineId,
    bind_group_layout: BindGroupLayout,
    sampler: Sampler,
}

impl FromWorld for NormalAoPipeline {
    fn from_world(world: &mut World) -> Self {
        info!("Creating normal_ao pipeline");   
        let render_device = world.resource::<RenderDevice>();
        
        // Create the sampler
        let sampler = render_device.create_sampler(&SamplerDescriptor {
            address_mode_u: bevy::render::render_resource::AddressMode::ClampToEdge,
            address_mode_v: bevy::render::render_resource::AddressMode::ClampToEdge,
            address_mode_w: bevy::render::render_resource::AddressMode::ClampToEdge,
            mag_filter: bevy::render::render_resource::FilterMode::Linear,
            min_filter: bevy::render::render_resource::FilterMode::Linear,
            mipmap_filter: bevy::render::render_resource::FilterMode::Linear,
            ..Default::default()
        });
        
        // Create the bind group layout
        let bind_group_layout = render_device.create_bind_group_layout(
            "normal_ao_bind_group_layout",
            &[
                // View uniforms
                bevy::render::render_resource::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: bevy::render::render_resource::BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Input texture (color buffer)
                bevy::render::render_resource::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // Normal texture
                bevy::render::render_resource::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // Sampler
                bevy::render::render_resource::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
                // Settings uniform
                bevy::render::render_resource::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: bevy::render::render_resource::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ]
        );
        
        // Create the pipeline
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("normal_ao_pipeline".into()),
            layout: vec![bind_group_layout.clone()],
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: AO_SHADER_HANDLE,
                shader_defs: vec![],
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::Rgba8UnormSrgb, // Use a specific format instead of bevy_default
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            push_constant_ranges: vec![],
            zero_initialize_workgroup_memory: true,
        });
        
        Self {
            pipeline,
            bind_group_layout,
            sampler,
        }
    }
}

// The render node that will run our shader
#[derive(Default)]
struct NormalAoNode;

// Uniform struct for the shader
#[derive(ShaderType, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct NormalAoSettingsUniform {
    intensity: f32,
    radius: f32,
    _padding: [f32; 2],
}

impl ViewNode for NormalAoNode {
    type ViewQuery = (
        &'static ExtractedCamera,
        &'static ViewTarget,
        &'static ViewUniformOffset,
        &'static NormalAoSettings,
    );

    fn run<'w>(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (_, view_target, view_uniform_offset, normal_ao_settings): bevy::ecs::query::QueryItem<'w, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        // Skip if disabled
        if !normal_ao_settings.enabled {
            return Ok(());
        }
        
        
        // Get our pipeline resource
        let normal_ao_pipeline = world.resource::<NormalAoPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let view_uniforms = world.resource::<ViewUniforms>();
        
        // Get the post-process input texture view
        let source_view = view_target.main_texture_view();
        
        // Get the normal texture view from the G-buffer
        // For now, just use the main texture as we don't have access to G-buffer
        let normal_texture_view = source_view;
        
        // Create the settings uniform
        let settings_buffer = render_context.render_device().create_buffer_with_data(
            &bevy::render::render_resource::BufferInitDescriptor {
                label: Some("normal_ao_settings_buffer"),
                contents: bytemuck::cast_slice(&[NormalAoSettingsUniform {
                    intensity: normal_ao_settings.intensity,
                    radius: normal_ao_settings.radius,
                    _padding: [0.0, 0.0],
                }]),
                usage: bevy::render::render_resource::BufferUsages::UNIFORM,
            },
        );
        
        // Create the bind group
        let bind_group = render_context.render_device().create_bind_group(
            "normal_ao_bind_group",
            &normal_ao_pipeline.bind_group_layout,
            &[
                BindGroupEntry {
                    binding: 0,
                    resource: view_uniforms.uniforms.binding().unwrap(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(source_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(normal_texture_view),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::Sampler(&normal_ao_pipeline.sampler),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: settings_buffer.as_entire_binding(),
                },
            ],
        );
        
        // Start a post-process write
        let post_process = view_target.post_process_write();
        
        // Begin the render pass
        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("normal_ao_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: post_process.destination,
                resolve_target: None,
                ops: Operations::default(),
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        
        // Set the pipeline and bind group
        if let Some(pipeline) = pipeline_cache.get_render_pipeline(normal_ao_pipeline.pipeline) {

            render_pass.set_render_pipeline(pipeline);
            render_pass.set_bind_group(0, &bind_group, &[view_uniform_offset.offset]);
            
            // Draw the fullscreen quad
            render_pass.draw(0..3, 0..1);
        } else {
            warn!("Normal AO pipeline not found");
        }
        
        Ok(())
    }
}
