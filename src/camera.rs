use bevy::{prelude::*, math::vec3};


use crate::prelude::*;

#[derive(Component)]
pub struct BvhCamera {
    pub position: Vec3,
    pub p0: Vec3,
    pub p1: Vec3,
    pub p2: Vec3,
}

impl Default for BvhCamera {
    fn default() -> Self {
        BvhCamera {
            position: -Vec3::Z * 3.0,
            p0: vec3(-1.0, 1.0, 2.0),
            p1: vec3(1.0, 1.0, 2.0),
            p2: vec3(-1.0, -1.0, 2.0),
        }
    }
}

impl BvhCamera {
    pub fn new(position: Vec3, p0: Vec3, p1: Vec3, p2: Vec3) -> Self {
        BvhCamera {
            position,
            p0,
            p1,
            p2,
        }
    }


pub fn render_image(
    &self,
    width: u32,
    height: u32,
    bvh: &Bvh,
    tris: &[Tri],
    img: &mut [u8],
) {
    let mut ray = Ray {
        origin: self.position,
        ..Default::default()
    };

    let transform = Transform::default();
    let inv_trans = InvTrans(transform.compute_matrix().inverse());

    for tile in 0..6400 {
        let x = tile % 80;
        let y = tile / 80;

        for v in 0u32..8 {
            for u in 0u32..8 {
                let pixel_pos = ray.origin
                    + self.p0
                    + (self.p1 - self.p0) * ((x * 8 + u) as f32 / width as f32)
                    + (self.p2 - self.p0) * ((y * 8 + v) as f32 / height as f32);

                ray.direction = (pixel_pos - ray.origin).normalize();
                ray.direction_inv = ray.direction.recip();
                ray.t = 1e30f32;

                #[cfg(feature = "bvh")]
                bvh.intersect(&mut ray, tris, &inv_trans);

                #[cfg(feature = "brute")]
                for t in tris {
                    ray.intersect_triangle(t);
                }
                 
                // println!("{}", c);
                let c = 500f32 - (ray.t * 42f32);
                if ray.t < 1e30f32 {
                    let pixel_index = ((y * 8 + v) * width + (x * 8 + u)) as usize * 3;
                    img[pixel_index] = c as u8;
                    img[pixel_index + 1] = c as u8;
                    img[pixel_index + 2] = c as u8;
                    //img.put_pixel(x * 8 + u, y * 8 + v, Rgb([c as u8, c  as u8, c  as u8]));
                }
            }
        }
    }
}

}