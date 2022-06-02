use bevy::prelude::*;
use criterion::{criterion_group, criterion_main, Criterion};

criterion_group!(
    benches,
    bvh_setup,
);
criterion_main!(benches);

fn bvh_setup(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("world_get");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(4));

    // for entity_count in RANGE.map(|i| i * 10_000) {
    group.bench_function("bvh setup", |bencher| {
            let _app = World::new();
              
    //         bencher.iter(|| {
    //             for i in 0..entity_count {
    //                 let entity = Entity::from_raw(i);
    //                 assert!(world.get::<Table>(entity).is_some());
    //             }
    //         });
    //     });
    //     group.bench_function(format!("{}_entities_sparse", entity_count), |bencher| {
    //         let world = setup::<Sparse>(entity_count);

    //         bencher.iter(|| {
    //             for i in 0..entity_count {
    //                 let entity = Entity::from_raw(i);
    //                 assert!(world.get::<Sparse>(entity).is_some());
    //             }
    //         });
    //     });
    });

    group.finish();
}

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
