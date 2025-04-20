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

@group(2) @binding(100)
var ao_texture_data: texture_3d<u32>;
@group(2) @binding(101)
var ao_texture_sampler: sampler;

const MASK2: u32 = 3;
const MASK3: u32 = 7;
const MASK4: u32 = 15;
const MASK6: u32 = 63;
const MASK9: u32 = 511;
const MASK16: u32 = 65535;
const MASK10: u32 = 0x3FF; // Binary: 1111111111 (10 ones)

const CHUNK_SIZE_FULL: i32 = 64;
const CHUNK_SIZE_FULL_POW2: i32 = CHUNK_SIZE_FULL * CHUNK_SIZE_FULL;
const CHUNK_SIZE_FULL_POW3: i32 = CHUNK_SIZE_FULL_POW2 * CHUNK_SIZE_FULL;
const CHUNK_SIZE: i32 = 62;
const CHUNK_SIZE_M_1: i32 = 61;
const EPSILON: f32 = 0.001;

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
    @location(5) wh: vec2<f32>,
    @location(6) face_normal: vec3<i32>,    
};
fn positive_modulo(a: i32, b: i32) -> i32 {
    return ((a % b) + b) % b;
}

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
fn get_face_normal(n_id: u32) -> vec3<i32> {
    switch n_id {
        case 0u: {
            return vec3<i32>(0, 1, 0);  // top
        }
        case 1u: {
            return vec3<i32>(0, -1, 0); // bottom
        }
        case 2u: {
            return vec3<i32>(1, 0, 0);  // right
        }
        case 3u: {
            return vec3<i32>(-1, 0, 0); // left
        }
        case 4u: {
            return vec3<i32>(0, 0, 1);  // front
        }
        case 5u:  {
            return vec3<i32>(0, 0, -1); // back
        }
        default: {
            return vec3<i32>(0, 0, 0);
        }
    }
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

fn affects_face(normal: vec3<i32>, offset: vec3<i32>) -> bool {
    return dot(normal, offset) <= 0;
}

fn color_from_id(id: u32) -> vec4<f32> {
    var r = f32(id & MASK3)/f32(MASK3);
    var g = f32((id >> 3) & MASK3)/f32(MASK3);
    var b = f32((id >> 6) & MASK3)/f32(MASK3);
    return vec4(r, g, b, 1.0);
}


fn get_debug_color(neighbor_count: i32) -> vec3<f32> {
    // Color scheme based on neighbor count
    switch neighbor_count {
        case 0: { return vec3<f32>(1.0, 1.0, 1.0); } // White - no neighbors
        case 1: { return vec3<f32>(1.0, 1.0, 0.0); } // Yellow - 1 neighbor
        case 2: { return vec3<f32>(0.0, 0.0, 1.0); } // Blue - 2 neighbors
        case 3: { return vec3<f32>(0.0, 1.0, 0.0); } // Green - 3 neighbors
        case 4: { return vec3<f32>(1.0, 0.0, 0.0); } // Red - 4 neighbors 
        case 5: { return vec3<f32>(1.0, 0.0, 1.0); } // Magenta - 5 neighbors
        case 6: { return vec3<f32>(0.0, 1.0, 1.0); } // Cyan - 6 neighbors
        case 7: { return vec3<f32>(0.5, 0.5, 0.5); } // Gray - 7 neighbors
        default: { return vec3<f32>(0.0, 0.0, 0.0); } // Black - 8+ neighbors
    }
}

fn count_ao_neighbors(world_pos: vec3<f32>, normal: vec3<i32>) -> i32 {
    var count = 0;
    
    var voxel_x = i32(floor(world_pos.x - (f32(normal.x)/2.0)));
    var voxel_y = i32(floor(world_pos.y - (f32(normal.y)/2.0)));
    var voxel_z = i32(floor(world_pos.z - (f32(normal.z)/2.0)));
    
    if (normal.x > 0) {
        // voxel_x +=1;
    } else if (normal.y > 0) {
        // voxel_y +=1;
    } else if (normal.z > 0) {
        // voxel_z += 1;
    }     
    if (normal.x < 0) {
        // voxel_x +=1;
    } else if (normal.y < 0) {
        // voxel_y +=1;
    } else if (normal.z < 0) {
        // voxel_z +=1;
    } 
    
    
    let chunk_x = positive_modulo(voxel_x, CHUNK_SIZE);
    let chunk_y = positive_modulo(voxel_y, CHUNK_SIZE);
    let chunk_z = positive_modulo(voxel_z, CHUNK_SIZE);

    var chunk_pos = vec3<i32>(chunk_x, chunk_y, chunk_z);
    if (normal.x > 0) {
        // chunk_pos.x +=1;
    } else if (normal.y > 0) {
        // chunk_pos.y +=1;
    } else if (normal.z > 0) {
        //chunk_pos.z +=1;
    }     
    if (normal.x < 0) {
        // chunk_pos.x +=1;
    } else if (normal.y < 0) {
        // chunk_pos.y +=1;
    } else if (normal.z < 0) {
        // chunk_pos.z +=1;
    } 


    // if (calc_pos.x != 0 && calc_pos.y != 0  && calc_pos.z != 0 ) {
    //     if (calc_pos.x != CHUNK_SIZE_M_1 && calc_pos.y != CHUNK_SIZE_M_1 && calc_pos.z != CHUNK_SIZE_M_1) {
    //         return 8;
    //     }
    // }

    // if (calc_pos.x == CHUNK_SIZE_M_1 || calc_pos.y == CHUNK_SIZE_M_1 || calc_pos.z == CHUNK_SIZE_M_1) {
    //     return 5;
    // }

    // if (calc_pos.x == 0 || calc_pos.y == 0  || calc_pos.z == 0 ) {
    //     return 8;
    // }

    if (normal.x != 0) {
        // X-axis face (right)
        let side = normal.x;
        if (check_voxel_presence(chunk_pos + vec3<i32>(side, 1, 0))) { count += 1; }  // Top
        if (check_voxel_presence(chunk_pos + vec3<i32>(side, -1, 0))) { count += 1; } // Bottom
        if (check_voxel_presence(chunk_pos + vec3<i32>(side, 0, 1))) { count += 1; }  // Front
        if (check_voxel_presence(chunk_pos + vec3<i32>(side, 0, -1))) { count += 1; } // Back
    } else if (normal.y != 0) {
        // Y-axis face (top)
        let side = normal.y ;
        if (check_voxel_presence(chunk_pos + vec3<i32>(1, side, 0))) { count += 1; }  // Right
        if (check_voxel_presence(chunk_pos + vec3<i32>(-1, side, 0))) { count += 1; } // Left
        if (check_voxel_presence(chunk_pos + vec3<i32>(0, side, 1))) { count += 1; }  // Front
        if (check_voxel_presence(chunk_pos + vec3<i32>(0, side, -1))) { count += 1; } // Back
    } else if (normal.z != 0) {
        // Z-axis face (front)
        let side = normal.z ;
        if (check_voxel_presence(chunk_pos + vec3<i32>(1, 0, side))) { count += 1; }  // Right
        if (check_voxel_presence(chunk_pos + vec3<i32>(-1, 0, side))) { count += 1; } // Left
        if (check_voxel_presence(chunk_pos + vec3<i32>(0, 1, side))) { count += 1; }  // Top
        if (check_voxel_presence(chunk_pos + vec3<i32>(0, -1, side))) { count += 1; } // Bottom
    }

    return count;
}


fn check_voxel_presence(pos: vec3<i32>) -> bool {
    var calc_pos = vec3<i32>(pos.z, pos.x, pos.y);
    calc_pos = calc_pos + vec3<i32>(1, 1, 1);
    
    // Calculate which u32 contains our bit (index / 32)
    let bit_index = calc_pos.x + calc_pos.y * CHUNK_SIZE_FULL + calc_pos.z * CHUNK_SIZE_FULL * CHUNK_SIZE_FULL;
    let u32_index = bit_index / 32;
    
    // Calculate which bit within that u32 (index % 32)
    let bit_position = bit_index % 32;
    
    // Calculate the texture coordinates to access the correct u32
    let chunk_size_packed = (CHUNK_SIZE_FULL + 31) / 32;
    let texture_x = u32_index % chunk_size_packed;
    let texture_y = (u32_index / chunk_size_packed) % CHUNK_SIZE_FULL;
    let texture_z = u32_index / (chunk_size_packed * CHUNK_SIZE_FULL);
    
    // Load the u32 value from the texture
    let packed_value = textureLoad(ao_texture_data, vec3<i32>(texture_x, texture_y, texture_z), 0).r;
    
    // Extract the correct bit
    let mask = 1u << u32(bit_position);
    return (packed_value & mask) != 0u;
}
fn calc_ao(world_pos: vec3<f32>, normal: vec3<i32>) -> f32 {
    var voxel_x = i32(floor(world_pos.x - (f32(normal.x)/2.0)));
    var voxel_y = i32(floor(world_pos.y - (f32(normal.y)/2.0)));
    var voxel_z = i32(floor(world_pos.z - (f32(normal.z)/2.0)));
    
    let chunk_x = positive_modulo(voxel_x, CHUNK_SIZE);
    let chunk_y = positive_modulo(voxel_y, CHUNK_SIZE);
    let chunk_z = positive_modulo(voxel_z, CHUNK_SIZE);

    var chunk_pos = vec3<i32>(chunk_x, chunk_y, chunk_z);
    
    // Calculate the fractional part of the position (where in the block the point is)
    let fract_pos = vec3<f32>(
        world_pos.x - floor(world_pos.x),
        world_pos.y - floor(world_pos.y),
        world_pos.z - floor(world_pos.z)
    );
    
    // Calculate weights based on position within the block
    var weights: vec3<f32>;
    
    if (normal.x != 0) {
        // For X-facing faces, use y and z fractional parts
        weights = vec3<f32>(0.0, fract_pos.y, fract_pos.z);
    } else if (normal.y != 0) {
        // For Y-facing faces, use x and z fractional parts
        weights = vec3<f32>(fract_pos.x, 0.0, fract_pos.z);
    } else {
        // For Z-facing faces, use x and y fractional parts
        weights = vec3<f32>(fract_pos.x, fract_pos.y, 0.0);
    }
    
    // Check for corner neighbors and calculate AO value
    var ao_value: f32 = 1.0;
        
    if (normal.x != 0) {
        let side = normal.x;
        let top = check_voxel_presence(chunk_pos + vec3<i32>(side, 1, 0));
        let bottom = check_voxel_presence(chunk_pos + vec3<i32>(side, -1, 0));
        let front = check_voxel_presence(chunk_pos + vec3<i32>(side, 0, 1));
        let back = check_voxel_presence(chunk_pos + vec3<i32>(side, 0, -1));
        
        // Check corners
        let top_front = check_voxel_presence(chunk_pos + vec3<i32>(side, 1, 1));
        let top_back = check_voxel_presence(chunk_pos + vec3<i32>(side, 1, -1));
        let bottom_front = check_voxel_presence(chunk_pos + vec3<i32>(side, -1, 1));
        let bottom_back = check_voxel_presence(chunk_pos + vec3<i32>(side, -1, -1));
        
        // Calculate AO based on position within face
        let top_factor = mix(0.0, 0.25, f32(top));
        let bottom_factor = mix(0.0, 0.25, f32(bottom));
        let front_factor = mix(0.0, 0.25, f32(front));
        let back_factor = mix(0.0, 0.25, f32(back));
        
        // Corner factors have less weight
        let top_front_factor = mix(0.0, 0.125, f32(top_front && !(top && front)));
        let top_back_factor = mix(0.0, 0.125, f32(top_back && !(top && back)));
        let bottom_front_factor = mix(0.0, 0.125, f32(bottom_front && !(bottom && front)));
        let bottom_back_factor = mix(0.0, 0.125, f32(bottom_back && !(bottom && back)));
        
        // Calculate weighted AO value based on position within the face
        let y_weight = weights.y;
        let z_weight = weights.z;
        
        // Apply weights to get smooth AO across the face
        let top_ao = mix(top_back_factor, top_front_factor, z_weight) + top_factor;
        let bottom_ao = mix(bottom_back_factor, bottom_front_factor, z_weight) + bottom_factor;
        let vertical_ao = mix(bottom_ao, top_ao, y_weight);
        
        let front_ao = front_factor;
        let back_ao = back_factor;
        let horizontal_ao = mix(back_ao, front_ao, z_weight);
        
        ao_value = 1.0 - (vertical_ao + horizontal_ao);
    } else if (normal.y != 0) {
        let side = normal.y;
        let right = check_voxel_presence(chunk_pos + vec3<i32>(1, side, 0));
        let left = check_voxel_presence(chunk_pos + vec3<i32>(-1, side, 0));
        let front = check_voxel_presence(chunk_pos + vec3<i32>(0, side, 1));
        let back = check_voxel_presence(chunk_pos + vec3<i32>(0, side, -1));
        
        // Check corners
        let right_front = check_voxel_presence(chunk_pos + vec3<i32>(1, side, 1));
        let right_back = check_voxel_presence(chunk_pos + vec3<i32>(1, side, -1));
        let left_front = check_voxel_presence(chunk_pos + vec3<i32>(-1, side, 1));
        let left_back = check_voxel_presence(chunk_pos + vec3<i32>(-1, side, -1));
        
        let right_factor = mix(0.0, 0.25, f32(right));
        let left_factor = mix(0.0, 0.25, f32(left));
        let front_factor = mix(0.0, 0.25, f32(front));
        let back_factor = mix(0.0, 0.25, f32(back));
        
        let right_front_factor = mix(0.0, 0.125, f32(right_front && !(right && front)));
        let right_back_factor = mix(0.0, 0.125, f32(right_back && !(right && back)));
        let left_front_factor = mix(0.0, 0.125, f32(left_front && !(left && front)));
        let left_back_factor = mix(0.0, 0.125, f32(left_back && !(left && back)));
        
        let x_weight = weights.x;
        let z_weight = weights.z;
        
        let right_ao = mix(right_back_factor, right_front_factor, z_weight) + right_factor;
        let left_ao = mix(left_back_factor, left_front_factor, z_weight) + left_factor;
        let horizontal_ao = mix(left_ao, right_ao, x_weight);
        
        let front_ao = front_factor;
        let back_ao = back_factor;
        let depth_ao = mix(back_ao, front_ao, z_weight);
        
        ao_value = 1.0 - (horizontal_ao + depth_ao);
    } else {
        // Z-axis face (front/back)
        let side = normal.z;
        let right = check_voxel_presence(chunk_pos + vec3<i32>(1, 0, side));
        let left = check_voxel_presence(chunk_pos + vec3<i32>(-1, 0, side));
        let top = check_voxel_presence(chunk_pos + vec3<i32>(0, 1, side));
        let bottom = check_voxel_presence(chunk_pos + vec3<i32>(0, -1, side));
        
        // Check corners
        let top_right = check_voxel_presence(chunk_pos + vec3<i32>(1, 1, side));
        let top_left = check_voxel_presence(chunk_pos + vec3<i32>(-1, 1, side));
        let bottom_right = check_voxel_presence(chunk_pos + vec3<i32>(1, -1, side));
        let bottom_left = check_voxel_presence(chunk_pos + vec3<i32>(-1, -1, side));
        
        let right_factor = mix(0.0, 0.25, f32(right));
        let left_factor = mix(0.0, 0.25, f32(left));
        let top_factor = mix(0.0, 0.25, f32(top));
        let bottom_factor = mix(0.0, 0.25, f32(bottom));
        
        let top_right_factor = mix(0.0, 0.125, f32(top_right && !(top && right)));
        let top_left_factor = mix(0.0, 0.125, f32(top_left && !(top && left)));
        let bottom_right_factor = mix(0.0, 0.125, f32(bottom_right && !(bottom && right)));
        let bottom_left_factor = mix(0.0, 0.125, f32(bottom_left && !(bottom && left)));
        
        let x_weight = weights.x;
        let y_weight = weights.y;
        
        let top_ao = mix(top_left_factor, top_right_factor, x_weight) + top_factor;
        let bottom_ao = mix(bottom_left_factor, bottom_right_factor, x_weight) + bottom_factor;
        let vertical_ao = mix(bottom_ao, top_ao, y_weight);
        
        let right_ao = right_factor;
        let left_ao = left_factor;
        let horizontal_ao = mix(left_ao, right_ao, x_weight);
        
        ao_value = 1.0 - (vertical_ao + horizontal_ao);
    }
    
    // Clamp to ensure valid range
    return clamp(ao_value, 0.3, 1.0);
}

@vertex
fn vertex(vertex: VertexInput) -> CustomVertexOutput {
    var out: CustomVertexOutput;

    // Vertex specific information
    var vertex_info = vertex.voxel_data.x;
    var x = f32(vertex_info & MASK6) - EPSILON;
    var y = f32((vertex_info >> 6) & MASK6) - EPSILON;
    var z = f32((vertex_info >> 12) & MASK6) - EPSILON;
    var position = vec4(x, y, z, 1.0);
    
    // Quad specific information
    var quad_info = vertex.voxel_data.y;
    var n_id = quad_info & MASK3;
    var normal = normal_from_id(n_id);
    var c_id = (quad_info >> 3) & MASK9;
    var face_color = color_from_id(c_id);
    var h = f32((quad_info >> 18) & MASK6); // Height is at bits 18-23
    var w = f32((quad_info >> 24) & MASK6); // Width is at bits 24-29
    var face_light = light_from_id(n_id);
    
    out.position = mesh_position_local_to_clip(
        get_world_from_local(vertex.instance_index),
        position,
    );
    out.world_position = mesh_position_local_to_world(
        get_world_from_local(vertex.instance_index),
        position,
    );
    out.world_normal = normal;
    
    var uv_x: f32 = 0.0;
    var uv_y: f32 = 0.0;
    
    if (abs(normal.x) > 0.5) {
        // X-facing normal (YZ plane)
        // For a quad on the X face, y and z vary
        uv_x = (position.y % 1.0); // Local position within the starting voxel
        uv_y = (position.z % 1.0);
        
        // Store the base position of the quad in the chunk
        // This is already done in your code with world_position.w
    } else if (abs(normal.y) > 0.5) {
        // Y-facing normal (XZ plane)
        uv_x = (position.x % 1.0);
        uv_y = (position.z % 1.0);
    } else {
        // Z-facing normal (XY plane)
        uv_x = (position.x % 1.0);
        uv_y = (position.y % 1.0);
    }
    
    out.uv = vec2<f32>(uv_x, uv_y);
    
    
    out.color = face_color;
    out.face_light = face_light;
    out.face_normal = get_face_normal(n_id);
    
    // Pass quad information needed for fragment shader
    // Store quad origin and dimensions in world_position.w and face_light.w
    out.world_position.w = f32(u32(x) | (u32(y) << 10) | (u32(z) << 20));
    out.wh = vec2(w, h);
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
    
    // // sample texture and apply lighting
    pbr_input.material.base_color = in.color * in.face_light;
    
    // // alpha discard
    pbr_input.material.base_color = fns::alpha_discard(pbr_input.material, pbr_input.material.base_color);


#ifdef PREPASS_PIPELINE
    // in deferred mode we can't modify anything after that, as lighting is run in a separate fullscreen shader.
    var out = deferred_output(in, pbr_input);
    out.normal = in.normal;
    // Set depth value for SSAO
    out.depth = in.position.z;
    return out;
#else
    var out: FragmentOutput;
    // apply lighting
    out.color = apply_pbr_lighting(pbr_input);

    // // // apply in-shader post processing
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    // let neighbor_count = count_ao_neighbors(in.world_position.xyz, in.face_normal);
    // let debug_color = get_debug_color(neighbor_count);
    let ao = calc_ao(in.world_position.xyz, in.face_normal);
    out.color = vec4<f32>(out.color.r*ao,out.color.g*ao,out.color.b*ao, 1.0);
   
#endif


    return out;
}