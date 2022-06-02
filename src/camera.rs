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
    samples: u32,
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

pub mod CameraSystem {
    use bevy::{prelude::*, render::render_resource::{Extent3d, TextureDimension, TextureFormat}, utils::Instant, math::vec3};
    use rand::Rng;
    use rayon::prelude::*;

    use crate::{prelude::{Aabb, Bvh, Ray}, BvhInstance, BvhStats, tlas::Tlas};

    use super::BvhCamera;

    const TILE_SIZE: usize = 64;

    //
    // Camera Systems
    //
    pub fn init_camera_image(
        mut commands: Commands,
        mut query: Query<(Entity, &mut BvhCamera), Added<BvhCamera>>,
        mut images: ResMut<Assets<Image>>,
    ) {
        for (e, mut camera) in query.iter_mut() {
            let mut image = images.add(Image::new(
                Extent3d {
                    width: camera.width as u32,
                    height: camera.height as u32,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                vec![0; (camera.width * camera.height) as usize * 4],
                TextureFormat::Rgba8UnormSrgb,
            ));
            camera.image = Some(image);
        }
    }

    pub fn update_camera(mut camera_query: Query<(&mut BvhCamera, &GlobalTransform)>) {
        for (mut camera, trans) in camera_query.iter_mut() {
            camera.update(trans);
        }
    }

    pub fn render_camera(
        camera_query: Query<(&BvhCamera)>,
        mut images: ResMut<Assets<Image>>,        
        mut keys: ResMut<Input<KeyCode>>,
        mut stats: ResMut<BvhStats>,
        tlas: Res<Tlas>,
    ) {
        if let Ok(camera) = camera_query.get_single() && let Some(image) = &camera.image {
            let start = Instant::now();
        
            let mut image = images.get_mut(image).unwrap();
        
            // TODO: Make this acutally tilings, currenty this just takes a slice pixels in a row
            const pixel_tile_count: usize = 64;
            const pixel_tile: usize = 4 * pixel_tile_count;
            image.data.par_chunks_mut(pixel_tile)        
            .enumerate()
            .for_each(|(i, mut pixels)| {
                let mut rng = rand::thread_rng();
                let mut ray = Ray::default();
                for pixel_offset in 0..(pixels.len() / 4) {
                    let index = i * pixel_tile_count + pixel_offset;
                    let offset = pixel_offset * 4;

                    let x = index as u32 % camera.width;
                    let y = index as u32 / camera.width;                
                    let u = x as f32 / camera.width as f32;
                    let v = y as f32 / camera.height as f32;
                    // TODO: Revisit multiple samples later
                    // if samples > 0 {
                    //     u += rng.gen::<f32>() / camera.width as f32;
                    //     v += rng.gen::<f32>() / camera.height as f32;
                    // }
                        
                    // TODO: flip v since image is upside down, figure out why
                    camera.set_ray(&mut ray, u, 1.0 - v);                         
                    ray.intersect_tlas(&tlas);
    
                    
                    let color = if ray.hit.t < 1e30f32 {
                        //info!("{:?}", ray.hit.t);
                        let c = 500f32 - (ray.hit.t * 42f32);
                        vec3(c, c, c)
                    } else {
                        Vec3::ZERO
                    };
                    
                    pixels[offset + 0] = (color.x) as u8;
                    pixels[offset + 1] = (color.x) as u8;
                    pixels[offset + 2] = (color.x) as u8;
                    pixels[offset + 3] = 255;
                }  
            });

            stats.ray_count = camera.width as f32 * camera.height as f32 * camera.samples as f32;
            stats.camera_time = start.elapsed();
        }                
    }
}
