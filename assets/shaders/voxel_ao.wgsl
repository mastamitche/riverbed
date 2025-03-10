#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    mesh_view_bindings::{view, globals},
    pbr_types::{STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT, STANDARD_MATERIAL_FLAGS_ALPHA_MODE_BLEND, PbrInput, pbr_input_new,StandardMaterial,STANDARD_MATERIAL_FLAGS_ALPHA_MODE_MASK },
    pbr_functions::{ alpha_discard},
    mesh_functions::{get_world_from_local,mesh_normal_local_to_world, mesh_position_local_to_clip, mesh_position_local_to_world},
}
#import bevy_core_pipeline::tonemapping::tone_mapping

#ifdef PREPASS_PIPELINE
#import bevy_pbr::{
    prepass_io::{VertexOutput, FragmentOutput},
    pbr_deferred_functions::deferred_output,
}
#else
#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
}
#endif

@group(2) @binding(100) var ao_none_texture: texture_2d<f32>;
@group(2) @binding(101) var ao_none_sampler: sampler;
@group(2) @binding(102) var ao_one_corner_texture: texture_2d<f32>;
@group(2) @binding(103) var ao_one_corner_sampler: sampler;
@group(2) @binding(104) var ao_two_corners_texture: texture_2d<f32>;
@group(2) @binding(105) var ao_two_corners_sampler: sampler;
@group(2) @binding(106) var ao_two_opposite_corners_texture: texture_2d<f32>;
@group(2) @binding(107) var ao_two_opposite_corners_sampler: sampler;
@group(2) @binding(108) var ao_three_corners_texture: texture_2d<f32>;
@group(2) @binding(109) var ao_three_corners_sampler: sampler;
@group(2) @binding(110) var ao_four_corners_texture: texture_2d<f32>;
@group(2) @binding(111) var ao_four_corners_sampler: sampler;
@group(2) @binding(112) var ao_one_edge_texture: texture_2d<f32>;
@group(2) @binding(113) var ao_one_edge_sampler: sampler;
@group(2) @binding(114) var ao_opposite_edges_texture: texture_2d<f32>;
@group(2) @binding(115) var ao_opposite_edges_sampler: sampler;
@group(2) @binding(116) var ao_two_adjacent_edges_texture: texture_2d<f32>;
@group(2) @binding(117) var ao_two_adjacent_edges_sampler: sampler;
@group(2) @binding(118) var ao_three_edges_texture: texture_2d<f32>;
@group(2) @binding(119) var ao_three_edges_sampler: sampler;
@group(2) @binding(120) var ao_four_edges_texture: texture_2d<f32>;
@group(2) @binding(121) var ao_four_edges_sampler: sampler;

// Helper function to rotate UVs based on rotation index
fn rotate_uv(uv: vec2<f32>, rotation: f32) -> vec2<f32> {
    let center = vec2<f32>(0.5, 0.5);
    let uv_centered = uv - center;
    
    var rotated_uv: vec2<f32>;
    
    // Apply rotation based on rotation index (0-3)
    if (rotation < 0.5) {
        // 0 degrees - no rotation
        rotated_uv = uv_centered;
    } else if (rotation < 1.5) {
        // 90 degrees
        rotated_uv = vec2<f32>(-uv_centered.y, uv_centered.x);
    } else if (rotation < 2.5) {
        // 180 degrees
        rotated_uv = vec2<f32>(-uv_centered.x, -uv_centered.y);
    } else {
        // 270 degrees
        rotated_uv = vec2<f32>(uv_centered.y, -uv_centered.x);
    }
    
    return rotated_uv + center;
}

