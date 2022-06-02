use bevy::{
    math::{vec3, Vec4Swizzles},
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    transform,
    utils::{Instant, hashbrown::hash_set::Intersection},
};
use bevy_inspector_egui::Inspectable;
use rand::prelude::*;
use rayon::prelude::*;

use crate::{
    prelude::{Aabb, Ray, Hit},
    BvhInstance, BvhStats,
};

#[derive(Component, Inspectable)]
pub struct BvhCamera {
    pub width: u32,
    pub height: u32,
    pub origin: Vec3,
    viewport_height: f32,
    viewport_width: f32,
    lower_left_corner: Vec3,
    focus_dist: f32,
    horizontal: Vec3,
    vertical: Vec3,
    u: Vec3,
    v: Vec3,
    w: Vec3,
    pub samples: u32,
    pub image: Option<Handle<Image>>,
}

impl BvhCamera {
    pub fn new(
        width: u32,
        height: u32,
        vfov: f32, // vertical field of view
        aperture: f32,
        focus_dist: f32,
        samples: u32,
    ) -> Self {
        let aspect_ratio = width as f32 / height as f32;
        let theta = vfov * std::f32::consts::PI / 180.0;
        let half_height = (theta / 2.0).tan();
        let viewport_height = 2.0 * half_height;
        let viewport_width = aspect_ratio * viewport_height;

        Self {
            width,
            height,
            viewport_height,
            viewport_width,
            focus_dist,
            samples,
            // Rest will be updated every frame for now
            origin: Vec3::ZERO,
            lower_left_corner: Vec3::ZERO,
            horizontal: Vec3::ZERO,
            vertical: Vec3::ZERO,
            u: Vec3::ZERO,
            v: Vec3::ZERO,
            w: Vec3::ONE,
            image: None,
        }
    }

    pub fn update(&mut self, trans: &GlobalTransform) {
        self.origin = trans.translation;

        let look_at = self.origin + trans.forward();

        self.w = -trans.forward();
        self.u = trans.right();
        self.v = trans.up();

        self.horizontal = self.focus_dist * self.viewport_width * self.u;
        self.vertical = self.focus_dist * self.viewport_height * self.v;

        self.lower_left_corner =
            self.origin - self.horizontal / 2.0 - self.vertical / 2.0 - self.focus_dist * self.w;
    }

    pub fn set_ray(&self, ray: &mut Ray, u: f32, v: f32) {
        ray.origin = self.origin;
        ray.direction = (self.lower_left_corner + u * self.horizontal + v * self.vertical - self.origin).normalize();        
        ray.direction_inv = ray.direction.recip();
        ray.t = 1e30f32;
        ray.hit = Hit::default();
    }
}
