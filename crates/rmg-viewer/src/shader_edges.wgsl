// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

// NOTE: light_dir is unused in this shader but retained to match the CPU-side
// Globals layout shared with node rendering. Keep field order/size identical
// to `crate::gpu::Globals`.
struct Globals {
  view_proj: mat4x4<f32>,
  light_dir: vec3<f32>,
  _pad: f32,
};

@group(0) @binding(0)
var<uniform> globals: Globals;

struct EdgeIn {
  @location(0) start: vec3<f32>,
  @location(1) end: vec3<f32>,
  @location(2) color: vec3<f32>,
  @location(3) head: f32,
};

struct VsOut {
  @builtin(position) pos: vec4<f32>,
  @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vid: u32, e: EdgeIn) -> VsOut {
  let delta = e.end - e.start;
  let len = length(delta);
  let dir = if (len > 1e-6) { delta / len } else { vec3<f32>(0.0, 0.0, 1.0) };
  var p: vec3<f32>;
  if (vid == 0u) {
    p = e.start + dir * e.head;
  } else {
    p = e.end - dir * e.head;
  }
  var o: VsOut;
  o.pos = globals.view_proj * vec4<f32>(p, 1.0);
  o.color = e.color;
  return o;
}

@fragment
fn fs_main(v: VsOut) -> @location(0) vec4<f32> {
  return vec4<f32>(v.color, 1.0);
}
