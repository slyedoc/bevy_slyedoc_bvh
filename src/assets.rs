use bevy::{prelude::*, math::vec3};

use rand::Rng;
use std::fs;

use crate::Tri;

pub fn load_tri_file(file: impl Into<String>) -> Vec<Tri> {
    let filename = file.into();
    fs::read_to_string(filename)
        .expect("Something went wrong reading the file")
        .split('\n')
        .filter(|line| !line.is_empty() && !line.starts_with("999"))
        .map(|line| {
            let parts: Vec<&str> = line.split(' ').collect();

            Tri::new(
                vec3(
                    parts[0].parse::<f32>().unwrap(),
                    parts[1].parse::<f32>().unwrap(),
                    parts[2].parse::<f32>().unwrap(),
                ),
                vec3(
                    parts[3].parse::<f32>().unwrap(),
                    parts[4].parse::<f32>().unwrap(),
                    parts[5].parse::<f32>().unwrap(),
                ),
                vec3(
                    parts[6].parse::<f32>().unwrap(),
                    parts[7].parse::<f32>().unwrap(),
                    parts[8].parse::<f32>().unwrap(),
                ),
            )
        })
        .collect::<Vec<_>>()
}

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

fn random_vec3(rng: &mut impl Rng) -> Vec3 {
    vec3(
        rng.gen_range(-1.0..=1.0),
        rng.gen_range(-1.0..=1.0),
        rng.gen_range(-1.0..=1.0),
    )
}
