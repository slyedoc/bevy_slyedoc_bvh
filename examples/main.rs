#![allow(warnings)]
mod helpers;
use std::f32::consts::PI;

use bevy::{prelude::*, window::PresentMode};
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
        .add_plugins(HelperPlugins) // See cusor plugin in helper plugins
        .add_plugin(BvhPlugin)
        //.add_plugin(DebugLinesPlugin::default())
        .add_startup_system(helpers::setup_cameras)
        .add_startup_system(helpers::load_enviroment)
        .add_startup_system(helpers::load_sponza)
        //.add_startup_system(load_test)
        .add_system_set_to_stage(
            CoreStage::PostUpdate,
            SystemSet::new()
                .after(BvhSystems::Camera)
                .with_system(display_camera),
        )
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

pub fn display_camera(mut commands: Commands, mut camera: Query<(&BvhCamera), Added<BvhCamera>>) {
    for camera in camera.iter() {
        if let Some(image) = &camera.image {
            commands
                .spawn_bundle(ImageBundle {
                    style: Style {
                        align_self: AlignSelf::FlexEnd,
                        position_type: PositionType::Absolute,
                        position: Rect {
                            bottom: Val::Px(50.0),
                            right: Val::Px(10.0),
                            ..Default::default()
                        },
                        ..default()
                    },
                    image: image.clone().into(),
                    ..default()
                })
                .insert(Name::new("BVH Image"));
        }
    }
}

fn load_test(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands
        .spawn_bundle(PbrBundle {
            transform: Transform::from_xyz(3.0, 0.0, 0.0),
            mesh: meshes.add(Mesh::from(shape::UVSphere {
                radius: 2.0,
                sectors: 100,
                stacks: 100,
            })),
            material: materials.add(StandardMaterial {
                base_color: Color::YELLOW,
                ..default()
            }),
            ..default()
        })
        .insert(BvhInit)
        .insert(Name::new("Yellow Target"));

    commands
        .spawn_bundle(PbrBundle {
            transform: Transform::from_xyz(-3.0, 0.0, 0.0),
            mesh: meshes.add(Mesh::from(shape::UVSphere {
                radius: 2.0,
                sectors: 100,
                stacks: 100,
            })),
            material: materials.add(StandardMaterial {
                base_color: Color::BLUE,
                ..default()
            }),
            ..default()
        })
        .insert(BvhInit)
        .insert(Name::new("Blue Target"));
}
