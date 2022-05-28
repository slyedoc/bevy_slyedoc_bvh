mod helpers;
use bevy::{prelude::*, window::PresentMode};
use bevy_slyedoc_bvh::prelude::*;
use helpers::*;

fn main() {
    App::new()
        .insert_resource(WindowDescriptor {
            present_mode: PresentMode::Fifo,
            ..default()
        })
        .add_plugins(DefaultPlugins)
        .add_plugins(HelperPlugins)
        .add_plugin(BvhPlugin)
        .add_startup_system(setup_cameras)
        .add_startup_system(load_enviroment)
        .run();
}



fn setup_cameras(mut commands: Commands) {
    // For any UI we need
    commands.spawn_bundle(UiCameraBundle::default());
    // For our 3d scene we need
    // Notice we dont need ray casting sources
    commands
        .spawn_bundle(PerspectiveCameraBundle {
            transform: Transform::from_xyz(0.0, 2.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        })
        // TODO: clean up this controller, I use it enought but its has issues with init
        .insert(CameraController::default());
}

fn load_enviroment(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // light
    commands.spawn_bundle(DirectionalLightBundle {
        transform: Transform::from_xyz(10.0, 10.0, 3.0),
        ..default()
    });

    // ground
    commands
        .spawn_bundle(PbrBundle {
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            mesh: meshes.add(Mesh::from(shape::Plane { size: 100.0 })),
            material: materials.add(StandardMaterial {
                base_color: Color::DARK_GREEN,
                ..default()
            }),
            ..default()
        })
        .insert(BvhInit)
        .insert(Name::new("Target"));

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
                //unlit: true,
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
                //unlit: true,
                ..default()
            }),
            ..default()
        })
        .insert(BvhInit)
        .insert(Name::new("Blue Target"));
}