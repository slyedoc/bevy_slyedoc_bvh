#![allow(warnings)]
mod helpers;
use std::f32::consts::PI;

use bevy::{math::vec3, prelude::*, window::PresentMode};
use bevy_prototype_debug_lines::{DebugLines, DebugLinesPlugin};
use bevy_slyedoc_bvh::prelude::*;
use helpers::*;

fn main() {
    App::new()
        .insert_resource(WindowDescriptor {
            present_mode: PresentMode::Fifo,
            ..default()
        })
        .add_plugins(DefaultPlugins)
        .add_plugin(HelperPlugin) // See cusor plugin in helper plugins
        .add_plugin(BvhPlugin)
        //.add_plugin(DebugLinesPlugin::default())
        .add_startup_system(helpers::setup_cameras)
        .add_startup_system(helpers::load_enviroment)
        //.add_startup_system(helpers::load_sponza)
        .add_startup_system(load_test)
        //.add_system(camera_gizmo)
        .run();
}

#[allow(dead_code)]
pub fn camera_gizmo(
    mut commands: Commands,
    mut lines: ResMut<DebugLines>,
    camera_query: Query<(&BvhCamera)>,
) {
    if let Ok(camera) = camera_query.get_single() {
        let start = camera.origin;
        let duration = 0.0;

        // Draw frustum lines
        Ray::default();
        for i in 0..4 {
            let u = if i % 2 == 0 { 0.0 } else { 1.0 };
            let v = if i < 2 { 0.0 } else { 1.0 };
            let mut ray = camera.get_ray(u, v);
            let end = camera.origin + (ray.direction * ray.distance);
            lines.line(start, end, duration);
        }
    }
}

fn load_test(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mesh_complexity = 3;
    for (position, size, complexity, color) in [
        (vec3(-3.0, 1.0, 0.0), 2.0, 12, Color::YELLOW),
        (vec3(3.0, 1.0, 0.0), 2.0, 12, Color::BLUE),
    ] {
        commands
            .spawn_bundle(PbrBundle {
                transform: Transform::from_translation(position),
                mesh: meshes.add(Mesh::from(shape::UVSphere {
                    radius: size,
                    sectors: complexity,
                    stacks: complexity,
                })),
                material: materials.add(StandardMaterial {
                    base_color: color,
                    ..default()
                }),
                ..default()
            })
            .insert(BvhInit)
            .insert(Name::new("Target"));
    }
}