// Sample the appropriate AO texture based on pattern
fn sample_ao_texture(uv: vec2<f32>, pattern: f32, rotation: f32) -> f32 {
    let rotated_uv = rotate_uv(uv, rotation);
    
    if (pattern < 0.5) {
        return textureSample(ao_none_texture, ao_none_sampler, rotated_uv).r;
    } else if (pattern < 1.5) {
        return textureSample(ao_one_corner_texture, ao_one_corner_sampler, rotated_uv).r;
    } else if (pattern < 2.5) {
        return textureSample(ao_two_corners_texture, ao_two_corners_sampler, rotated_uv).r;
    } else if (pattern < 3.5) {
        return textureSample(ao_two_opposite_corners_texture, ao_two_opposite_corners_sampler, rotated_uv).r;
    } else if (pattern < 4.5) {
        return textureSample(ao_three_corners_texture, ao_three_corners_sampler, rotated_uv).r;
    } else if (pattern < 5.5) {
        return textureSample(ao_four_corners_texture, ao_four_corners_sampler, rotated_uv).r;
    } else if (pattern < 6.5) {
        return textureSample(ao_one_edge_texture, ao_one_edge_sampler, rotated_uv).r;
    } else if (pattern < 7.5) {
        return textureSample(ao_opposite_edges_texture, ao_opposite_edges_sampler, rotated_uv).r;
    } else if (pattern < 8.5) {
        return textureSample(ao_two_adjacent_edges_texture, ao_two_adjacent_edges_sampler, rotated_uv).r;
    } else if (pattern < 9.5) {
        return textureSample(ao_three_edges_texture, ao_three_edges_sampler, rotated_uv).r;
    } else {
        return textureSample(ao_four_edges_texture, ao_four_edges_sampler, rotated_uv).r;
    }
}
struct CustomVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec4<f32>,
    @location(4) instance_index: u32,
    @location(5) ao_data: f32,
}

@vertex
fn vertex(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(5) color: vec4<f32>,
    @location(6) ao_data: f32,
    @builtin(instance_index) instance_index: u32,
) -> CustomVertexOutput {
    // Use Bevy's standard vertex output generation
    let model = get_world_from_local(instance_index);
    let world_position = mesh_position_local_to_world(model, vec4(position, 1.0));
    let clip_position = mesh_position_local_to_clip(model, vec4(position, 1.0));
    let world_normal = mesh_normal_local_to_world(normal, instance_index);
    
    // Create standard vertex output
    var out: CustomVertexOutput;
    out.position = clip_position;
    out.world_position = world_position;
    out.world_normal = world_normal;
    out.uv = uv;
    out.color = color;
    out.instance_index = instance_index;
    out.ao_data = ao_data;
    
    return out;
}

@fragment
fn fragment(
    in: CustomVertexOutput,
    @builtin(front_facing) is_front: bool,
) -> @location(0) vec4<f32> {
    // Retrieve our AO data from where we stored it
    let ao_data = in.ao_data;
    
    // Extract pattern and rotation from the ao_data
    // Assuming ao_data format: pattern * 10 + rotation
    let pattern = floor(ao_data / 10.0);
    let rotation = ao_data - (pattern * 10.0);
    
    // For debugging, return flat red to confirm the shader is working
    return vec4(1.0, 0.0, 0.0, 1.0);
    
    // Once the flat red is working, uncomment this section:
    /*
    var vert: VertexOutput;
    vert.position = in.position;
    vert.world_position = in.world_position;
    vert.world_normal = in.world_normal;
    vert.uv = in.uv;
    vert.color = in.color;
    vert.instance_index = in.instance_index;

    // Generate a PbrInput struct from the standard material
    var pbr_input = pbr_input_from_standard_material(vert, is_front);
    
    // Sample the appropriate AO texture
    let ao_factor = sample_ao_texture(in.uv, pattern, rotation);
    
    // Apply AO to the base color and handle alpha
    pbr_input.material.base_color = alpha_discard(pbr_input.material, vec4(pbr_input.material.base_color.rgb * ao_factor, pbr_input.material.base_color.a));
    
    // Apply PBR lighting
    var output_color = apply_pbr_lighting(pbr_input);
    
    // Apply any post-processing
    output_color = main_pass_post_lighting_processing(pbr_input, output_color);
    
    return output_color;
    */
}
