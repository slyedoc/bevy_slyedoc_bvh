use bevy::prelude::*;

#[derive(Debug, Copy, Clone)]
pub struct Aabb {
    bmin: Vec3,
    bmax: Vec3,
}

impl Default for Aabb {
    fn default() -> Self {
        Self {
            bmin: Vec3::splat(1e30f32),
            bmax: Vec3::splat(-1e30f32),
        }
    }
}

impl Aabb {
    pub fn grow(&mut self, p: Vec3) {
        self.bmin = self.bmin.min(p);
        self.bmax = self.bmax.max(p);
    }

    pub fn grow_aabb(&mut self, b: &Aabb) {
        self.grow(b.bmin);
        self.grow(b.bmax);
    }

    pub fn area(&self) -> f32 {
        let e = self.bmax - self.bmin; // box extent
        e.x * e.y + e.y * e.z + e.z * e.x
    }
}
