use std::f32::consts::PI;

use bevy::{
    math::{vec3, Quat, Vec3},
    prelude::{Entity, GlobalTransform},
};
use bevy_slyedoc_bvh::prelude::*;
use image::{Rgb, RgbImage};
use rand::SeedableRng;
use rand_chacha::ChaChaRng;



#[cfg(feature = "trace")]
use tracing::{info_span};
#[cfg(feature = "trace")]
use tracing_chrome::ChromeLayerBuilder;
#[cfg(feature = "trace")]
use tracing_subscriber::prelude::*;

// Test for figuring out how to setup a bench without alot of bevy
fn main() {
    #[cfg(feature = "trace")]
    let (chrome_layer, _guard) = ChromeLayerBuilder::new().build();
    #[cfg(feature = "trace")]
    tracing_subscriber::registry().with(chrome_layer).init();
    

    let mut rng = ChaChaRng::seed_from_u64(0);
    let mut tlas = Tlas::default();

    // create a scene
    let side_count = 10; // x*x blas
    let triangle_count = 1000;

    let offset = 12.0;
    let side_offset = side_count as f32 * offset * 0.5;

    {
        #[cfg(feature = "trace")]
        let _span = info_span!("build scene").entered();
        for i in 0..side_count {
            for j in 0..side_count {
                let id = i * side_count + j;
                let tris = gen_random_triangles(triangle_count, &mut rng);
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
    }

    println!("BvhNode size: {}", std::mem::size_of::<BvhNode>());
    println!("TlasNode size: {}", std::mem::size_of::<TlasNode>());
    println!("Ray size: {}", std::mem::size_of::<Ray>());

    {
        #[cfg(feature = "trace")]
        let _span = info_span!("build tlas").entered();
        // Bench: Build the tlas, since we dont get updated by a service here
        tlas.build();
    }

    {
        for size in [256, 2048] {
            #[cfg(feature = "trace")]
            let _span = info_span!("render image").entered();
            let mut camera = BvhCamera::new(size, size);

            // Bench: update camera with trans, since we dont get updated by a service here
            camera.update(&GlobalTransform {
                translation: vec3(0.0, 40.0, 100.0),
                rotation: Quat::from_axis_angle(Vec3::X, -PI / 6.0),
                ..Default::default()
            });

            let mut img = RgbImage::new(camera.width, camera.height);
            let mut ray = Ray::default();
            for y in 0..camera.height {
                for x in 0..camera.width {
                    #[cfg(feature = "trace")]
                    let _span = info_span!("pixel").entered();
                    camera.set_ray(
                        &mut ray,
                        x as f32 / camera.width as f32,
                        // TODO: image still reversed
                        1.0 - (y as f32 / camera.height as f32),
                    );
                    let color = if let Some(hit) = ray.intersect_tlas(&tlas) {
                        let c = vec3(hit.u, hit.v, 1.0 - (hit.u + hit.v)) * 255.0;
                        Rgb([c.x as u8, c.y as u8, c.z as u8])
                    } else {
                        Rgb([0, 0, 0])
                    };

                    img[(x, y)] = color;
                }
            }

            img.save(format!("out/img_{}x{}.png", size, size))
                .unwrap();
        }
    }
}
