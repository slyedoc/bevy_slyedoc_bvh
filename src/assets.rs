use crate::{
    prelude::{Bvh, BvhInstance},
    tlas::Tlas,
    Tri,
};
use bevy::{math::vec3, prelude::*};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaChaRng;

// TODO: pretty sure vec3 already has this
fn random_vec3(rng: &mut impl Rng) -> Vec3 {
    vec3(
        rng.gen_range(-1.0..=1.0),
        rng.gen_range(-1.0..=1.0),
        rng.gen_range(-1.0..=1.0),
    )
}

pub fn gen_random_triangles(size: u32, scale: f32, rng: &mut impl Rng) -> Vec<Tri> {
    (0..size)
        .map(|_| {
            // TODO: there should already be a random vec3 impl somewhere
            let r0 = random_vec3(rng);
            let r1 = random_vec3(rng);
            let r2 = random_vec3(rng);

            let v0 = r0 * scale;
            Tri::new(v0, v0 + r1, v0 + r2)
        })
        .collect::<Vec<_>>()
}

/// Generate a random scene for testing
#[allow(dead_code)]
pub fn build_random_tri_scene() -> Tlas {
    let mut rng = ChaChaRng::seed_from_u64(0);
    let mut tlas = Tlas::default();
    let enity_count = 100;
    let tri_per_entity = 1000;
    // create a scene
    let side_count = (enity_count as f32).sqrt().ceil() as u32;
    let offset = 12.0;
    let side_offset = side_count as f32 * offset * 0.5;
    for i in 0..side_count {
        for j in 0..side_count {
            let id = i * side_count + j;
            let tris = gen_random_triangles(tri_per_entity, 4.0, &mut rng);
            let bvh_index = tlas.add_bvh(Bvh::new(tris));
            let e = Entity::from_raw(id);
            let mut blas = BvhInstance::new(e, bvh_index);

            // Bench: Go ahead and update the bvh instance, since we dont get updated by a service here
            blas.update(
                &GlobalTransform {
                    translation: vec3(
                        i as f32 * offset - side_offset + (offset * 0.5),
                        0.0,
                        j as f32 * offset - side_offset + (offset * 0.5),
                    ),
                    ..Default::default()
                },
                &tlas.bvhs[blas.bvh_index].nodes[0],
            );

            // Add to tlas
            tlas.add_instance(blas);
        }
    }
    // Bench: Build the tlas, since we dont get updated by a service here
    tlas.build();
    tlas
}
