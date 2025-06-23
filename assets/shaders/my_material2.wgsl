#import bevy_pbr::{
    forward_io::VertexOutput,
    mesh_view_bindings::view,
    pbr_types::{PbrInput, pbr_input_new},
    mesh_functions::{get_world_from_local, mesh_position_local_to_clip},
    pbr_functions::{apply_pbr_lighting, calculate_view},
}
#import bevy_core_pipeline::tonemapping::tone_mapping

@group(2) @binding(0) var my_texture: texture_2d<f32>;
@group(2) @binding(1) var my_sampler: sampler;


@fragment
fn fragment(frag: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(my_texture, my_sampler, frag.uv);
    let ones = vec3f(1.0, 1.0, 1.0);

    var pbr_input: PbrInput = pbr_input_new();
    pbr_input.material.base_color = color;
    pbr_input.world_position = frag.world_position;
    pbr_input.frag_coord = frag.position;
    pbr_input.is_orthographic = false;
    pbr_input.world_normal = frag.world_normal;
    pbr_input.N = normalize(frag.world_normal);
    pbr_input.V = normalize(view.world_position.xyz - frag.world_position.xyz);
    // pbr_input.V = calculate_view(frag.world_position, false);
    let color2 = apply_pbr_lighting(pbr_input);

    let color3 = tone_mapping(color2, view.color_grading);
    // return vec4f((pbr_input.V + ones) / 2.0, 1.0);
    // return vec4f(fract(view.world_position.xyz / 4.0), 1.0);
    // return vec4f(fract(frag.world_position.xyz / 4.0), 1.0);
    return color3;
}
