struct Screen {
    screen_size: vec2<f32>,
    cell_size: vec2<f32>,
    origin: vec2<f32>,
    pad: vec2<f32>,
};

@group(0) @binding(0) var<uniform> screen: Screen;
@group(0) @binding(1) var atlas: texture_2d<f32>;
@group(0) @binding(2) var atlas_sampler: sampler;

struct InstanceInput {
    @location(0) pos: vec2<f32>,
    @location(1) size: vec2<f32>,
    @location(2) uv: vec4<f32>,
    @location(3) fg: vec4<f32>,
    @location(4) bg: vec4<f32>,
    @location(5) kind: u32,
};

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) fg: vec4<f32>,
    @location(2) @interpolate(flat) kind: u32,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32, instance: InstanceInput) -> VsOut {
    let corners = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 1.0),
    );
    let corner = corners[vi];
    let px = instance.pos + corner * instance.size;
    let ndc = vec2<f32>(
        px.x / screen.screen_size.x * 2.0 - 1.0,
        1.0 - px.y / screen.screen_size.y * 2.0,
    );
    var out: VsOut;
    out.clip = vec4<f32>(ndc, 0.0, 1.0);
    out.uv = instance.uv.xy + corner * instance.uv.zw;
    out.fg = instance.fg;
    out.kind = instance.kind;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    if (in.kind == 0u) {
        return vec4<f32>(in.fg.rgb, in.fg.a);
    }
    let alpha = textureSample(atlas, atlas_sampler, in.uv).r;
    return vec4<f32>(in.fg.rgb, in.fg.a * alpha);
}
