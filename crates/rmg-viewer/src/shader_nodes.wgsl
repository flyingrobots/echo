// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

struct Globals {
  view_proj: mat4x4<f32>,
  light_dir: vec3<f32>,
  _pad: f32,
};

@group(0) @binding(0)
var<uniform> globals: Globals;

struct VsIn {
  @location(0) pos: vec3<f32>,
  @location(1) normal: vec3<f32>,
  @location(2) m0: vec4<f32>,
  @location(3) m1: vec4<f32>,
  @location(4) m2: vec4<f32>,
  @location(5) m3: vec4<f32>,
  @location(6) color: vec4<f32>,
};

struct VsOut {
  @builtin(position) pos: vec4<f32>,
  @location(0) normal: vec3<f32>,
  @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(v: VsIn) -> VsOut {
  let model = mat4x4<f32>(v.m0, v.m1, v.m2, v.m3);
  let world_pos = model * vec4<f32>(v.pos, 1.0);
  let world_normal = normalize((model * vec4<f32>(v.normal, 0.0)).xyz);
  var o: VsOut;
  o.pos = globals.view_proj * world_pos;
  o.normal = world_normal;
  o.color = v.color;
  return o;
}

@fragment
fn fs_main(v: VsOut) -> @location(0) vec4<f32> {
  let n = normalize(v.normal);
  let l = normalize(-globals.light_dir);
  let diff = max(dot(n, l), 0.1);
  let color = v.color.rgb * diff + 0.05;
  return vec4<f32>(color, v.color.a);
}
