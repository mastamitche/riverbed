use bevy::{
    ecs::query::QueryItem,
    prelude::*,
    render::{
        camera::ExtractedCamera,
        render_graph::{NodeRunError, RenderGraphContext, ViewNode},
        render_resource::{
            BufferInitDescriptor, BufferUsages, ComputePassDescriptor, Extent3d, ImageCopyBuffer,
            ImageCopyTexture, ImageDataLayout, Origin3d, PipelineCache, TextureAspect,
        },
        renderer::RenderContext,
        view::ViewUniformOffset,
    },
};

use super::hbao::{HBAOBindGroups, HBAOPipelines, HBAOSharedPipelineResources};

#[derive(Default)]
pub struct HBAOAoGenPrepassNode;

impl ViewNode for HBAOAoGenPrepassNode {
    type ViewQuery = (
        &'static ExtractedCamera,
        &'static HBAOBindGroups,
        &'static ViewUniformOffset,
        &'static HBAOSharedPipelineResources,
    );
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (camera, bind_groups, view_uniform_offset, pipeline_resources): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipelines = world.resource::<HBAOPipelines>();
        let (Some(camera_size), Some(ao_gen_pipeline)) = (
            camera.physical_viewport_size,
            pipeline_cache.get_compute_pipeline(pipelines.ao_gen_pipeline),
        ) else {
            // println!("Skipping prepass node");
            return Ok(());
        };

        // Start the prepass process
        render_context
            .command_encoder()
            .push_debug_group("HBAO Ao Gen Prepass");
        let render_device = render_context.render_device();
        let random_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("random_rotation_buffer"),
            contents: bytemuck::cast_slice(&bind_groups.random_data),
            usage: BufferUsages::COPY_SRC,
        });

        let size = Extent3d {
            width: camera_size.x,
            height: camera_size.y,
            depth_or_array_layers: 1,
        };

        render_context.command_encoder().copy_buffer_to_texture(
            ImageCopyBuffer {
                buffer: &random_buffer,
                layout: ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * size.width),
                    rows_per_image: None,
                },
            },
            ImageCopyTexture {
                texture: &pipeline_resources.random_rotation_texture.texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            size,
        );
        // 2. AO Gen Pass
        {
            let mut preprocess_depth_pass =
                render_context
                    .command_encoder()
                    .begin_compute_pass(&ComputePassDescriptor {
                        label: Some("hbao_ao_gen_pass"),
                        timestamp_writes: None,
                    });
            preprocess_depth_pass.set_pipeline(&ao_gen_pipeline);
            preprocess_depth_pass.set_bind_group(0, &bind_groups.ao_gen_bind_group, &[]);
            preprocess_depth_pass.set_bind_group(
                1,
                &bind_groups.common_bind_group,
                &[view_uniform_offset.offset],
            );
            preprocess_depth_pass.dispatch_workgroups(
                camera_size.x.div_ceil(8),
                camera_size.y.div_ceil(8),
                1,
            );
        }
        render_context.command_encoder().pop_debug_group();

        Ok(())
    }
}
