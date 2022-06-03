use bevy::{prelude::*, math::vec3};
use rand::Rng;
use crate::Tri;

// generate random triangles
#[allow(dead_code)]
pub fn gen_random_triangles(size: usize, rng: &mut impl Rng) -> Vec<Tri> {
    (0..size)
        .map(|_| {

            // TODO: there should already be a random vec3 impl somewhere
            let r0 = random_vec3(rng);
            let r1 = random_vec3(rng);
            let r2 = random_vec3(rng);

            let v0 = r0 * 5.0;
            Tri::new(v0, v0 + r1, v0 + r2)
        })
        .collect::<Vec<_>>()
}

// TODO: pretty sure vec3 already has this
fn random_vec3(rng: &mut impl Rng) -> Vec3 {
    vec3(
        rng.gen_range(-1.0..=1.0),
        rng.gen_range(-1.0..=1.0),
        rng.gen_range(-1.0..=1.0),
    )
}
