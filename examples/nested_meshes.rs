mod helpers;
use bevy::{prelude::*, render::camera::Camera3d, window::PresentMode};
use bevy_slyedoc_bvh::prelude::*;
use helpers::*;


// TODO: This isn't implemented yet, will come back to this once remove the Tris object
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
        .add_startup_system(load_clock_tower)
        .run();
}



fn setup_cameras(
    mut commands: Commands,
) {
    // For any UI we need
    commands.spawn_bundle(UiCameraBundle::default());
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
    commands.spawn_bundle(PbrBundle {
        transform: Transform::from_xyz(0.0, 0.0, 0.0),
        mesh: meshes.add(Mesh::from(shape::Plane { size: 100.0 })),
        material: materials.add(StandardMaterial {
            base_color: Color::DARK_GREEN,
            ..default()
        }),
        ..default()
    })
    .insert(BvhInit)
    .insert(Name::new("Ground"));
}



fn load_clock_tower(mut commands: Commands, asset_server: ResMut<AssetServer>) {
    let clock = asset_server.load("models/clock-tower/scene.glb#Scene0");
    commands
        .spawn_bundle(TransformBundle {
            local: Transform::from_xyz(0.0, 5.0, -10.0).with_scale(Vec3::splat(0.001)), // scale it down so we can see it
            global: GlobalTransform::identity(),
        })
        .with_children(|parent| {
            parent.spawn_scene(clock.clone());
        })
        .insert(BvhInitWithChildren(clock))
        .insert(Name::new("Clock Tower"));
}


