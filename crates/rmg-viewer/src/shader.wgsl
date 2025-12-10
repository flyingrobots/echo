struct Globals {
    view_proj: mat4x4<f32>;
    light_dir: vec3<f32>;
    _pad: f32;
};

@group(0) @binding(0)
var<uniform> globals: Globals;

struct VsIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) i0: vec4<f32>,
    @location(3) i1: vec4<f32>,
    @location(4) i2: vec4<f32>,
    @location(5) i3: vec4<f32>,
    @location(6) color: vec3<f32>,
};

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) color: vec3<f32>,
};

@vertex
fn vs_main(input: VsIn) -> VsOut {
    let model = mat4x4<f32>(input.i0, input.i1, input.i2, input.i3);
    let world_pos = model * vec4<f32>(input.position, 1.0);
    let m3 = mat3x3<f32>(input.i0.xyz, input.i1.xyz, input.i2.xyz);
    let normal_mat = transpose(inverse(m3));
    let world_normal = normalize(normal_mat * input.normal);
    var out: VsOut;
    out.pos = globals.view_proj * world_pos;
    out.normal = world_normal;
    out.color = input.color;
    return out;
}

@fragment
fn fs_main(input: VsOut) -> @location(0) vec4<f32> {
    let n = normalize(input.normal);
    let l = normalize(-globals.light_dir);
    let diff = max(dot(n, l), 0.1);
    let color = input.color * diff + 0.08;
    return vec4<f32>(color, 1.0);
}
