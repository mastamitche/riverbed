use bevy::{
    prelude::*,
    render::{
        camera::ExtractedCamera,
        render_graph::{NodeRunError, RenderGraphContext, ViewNode},
        render_resource::{
            LoadOp, Operations, PipelineCache, RenderPassColorAttachment, RenderPassDescriptor,
            StoreOp,
        },
        renderer::RenderContext,
        view::{ViewTarget, ViewUniformOffset},
    },
};

use super::hbao::{HBAOBindGroups, HBAOPipelines};

#[derive(Default)]
pub struct HBAOApplicationNode;

impl ViewNode for HBAOApplicationNode {
    type ViewQuery = (
        &'static ExtractedCamera,
        &'static ViewTarget,
        &'static HBAOBindGroups,
        &'static ViewUniformOffset,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (camera, view_target, bind_groups, view_uniform_offset): bevy::ecs::query::QueryItem<
            Self::ViewQuery,
        >,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipelines = world.resource::<HBAOPipelines>();
        let (Some(camera_size), Some(application_pipeline)) = (
            camera.physical_viewport_size,
            pipeline_cache.get_render_pipeline(pipelines.application_pipeline),
        ) else {
            // println!("Skipping application node");
            return Ok(());
        };

        // Start the application process
        render_context
            .command_encoder()
            .push_debug_group("HBAO Application");

        // {
        //     let pass_descriptor = RenderPassDescriptor {
        //         label: Some("HBAO Application pass"),
        //         color_attachments: &[Some(RenderPassColorAttachment {
        //             view: view_target.main_texture_view(),
        //             resolve_target: None,
        //             ops: Operations {
        //                 load: LoadOp::Load, // Important: Load existing content, don't clear
        //                 store: StoreOp::Store,
        //             },
        //         })],
        //         depth_stencil_attachment: None, // No need for depth/stencil in post-processing
        //         timestamp_writes: None,
        //         occlusion_query_set: None,
        //     };

        //     let mut render_pass = render_context
        //         .command_encoder()
        //         .begin_render_pass(&pass_descriptor);

        //     render_pass.set_pipeline(application_pipeline);
        //     render_pass.set_bind_group(0, &bind_groups.application_bind_group, &[]);
        //     render_pass.set_bind_group(
        //         1,
        //         &bind_groups.common_bind_group,
        //         &[view_uniform_offset.offset],
        //     );

        //     // Only needed if you're using stencil testing to selectively apply AO
        //     // In most post-process cases, this isn't necessary
        //     // render_pass.set_stencil_reference(1);

        //     // Draw a full-screen triangle/quad
        //     render_pass.draw(0..3, 0..1);
        // }

        render_context.command_encoder().pop_debug_group();

        Ok(())
    }
}
