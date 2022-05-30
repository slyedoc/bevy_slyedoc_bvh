use bevy::{
    math::{vec3, Vec4Swizzles},
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    transform,
    utils::Instant,
};
use bevy_inspector_egui::Inspectable;
use rand::{prelude::ThreadRng, Rng};
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use crate::{
    prelude::{Aabb, Ray},
    BvhHandle, BvhVec, InvTrans, Tris,
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

        self.w = -trans.forward(); //(self.origin - look_at).normalize();
        self.u = trans.right(); //.cross(self.w).normalize();
        self.v = trans.up(); //self.w.cross(self.u);

        self.horizontal = self.focus_dist * self.viewport_width * self.u;
        self.vertical = self.focus_dist * self.viewport_height * self.v;

        self.lower_left_corner =
            self.origin - self.horizontal / 2.0 - self.vertical / 2.0 - self.focus_dist * self.w;
        // let origin = look_from;
        // let horizontal = focus_dist * viewport_width * u;
        // let vertical = focus_dist * viewport_height * v;
    }

    pub fn get_ray(&self, u: f32, v: f32) -> Ray {
        //let rd = self.lens_radius * Vec3::random_in_unit_disk();
        let offset = Vec3::ZERO;
        let mut direction =
            self.lower_left_corner + u * self.horizontal + v * self.vertical - self.origin - offset;
        direction = direction.normalize();

        Ray {
            origin: self.origin,
            direction: direction,
            direction_inv: direction.recip(),
            t: 1e30f32,
        }
    }

    //
    // Camera Systems
    //
    pub fn init_camera_image(
        mut commands: Commands,
        mut query: Query<(Entity, &mut BvhCamera), Added<BvhCamera>>,
        mut images: ResMut<Assets<Image>>,
    ) {
        for (e, mut camera) in query.iter_mut() {
            info!("Setup Camera");
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
        query: Query<(
            Entity,
            &GlobalTransform,
            &Tris,
            &InvTrans,
            &Aabb,
            &BvhHandle,
        )>,
        camera_query: Query<(&BvhCamera)>,
        bvh_vec: Res<BvhVec>,
        mut images: ResMut<Assets<Image>>,
        mut keys: ResMut<Input<KeyCode>>,
    ) {
        let start = Instant::now();

        if let Ok(camera) = camera_query.get_single() {
            if let Some(image) = &camera.image {
                let mut image = images.get_mut(image).unwrap();
                let pixel_count = camera.height * camera.width;
                // for each pixel, find the ray it would cast from the camera
                let results = (0..pixel_count)
                    .into_par_iter()
                    .map(|i| {
                        let x = i % camera.width;
                        let y = i / camera.width;
                        let pixel_index = ((camera.height - y - 1) * camera.width + x) as usize * 4;
                        let mut rng = rand::thread_rng();
                        let mut result = Vec4::new(0.0, 0.0, 0.0, 1.0);

                        for k in 0..camera.samples {
                            let u = if camera.samples > 1 {
                                (x as f32 + rng.gen_range(0.0..1.0)) / camera.width as f32
                            } else {
                                x as f32 / camera.width as f32
                            };

                            let v = if camera.samples > 1 {
                                (y as f32 + rng.gen_range(0.0..1.0)) / camera.height as f32
                            } else {
                                y as f32 / camera.height as f32
                            };
                            let mut ray = camera.get_ray(u, v);

                            let mut t = ray.t;
                            let mut target_entity = None;
                            for (e, _trans, tris, inv_trans, bounds, bvh_handle) in query.iter() {
                                //if ray.intersect_aabb(bounds.bmin, bounds.bmax) != 1e30f32 {
                                let bvh = bvh_vec.get(bvh_handle);
                                bvh.intersect(&mut ray, &tris.0, &inv_trans);
                                if t != ray.t {
                                    target_entity = Some((e, ray));
                                    t = ray.t;
                                }
                            }

                            if let Some((e, ray)) = target_entity {
                                let c = 900f32 - (ray.t * 42f32);
                                let c = c as u8;
                                let c = c as f32 / 255f32;
                                let c = Vec4::new(c, c, c, 1.0);
                                result += c;
                            } else {
                                result += Vec4::new(0.0, 0.0, 0.0, 1.0);
                            }
                        }
                        result /= camera.samples as f32;

                        (
                            pixel_index,
                            [
                                (result.x * 255.0) as u8,
                                (result.y * 255.0) as u8,
                                (result.z * 255.0) as u8,
                                (result.w * 255.0) as u8,
                            ],
                        )
                    })
                    .collect::<Vec<_>>();

                // save results, should part of the parallelized step above
                for (i, pixel) in results.iter() {
                    image.data[*i] = pixel[0];
                    image.data[i + 1] = pixel[1];
                    image.data[i + 2] = pixel[2];
                    image.data[i + 3] = pixel[3];
                }
            }
            //info!("Render time: {:?}", start.elapsed());
        }
        //}
    }
}

fn random_in_unit_disk(rng: &ThreadRng) -> Vec3 {
    loop {
        let mut rng = rand::thread_rng();
        let p = vec3(rng.gen_range(-1.0..1.0), rng.gen_range(-1.0..1.0), 0.0);
        if p.length_squared() >= 1.0 {
            continue;
        }
        return p;
    }
}
