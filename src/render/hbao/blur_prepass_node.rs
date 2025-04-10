use bevy::{
    ecs::query::QueryItem,
    prelude::*,
    render::{
        camera::ExtractedCamera,
        render_graph::{NodeRunError, RenderGraphContext, ViewNode},
        render_resource::PipelineCache,
        renderer::RenderContext,
        view::ViewUniformOffset,
    },
};

use super::hbao::{HBAOBindGroups, HBAOPipelines};

#[derive(Default)]
pub struct HBAOBlurPrepassNode;

impl ViewNode for HBAOBlurPrepassNode {
    type ViewQuery = (
        &'static ExtractedCamera,
        &'static HBAOBindGroups,
        &'static ViewUniformOffset,
    );
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (camera, _, _): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipelines = world.resource::<HBAOPipelines>();
        let (Some(_), Some(_)) = (
            camera.physical_viewport_size,
            pipeline_cache.get_compute_pipeline(pipelines.blur_pipeline),
        ) else {
            // println!("Skipping prepass node");
            return Ok(());
        };

        // Start the prepass process
        render_context
            .command_encoder()
            .push_debug_group("HBAO Blur Prepass");

        // 2. Blur Pass
        // {
        //     let mut preprocess_depth_pass =
        //         render_context
        //             .command_encoder()
        //             .begin_compute_pass(&ComputePassDescriptor {
        //                 label: Some("hbao_blur_pass"),
        //                 timestamp_writes: None,
        //             });
        //     preprocess_depth_pass.set_pipeline(&blur_pipeline);
        //     preprocess_depth_pass.set_bind_group(0, &bind_groups.blur_bind_group, &[]);
        //     preprocess_depth_pass.set_bind_group(
        //         1,
        //         &bind_groups.common_bind_group,
        //         &[view_uniform_offset.offset],
        //     );
        //     preprocess_depth_pass.dispatch_workgroups(
        //         camera_size.x.div_ceil(8),
        //         camera_size.y.div_ceil(8),
        //         1,
        //     );
        // }
        render_context.command_encoder().pop_debug_group();

        Ok(())
    }
}
