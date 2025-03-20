#import bevy_render::view::View

@group(0) @binding(0) var input_depth: texture_depth_2d;
@group(0) @binding(1) var output_linear_depth: texture_storage_2d<r32float, write>;
@group(1) @binding(0) var non_filtering_sampler: sampler;
@group(1) @binding(1) var filtering_clamp_sampler: sampler;
@group(1) @binding(2) var filtering_wrap_sampler: sampler;
@group(1) @binding(3) var<uniform> view: View;

fn linearize_depth(depth: f32) -> f32 {
    let clip_from_view = view.clip_from_view;
    let z_near = -clip_from_view[3][2] / (clip_from_view[2][2] - 1.0);
    let z_far = -clip_from_view[3][2] / (clip_from_view[2][2] + 1.0);
    
    // Increase sensitivity to small depth changes
    return pow(z_near / (z_near + depth * (z_far - z_near)), 1.5);
}
@compute @workgroup_size(8, 8, 1)
fn linearize_depth_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let dims = textureDimensions(input_depth);
    
    if (global_id.x >= dims.x || global_id.y >= dims.y) {
        return;
    }
    
    // Simple direct sampling (no textureGather)
    let depth = textureLoad(input_depth, vec2<i32>(global_id.xy), 0);
    let linear_depth = linearize_depth(depth);
    
    // Store the linearized depth
    textureStore(output_linear_depth, vec2<i32>(global_id.xy), vec4<f32>((linear_depth - 1.0) * 100.0, 0.0, 0.0, 0.0));
}