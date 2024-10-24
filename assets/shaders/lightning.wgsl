#import bevy_pbr::{
    mesh_view_bindings::globals,
    mesh_view_bindings::view,
    forward_io::{VertexOutput, Vertex},
    utils::coords_to_viewport_uv,
}

const effect_color = vec3(1.0, 1.0, 0.8);
const octave_count = 2u;
const amp_start = 0.25;
const amp_coeff = 0.25;
const freq_coeff = 200.0;
const speed = 8.0;

fn hash12(x: vec2<f32>) -> f32 {
    return fract(cos(dot(x, vec2(13.9898, 8.141)) % 3.14) * 43758.5453);
}

fn hash22(uv: vec2<f32>) -> vec2<f32> {
    let uv2 = vec2(dot(uv, vec2(127.1,311.7)),
              dot(uv, vec2(269.5,183.3)));
    return 2.0 * fract(sin(uv2) * 43758.5453123) - 1.0;
}

fn noise(uv: vec2<f32>) -> f32 {
    let iuv:vec2<f32> = floor(uv);
    let fuv:vec2<f32> = fract(uv);
    let blur:vec2<f32> = smoothstep(vec2(0.0, 0.0), vec2(1.0, 1.0), fuv);
    return mix(mix(dot(hash22(iuv + vec2(0.0,0.0)), fuv - vec2(0.0,0.0)),
                   dot(hash22(iuv + vec2(1.0,0.0)), fuv - vec2(1.0,0.0)), blur.x),
               mix(dot(hash22(iuv + vec2(0.0,1.0)), fuv - vec2(0.0,1.0)),
                   dot(hash22(iuv + vec2(1.0,1.0)), fuv - vec2(1.0,1.0)), blur.x), blur.y) + 0.5;
}

fn fbm(uv: vec2<f32>, octaves:u32 ) -> f32 {
    var uv2 = uv;
    var value = 0.0;
    var amplitude = amp_start;
    for (var i = 0u; i < octaves; i++) {
        value += amplitude * noise(uv);
        uv2 *= freq_coeff;
        amplitude *= amp_coeff;
    }
    return value;
}

fn sdSegment( p:vec2f, a:vec2f, b:vec2f) -> f32
{
    let pa = p-a;
    let ba = b-a;
    let h = clamp( dot(pa,ba)/dot(ba,ba), 0.0, 1.0 );
    return length( pa - ba*h );
}

struct LineMaterial {
    points: array<vec4<f32>, 16>,
    point_count: u32,
};

@group(2) @binding(100) var<uniform> material: LineMaterial;


@fragment
fn fragment(
    in: VertexOutput,
) -> @location(0) vec4<f32> {
    var uv = in.world_position.xy + vec2(0.5, 0.5);
    uv += 2.0 * fbm(uv + globals.time * speed, octave_count) - 1.0;
    var d = 100.0;
    let p_count = material.point_count - 1;
    for (var i = 0u; i < p_count; i++) {
        let dd = sdSegment(uv, material.points[i].xy, material.points[i+1].xy);
        d = min(d, dd);
    }
    d *= 1.5;
    let color = effect_color * mix(0.0, 0.05, hash12(vec2(globals.time))) / d;
    let a = 1.0 - d*2.0;
    return vec4(color, a);
}
