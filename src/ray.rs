use std::{arch::x86_64::*, mem::swap};
use tracing::{debug, error, info, span, warn, Level};

use crate::{
    prelude::BvhInstance,
    tlas::{Tlas, TlasNode},
    tri::Tri,
    Bvh,
};
use bevy::prelude::*;

#[derive(Debug, Clone, Copy)]
pub struct Hit {
    pub t: f32, // intersection distance along ray
    pub u: f32, // barycentric coordinates of the intersection
    pub v: f32,
    // We are using more bits here than in tutorial
    pub tri_index: usize,
    pub entity: Entity,
}

impl Default for Hit {
    fn default() -> Self {
        Self {
            t: 1e30f32,
            u: Default::default(),
            v: Default::default(),
            tri_index: Default::default(),
            // TODO: Yes this isnt ideal, should be an option, will come back to this
            entity: Entity::from_raw(0),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3, // Should be normalized
    pub direction_inv: Vec3,
    pub t: f32,
    pub hit: Option<Hit>,
}

impl Default for Ray {
    fn default() -> Self {
        Ray {
            origin: Vec3::ZERO,
            direction: Vec3::Z,
            t: 1e30f32,
            direction_inv: Vec3::ZERO,
            hit: None,
        }
    }
}

impl Ray {
    pub fn new(origin: Vec3, direction: Vec3) -> Self {
        let direction_inv = direction.recip();
        Self {
            origin,
            direction,
            direction_inv,
            ..Default::default()
        }
    }

    // TODO: This is from bevy_mod_raycast, need to do more reading up on ndc
    pub fn from_screenspace(
        cursor_pos_screen: Vec2,
        window: &Window,
        camera: &Camera,
        camera_transform: &GlobalTransform,
    ) -> Self {
        let camera_position = camera_transform.compute_matrix();
        let screen_size = Vec2::from([window.width() as f32, window.height() as f32]);
        let projection_matrix = camera.projection_matrix;

        // Normalized device coordinate cursor position from (-1, -1, -1) to (1, 1, 1)
        let cursor_ndc = (cursor_pos_screen / screen_size) * 2.0 - Vec2::from([1.0, 1.0]);
        let cursor_pos_ndc_near: Vec3 = cursor_ndc.extend(-1.0);
        let cursor_pos_ndc_far: Vec3 = cursor_ndc.extend(1.0);

        // Use near and far ndc points to generate a ray in world space
        // This method is more robust than using the location of the camera as the start of
        // the ray, because ortho cameras have a focal point at infinity!
        let ndc_to_world: Mat4 = camera_position * projection_matrix.inverse();
        let cursor_pos_near: Vec3 = ndc_to_world.project_point3(cursor_pos_ndc_near);
        let cursor_pos_far: Vec3 = ndc_to_world.project_point3(cursor_pos_ndc_far);
        let ray_direction = cursor_pos_far - cursor_pos_near;

        Ray {
            origin: cursor_pos_near,
            direction: ray_direction,
            direction_inv: ray_direction.recip(),
            t: 1e30,
            hit: None,
        }
    }

    // Moller Trumbore
    // https://en.wikipedia.org/wiki/M%C3%B6ller%E2%80%93Trumbore_intersection_algorithm
    pub fn intersect_triangle(&mut self, tri: &Tri, tri_index: usize, entity: Entity) {
        #[cfg(feature = "trace")]
        let _span = info_span!("intersect_triangle").entered();
        let edge1 = tri.vertex1 - tri.vertex0;
        let edge2 = tri.vertex2 - tri.vertex0;
        let h = self.direction.cross(edge2);
        let a = edge1.dot(h);
        if a > -0.0001 && a < 0.0001 {
            return;
        };
        // ray parallel to triangle
        let f = 1.0 / a;
        let s = self.origin - tri.vertex0;
        let u = f * s.dot(h);
        if !(0.0..=1.0).contains(&u) {
            return;
        }
        let q = s.cross(edge1);
        let v = f * self.direction.dot(q);
        if v < 0.0 || u + v > 1.0 {
            return;
        }
        let t = f * edge2.dot(q);
        // TODO: The option part here feels sloppy
        if t > 0.0001 {
            if let Some(hit) = self.hit {
                if t < hit.t {
                    self.hit = Some(Hit {
                        t,
                        u,
                        v,
                        tri_index,
                        entity,
                    });
                }
            } else {
                self.hit = Some(Hit {
                    t,
                    u,
                    v,
                    tri_index,
                    entity,
                });
            }
        }
    }

    pub fn intersect_aabb(&self, bmin: Vec3, bmax: Vec3) -> f32 {
        #[cfg(feature = "trace")]
        let _span = info_span!("intersect_aabb").entered();
        let tx1 = (bmin.x - self.origin.x) * self.direction_inv.x;
        let tx2 = (bmax.x - self.origin.x) * self.direction_inv.x;
        let tmin = tx1.min(tx2);
        let tmax = tx1.max(tx2);
        let ty1 = (bmin.y - self.origin.y) * self.direction_inv.y;
        let ty2 = (bmax.y - self.origin.y) * self.direction_inv.y;
        let tmin = tmin.max(ty1.min(ty2));
        let tmax = tmax.min(ty1.max(ty2));
        let tz1 = (bmin.z - self.origin.z) * self.direction_inv.z;
        let tz2 = (bmax.z - self.origin.z) * self.direction_inv.z;
        let tmin = tmin.max(tz1.min(tz2));
        let tmax = tmax.min(tz1.max(tz2));
        let t_hit = if let Some(hit) = self.hit {
            hit.t
        } else {
            1e30f32
        };
        
        if tmax >= tmin && tmin < t_hit && tmax > 0.0 {
            tmin
        } else {
            1e30f32
        }
    }

    pub fn intersect_bvh(&mut self, bvh: &Bvh, entity: Entity) {
        #[cfg(feature = "trace")]
        let _span = info_span!("intersect_bvh").entered();
        let mut node = &bvh.nodes[0];
        let mut stack = Vec::with_capacity(64);
        loop {
            if node.is_leaf() {
                for i in 0..node.tri_count {
                    let tri_index = bvh.triangle_indexs[(node.left_first + i) as usize];
                    self.intersect_triangle(&bvh.tris[tri_index], tri_index, entity);
                }
                if stack.is_empty() {
                    break;
                }
                node = stack.pop().unwrap();
                continue;
            }
            let mut child1 = &bvh.nodes[node.left_first as usize];
            let mut child2 = &bvh.nodes[(node.left_first + 1) as usize];
            let mut dist1 = self.intersect_aabb(child1.aabb_min, child1.aabb_max);
            let mut dist2 = self.intersect_aabb(child2.aabb_min, child2.aabb_max);
            if dist1 > dist2 {
                swap(&mut dist1, &mut dist2);
                swap(&mut child1, &mut child2);
            }
            if dist1 == 1e30f32 {
                if stack.is_empty() {
                    break;
                }
                node = stack.pop().unwrap();
            } else {
                node = child1;
                if dist2 != 1e30f32 {
                    stack.push(child2);
                }
            }
        }
    }

    pub fn intersect_bvh_instance(&mut self, bvh_instance: &BvhInstance, bvhs: &[Bvh]) {
        #[cfg(feature = "trace")]
        let _span = info_span!("intersect_bvh_instance").entered();
        let bvh = &bvhs[bvh_instance.bvh_index];
        // backup ray and transform original
        let mut backup_ray = self.clone();

        self.origin = bvh_instance.inv_trans.transform_point3(self.origin);
        self.direction = bvh_instance.inv_trans.transform_vector3(self.direction);
        self.direction_inv = self.direction.recip();
        self.intersect_bvh(bvh, bvh_instance.entity);

        // restore ray origin and direction
        backup_ray.hit = self.hit;
        *self = backup_ray;
    }

    pub fn intersect_tlas(&mut self, tlas: &Tlas) -> Option<Hit> {
        #[cfg(feature = "trace")]
        let _span = info_span!("intersect_tlas").entered();
        let mut stack = Vec::<&TlasNode>::with_capacity(64);
        let mut node = &tlas.tlas_nodes[0];
        while true {
            if node.is_leaf() {
                self.intersect_bvh_instance(&tlas.blas[node.blas as usize], &tlas.bvhs);
                if stack.is_empty() {
                    break;
                } else {
                    node = stack.pop().unwrap();
                }
                continue;
            }
            let mut child1 = &tlas.tlas_nodes[(node.left_right & 0xffff) as usize];
            let mut child2 = &tlas.tlas_nodes[(node.left_right >> 16) as usize];
            let mut dist1 = self.intersect_aabb(child1.aabb_min, child1.aabb_max);
            let mut dist2 = self.intersect_aabb(child2.aabb_min, child2.aabb_max);
            if dist1 > dist2 {
                swap(&mut dist1, &mut dist2);
                swap(&mut child1, &mut child2);
            }
            if dist1 == 1e30f32 {
                if stack.is_empty() {
                    break;
                } else {
                    node = &stack.pop().unwrap();
                }
            } else {
                node = child1;
                if dist2 != 1e30f32 {
                    stack.push(child2);
                }
            }
        }
        self.hit
    }
}
