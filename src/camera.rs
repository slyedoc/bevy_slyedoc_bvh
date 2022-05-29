use bevy::{prelude::*, transform, render::render_resource::{Extent3d, TextureDimension, TextureFormat}};
use bevy_inspector_egui::Inspectable;

use crate::prelude::Ray;

#[derive(Component, Inspectable)]
pub struct BvhCamera {

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
}

impl BvhCamera {
    pub fn new(
        vfov: f32, // vertical field of view
        aspect_ratio: f32,
        aperture: f32,
        focus_dist: f32,
    ) -> Self {
        let theta = vfov * std::f32::consts::PI / 180.0;
        let half_height = (theta / 2.0).tan();
        let viewport_height = 2.0 * half_height;
        let viewport_width = aspect_ratio * viewport_height;

        Self {
            viewport_height,
            viewport_width,
            focus_dist,

            // Rest will be updated every frame for now
            origin: Vec3::ZERO,
            lower_left_corner:  Vec3::ZERO,
            horizontal: Vec3::ZERO,
            vertical: Vec3::ZERO,
            u: Vec3::ZERO,
            v: Vec3::ZERO,
            w: Vec3::ONE,
            
        }
    }

    pub fn update(&mut self, trans: &GlobalTransform) {
        self.origin = trans.translation;

        let look_at = self.origin + trans.forward();
        
        self.w = (self.origin - look_at).normalize();
        self.u = trans.up().cross(self.w).normalize();
        self.v = self.w.cross(self.u);


        self.horizontal = self.focus_dist * self.viewport_width * self.u;
        self.vertical = self.focus_dist * self.viewport_height * self.v;

        self.lower_left_corner = self.origin
        - self.horizontal / 2.0
        - self.vertical / 2.0
        - self.focus_dist * self.w;
        // let origin = look_from;
        // let horizontal = focus_dist * viewport_width * u;
        // let vertical = focus_dist * viewport_height * v;
    }

    pub fn get_ray(&self, u: f32, v: f32) -> Ray {
        //let rd = self.lens_radius * Vec3::random_in_unit_disk();
        let offset = Vec3::ZERO;
        let mut direction = self.lower_left_corner + u * self.horizontal + v * self.vertical
        - self.origin
        - offset; 
        direction = direction.normalize();


        Ray {
            origin: self.origin,
            direction: direction,
            direction_inv: direction.recip(),
            t: 1e30f32,
        }
        // Ray {
        //     origin: self.origin,
        //     direction: self.lower_left_corner + u * self.horizontal + v * self.vertical - self.origin,
        // }
    }
}


#[derive(Inspectable)]
pub struct BvhImage {
    pub width: u32,
    pub height: u32,
    pub image: Handle<Image>,
    pub material: Handle<StandardMaterial>,
}

impl FromWorld for BvhImage {
    fn from_world(world: &mut World) -> Self {
        let world = world.cell();
        let mut materials = world
            .get_resource_mut::<Assets<StandardMaterial>>()
            .unwrap();

        let mut images = world.get_resource_mut::<Assets<Image>>().unwrap();

        let mut image = Image::new(
            Extent3d {
                width: 640,
                height: 640,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            vec![0; 640 * 640 * 4],
            TextureFormat::Rgba8UnormSrgb,
        );

        let img_handle = images.add(image);

        BvhImage {
            width: 640,
            height: 640,
            image: img_handle.clone(),
            material: materials.add(StandardMaterial {
                //base_color: Color::ORANGE_RED,
                base_color_texture: Some(img_handle),
                unlit: true,
                ..Default::default()
            }),
        }
    }
}