#import bevy_pbr::{
    mesh_view_bindings::view,
    pbr_types::{PbrInput, pbr_input_new},
    mesh_functions::{get_world_from_local, mesh_position_local_to_clip, mesh_position_local_to_world},
    pbr_functions::{apply_pbr_lighting, calculate_view},
}
#import bevy_core_pipeline::tonemapping::tone_mapping

@group(2) @binding(0) var my_texture: texture_2d_array<f32>;
@group(2) @binding(1) var my_sampler: sampler;

struct VertexIn {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) texture_index: u32,
    @location(3) normal: vec3<f32>,
}

struct VertexOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) texture_index: u32,
    @location(2) normal: vec3<f32>,
    @location(3) world_position: vec4<f32>,
}

struct FragIn {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) texture_index: u32,
    @location(2) normal: vec3<f32>,
    @location(3) world_position: vec4<f32>,
}

@vertex
fn vertex(vertex: VertexIn) -> VertexOut {
    var output: VertexOut;
    let position = vec4<f32>(vertex.position, 1.0);
    let world_from_local = get_world_from_local(vertex.instance_index);
    output.clip_position = mesh_position_local_to_clip(world_from_local, position);
    output.uv = vertex.uv;
    output.texture_index = vertex.texture_index;
    output.normal = vertex.normal;
    output.world_position = mesh_position_local_to_world(world_from_local, position);
    return output;
}

@fragment
fn fragment(frag: FragIn) -> @location(0) vec4<f32> {
    var pbr_input: PbrInput = pbr_input_new();
    pbr_input.material.base_color = textureSample(my_texture, my_sampler, frag.uv, frag.texture_index);
    pbr_input.world_position = frag.world_position;
    pbr_input.frag_coord = frag.clip_position;
    pbr_input.is_orthographic = false;
    pbr_input.world_normal = frag.normal;
    pbr_input.N = normalize(frag.normal);
    pbr_input.V = normalize(view.world_position.xyz - frag.world_position.xyz);
    return tone_mapping(apply_pbr_lighting(pbr_input), view.color_grading);
}
