use std::{f32::consts::PI, time::Duration};

use bevy::{math::vec3, prelude::*};
use bevy_slyedoc_bvh::prelude::*;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use image::{Rgb, RgbImage};


criterion_group!(benches, tlas_intersection);
criterion_main!(benches);

fn tlas_intersection(criterion: &mut Criterion) {
    let tlas = build_random_tri_scene();

    let mut group = criterion.benchmark_group("bvh_intersection");
    group.warm_up_time(Duration::from_millis(500));
    

    // size and time
    for (img_size, time) in [(256, 10), (512, 60)] {
        group.measurement_time(Duration::from_secs(time));
        let name = format!(
            "random_{}k_tri_{}x{}",
            (100 * 1000) as f32 / 1000.0,
            img_size,
            img_size
        );

        let mut camera = BvhCamera::new(img_size, img_size);

        // Bench: update camera with trans, since we dont get updated by a service here
        camera.update(&GlobalTransform {
            translation: vec3(0.0, 40.0, 100.0),
            rotation: Quat::from_axis_angle(Vec3::X, -PI / 6.0),
            ..Default::default()
        });


        group.bench_function(name.clone(), |bencher| {
            bencher.iter(|| {
                let mut img = RgbImage::new(camera.width, camera.height);
                // TODO: this tiling doesnt work all resolutions, but its faster, so leaving it in for now            
                let grid_edge_divisions: u32 = camera.width / 8;
                for grid_x in 0..grid_edge_divisions {
                    for grid_y in 0..grid_edge_divisions {
                        for u in 0..(camera.width / grid_edge_divisions) {
                            for v in 0..(camera.height / grid_edge_divisions) {
                                // PERF: calculating an offset 2 loops up is slower than doing it in the inter loop
                                let x = (grid_x * camera.width / grid_edge_divisions) + u;
                                let y = (grid_y * camera.height / grid_edge_divisions) + v;
                                let mut ray = camera.get_ray(
                                    
                                    x as f32 / camera.width as f32,
                                    // TODO: image still reversed
                                    y as f32 / camera.height as f32,
                                );
                                let color = if let Some(hit) = ray.intersect_tlas(&tlas) {
                                    let c = vec3(hit.u, hit.v, 1.0 - (hit.u + hit.v)) * 255.0;
                                    Rgb([c.x as u8, c.y as u8, c.z as u8])
                                } else {
                                    Rgb([0, 0, 0])
                                };
                                img[(x, camera.height - 1 - y)] = color;
                            }
                        }
                    }
                }
                #[cfg(feature = "save")]
                img.save(format!("target/{}.png", name)).unwrap();

                black_box(img);
            });
        });
    }
    group.finish();
}

