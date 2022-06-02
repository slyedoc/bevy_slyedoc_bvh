use criterion::{criterion_group, criterion_main, Criterion};

//crate::prelude::*;

// fn criterion_benchmark(c: &mut Criterion) {
//     let tris = load_tri_file("assets/bigben.tri");

//     c.bench_function("Bvh setup", |b| {
//         b.iter(|| {
//             let _bvh = Bvh::new(&tris);
//         })
//     });

//     c.bench_function("render image", |b| {
//         let bvh = Bvh::new(&tris);
//         let mut img = RgbImage::new(640, 640);
//         let camera = BvhCamera::default();
//         b.iter(|| {
//             render_image(&camera, 640, 640, &bvh, &tris, &mut img);
//         })
//     });
// }

//criterion_group!(benches, criterion_benchmark);
//criterion_main!(benches);
