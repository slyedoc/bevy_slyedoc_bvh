use std::{f32::consts::PI, time::Duration};

use bevy::{math::vec3, prelude::*};
use bevy_slyedoc_bvh::prelude::*;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use image::{Rgb, RgbImage};
use rand::prelude::*;
use rand_chacha::ChaChaRng;

criterion_group!(benches, tlas_intersection,);
criterion_main!(benches);

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

fn random_vec3(rng: &mut impl Rng) -> Vec3 {
    vec3(
        rng.gen_range(-1.0..=1.0),
        rng.gen_range(-1.0..=1.0),
        rng.gen_range(-1.0..=1.0),
    )
}

fn tlas_intersection(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("bvh_intersection");
    group.warm_up_time(Duration::from_millis(500));
    group.measurement_time(Duration::from_secs(30));
    
    let entity_count = 100;
    let tri_per_entity = 1000;
    let tlas = build_random_tri_scene(entity_count, tri_per_entity);

    for size in [256, 512, 1024] { 
        let name = format!("random_{}k_tri_{}x{}", (entity_count * tri_per_entity) as f32 / 1000.0, size, size );
        if size > 512 {
            group.sample_size(30);
        }
        
        group.bench_function(name.clone(), |bencher| {            
            bencher.iter(|| {        
                    let mut camera = BvhCamera::new(size, size);
            
                    // Bench: update camera with trans, since we dont get updated by a service here
                    camera.update(&GlobalTransform {
                        translation: vec3(0.0, 40.0, 100.0),
                        rotation: Quat::from_axis_angle(Vec3::X, - PI / 6.0),
                        ..Default::default()
                    });
                    
                    let mut img = RgbImage::new(camera.width, camera.height);
                    let mut ray = Ray::default();
                    for y in 0..camera.height {
                        for x in 0..camera.width {
                            camera.set_ray(
                                &mut ray,
                                x as f32 / camera.width as f32,
                                // TODO: image still reversed
                                1.0 - (y as f32 / camera.height as f32),
                            );
                            ray.intersect_tlas(&tlas);
                            let color = if ray.hit.t < 1e30f32 {
                                let c = vec3(ray.hit.u, ray.hit.v, 1.0 - (ray.hit.u + ray.hit.v)) * 255.0;
                                Rgb([c.x as u8, c.y as u8, c.z as u8])                    
                            } else {
                                Rgb([0, 0, 0])
                            };
                            img[(x, y)] = color;
                        }
                    }
            
                    // img.save(format!("out/{}.png", name))
                    //     .unwrap();    
                    black_box(img);
                    
            });
        });
    }
    group.finish();
}

fn build_random_tri_scene(enity_count: u32, tri_per_entity: u32) -> Tlas {
    let mut rng = ChaChaRng::seed_from_u64(0);
    let mut tlas = Tlas::default();
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
                    translation: vec3(i as f32 * offset - side_offset + (offset * 0.5), 0.0, j as f32 * offset - side_offset + (offset * 0.5)),
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
