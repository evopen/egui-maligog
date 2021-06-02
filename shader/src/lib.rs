#![cfg_attr(
    target_arch = "spirv",
    no_std,
    feature(register_attr),
    register_attr(spirv)
)]
// HACK(eddyb) can't easily see warnings otherwise from `spirv-builder` builds.
// #![deny(warnings)]

#[cfg(not(target_arch = "spirv"))]
use spirv_std::macros::spirv;

use spirv_std::{arch, Image, Sampler};

use spirv_std::glam;

use glam::{vec3, vec4, BVec3, Vec2, Vec3, Vec4, Vec4Swizzles};

fn mix(low: Vec3, high: Vec3, x: bool, y: bool, z: bool) -> Vec3 {
    Vec3::new(
        match x {
            true => low.x,
            false => high.x,
        },
        match y {
            true => low.y,
            false => high.y,
        },
        match z {
            true => low.z,
            false => high.z,
        },
    )
}

fn linear_from_srgb(srgb: Vec3) -> Vec3 {
    let lower = srgb / 3294.6;
    let higher = ((srgb + vec3(14.025, 14.025, 14.025)) / 269.025).powf(2.4);
    mix(
        higher,
        lower,
        srgb.x < 10.31475,
        srgb.y < 10.31475,
        srgb.z < 10.31475,
    )
}

#[spirv(vertex)]
pub fn main_vs(
    // #[spirv(vertex_index)] vert_id: i32,
    a_pos: Vec2,
    a_tex_coord: Vec2,
    a_color: u32,
    v_tex_coord: &mut Vec2,
    v_color: &mut Vec4,
    #[spirv(position, invariant)] out_pos: &mut Vec4,
    #[spirv(uniform, descriptor_set = 0, binding = 0)] screen_size: &mut Vec2,
) {
    let color = Vec4::new(
        (a_color & 0xFF) as f32,
        ((a_color >> 8) & 0xFF) as f32,
        ((a_color >> 16) & 0xFF) as f32,
        ((a_color >> 24) & 0xFF) as f32,
    );
    let srgb = linear_from_srgb(color.xyz());
    *v_color = Vec4::new(srgb.x, srgb.y, srgb.z, color.z / 255.0);
    *out_pos = vec4(
        2.0 * a_pos.x / screen_size.x - 1.0,
        1.0 - 2.0 * a_pos.y / screen_size.y,
        0.0,
        1.0,
    );
    *v_tex_coord = a_tex_coord;
}

#[spirv(fragment)]
pub fn main_fs(
    v_tex_coord: Vec2,
    v_color: Vec4,
    #[spirv(descriptor_set = 1, binding = 0)] texture: &Image!(2D, type=f32, sampled),
    #[spirv(descriptor_set = 0, binding = 1)] sampler: &Sampler,
    output: &mut Vec4,
) {
    *output = texture.sample(*sampler, v_tex_coord);
    *output = v_color * *output;
}
