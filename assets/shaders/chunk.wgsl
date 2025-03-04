#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    mesh_view_bindings::{view, globals},
    pbr_types::{STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT, STANDARD_MATERIAL_FLAGS_ALPHA_MODE_BLEND, PbrInput, pbr_input_new},
    pbr_functions as fns,
    mesh_functions::{get_world_from_local, mesh_position_local_to_clip, mesh_position_local_to_world},
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


const MASK2: u32 = 3;
const MASK3: u32 = 7;
const MASK4: u32 = 15;
const MASK6: u32 = 63;
const MASK9: u32 = 511;
const MASK16: u32 = 65535;

struct VertexInput {
    @builtin(instance_index) instance_index: u32,
    @location(0) voxel_data: vec2<u32>,
};

struct CustomVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec4<f32>,
    @location(4) face_light: vec4<f32>,
};


fn normal_from_id(id: u32) -> vec3<f32> {
    var n: vec3<f32>;
    switch id {
        case 0u {
            n = vec3(0.0, 1.0, 0.0);
        }
        case 1u {
            n = vec3(0.0, -1.0, 0.0);
        }
        case 2u {
            n = vec3(1.0, 0.0, 0.0);
        }
        case 3u {
            n = vec3(-1.0, 0.0, 0.0);
        }
        case 4u {
            n = vec3(0.0, 0.0, 1.0);
        }
        case 5u {
            n = vec3(0.0, 0.0, -1.0);
        }
        default {
            n = vec3(0.0);
        }
    }
    return n;
}

fn light_from_id(id: u32) -> vec4<f32> {
    switch id {
        case 0u {
            return vec4(1.0, 1.0, 1.0, 1.0); // top
        }
        case 2u, 3u {
            return vec4(0.7, 0.7, 0.7, 1.0); // right left
        }
        case 4u, 5u {
            return vec4(0.5, 0.5, 0.5, 1.0); // front back
        }
        case 1u {
            return vec4(0.3, 0.3, 0.3, 1.0); // bottom
        }
        default {
            return vec4(0.0, 0.0, 0.0, 1.0);
        }
    }
}

fn color_from_id(id: u32) -> vec4<f32> {
    var r = f32(id & MASK3)/f32(MASK3);
    var g = f32((id >> 3) & MASK3)/f32(MASK3);
    var b = f32((id >> 6) & MASK3)/f32(MASK3);
    return vec4(r, g, b, 1.0);
}

@vertex
fn vertex(vertex: VertexInput) -> CustomVertexOutput {
    var out: CustomVertexOutput;

    // Vertex specific information
    var vertex_info = vertex.voxel_data.x;
    var x = f32(vertex_info & MASK6);
    var y = f32((vertex_info >> 6) & MASK6);
    var z = f32((vertex_info >> 12) & MASK6);
    var u = f32((vertex_info >> 18) & MASK6);
    var v = f32((vertex_info >> 24) & MASK6);
    var position = vec4(x, y, z, 1.0);
    
    // Quad specific information
    var quad_info = vertex.voxel_data.y;
    var n_id = quad_info & MASK3;
    var normal = normal_from_id(n_id);
    var c_id = (quad_info >> 3) & MASK9;
    var face_color = color_from_id(c_id);
    var texture_layer = quad_info >> 12;
    var face_light = light_from_id(n_id);
    var light = f32((quad_info >> 28) & MASK4) / f32(MASK4);

    out.position = mesh_position_local_to_clip(
        get_world_from_local(vertex.instance_index),
        position,
    );
    out.world_position = mesh_position_local_to_world(
        get_world_from_local(vertex.instance_index),
        position,
    );
    out.world_normal = normal;
    out.uv = vec2(u, v);
    out.color = face_color;
    out.face_light = face_light;
    return out;
}

@fragment
fn fragment(
    in: CustomVertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    var vertex_output: VertexOutput;
    vertex_output.position = in.position;
    vertex_output.world_position = in.world_position;
    vertex_output.world_normal = in.world_normal;
#ifdef VERTEX_UVS
    vertex_output.uv = in.uv;
#endif
#ifdef VERTEX_UVS_B
    vertex_output.uv_b = in.uv;
#endif
#ifdef VERTEX_COLORS
    vertex_output.color = in.color;
#endif
    // generate a PbrInput struct from the StandardMaterial bindings
    var pbr_input = pbr_input_from_standard_material(vertex_output, is_front);
    
    // sample texture
    pbr_input.material.base_color = in.color * in.face_light;
    
    // alpha discard
    pbr_input.material.base_color = fns::alpha_discard(pbr_input.material, pbr_input.material.base_color);

#ifdef PREPASS_PIPELINE
    // in deferred mode we can't modify anything after that, as lighting is run in a separate fullscreen shader.
    let out = deferred_output(in, pbr_input);
#else
    var out: FragmentOutput;
    // apply lighting
    out.color = apply_pbr_lighting(pbr_input);

    // apply in-shader post processing (fog, alpha-premultiply, and also tonemapping, debanding if the camera is non-hdr)
    // note this does not include fullscreen postprocessing effects like bloom.
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
#endif

    return out;
}