use std::f32::consts::PI;

use bevy::{
    math::{vec3, Quat, Vec3},
    prelude::{GlobalTransform},
};
use bvh::prelude::*;
use image::{Rgb, RgbImage};


#[cfg(feature = "trace")]
use tracing::info_span;
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

    println!("BvhNode size: {}", std::mem::size_of::<BvhNode>());
    println!("TlasNode size: {}", std::mem::size_of::<TlasNode>());
    println!("Ray size: {}", std::mem::size_of::<Ray>());

    #[cfg(feature = "trace")]
    let build_span = info_span!("build tlas").entered();
    let build_time = std::time::Instant::now();
    let tlas = build_random_tri_scene();
    println!("Tlas build time: {:?}", build_time.elapsed());
    #[cfg(feature = "trace")]
    build_span.exit();

    
    for size in [256, 512] {
        let render_time = std::time::Instant::now();
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
        for y in 0..camera.height {
            for x in 0..camera.width {
                let mut ray = camera.get_ray(
                    x as f32 / camera.width as f32,                    
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
        println!("Render time {}x{}: {:?}", camera.width, camera.height, render_time.elapsed());

        img.save(format!("out/img_{}x{}.png", size, size)).unwrap();
    }
}
