mod camera_controller;
mod cursor;
mod exit;
mod overlay;

use bevy::prelude::*;
//use bevy_inspector_egui::{WorldInspectorParams, WorldInspectorPlugin};
use bevy_slyedoc_bvh::{BvhInit, BvhInitWithChildren};
pub use camera_controller::*;
pub use cursor::*;
pub use exit::*;
pub use overlay::*;


pub struct HelperPlugin;

impl Plugin for HelperPlugin {
    fn build(&self, app: &mut App) {
        app
            // .insert_resource(WorldInspectorParams {
            //     enabled: false,
            //     ..Default::default()
            // })
            // .add_plugin(WorldInspectorPlugin::new())
            // Quality of life plugins
            .add_plugin(CameraControllerPlugin)
            .add_plugin(OverlayPlugin)
            .add_plugin(ExitPlugin)
            // Simple 3d cursor to test bvh
            .add_plugin(CursorPlugin);
            //.add_system(HelperPlugin::toggle_inspector);
    }
}

// impl HelperPlugin {
//     fn toggle_inspector(
//         input: ResMut<Input<KeyCode>>,
//         mut window_params: ResMut<WorldInspectorParams>,
//     ) {
//         if input.just_pressed(KeyCode::Grave) {
//             window_params.enabled = !window_params.enabled
//         }
//     }
// }

// Adding a few system here to make reusing them easy

#[allow(dead_code)]
pub fn load_enviroment(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // light
    commands.spawn_bundle(DirectionalLightBundle {
        transform: Transform::from_xyz(50.0, 50.0, 50.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    //ground
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
        .insert(Name::new("Ground"))
        // This Marker will have our mesh added
        .insert(BvhInit);
}

#[allow(dead_code)]
pub fn load_clock_tower(mut commands: Commands, asset_server: ResMut<AssetServer>) {
    let clock = asset_server.load("models/clock-tower/scene.glb#Scene0");
    commands
        .spawn_bundle(SceneBundle {
            
            transform: Transform::from_xyz(0.0, 4.0, -10.0)
                .with_scale(Vec3::splat(0.001)), // scale it down so we can see it
            scene: clock.clone(),
            ..default()
        })
        .insert(Name::new("Clock Tower"))
        // This marker tells the BVH system to build nested children
        // for this entity, the handle is used to wait till asset is loaded
        .insert(BvhInitWithChildren(clock));
}

#[allow(dead_code)]
pub fn load_sponza(mut commands: Commands, asset_server: ResMut<AssetServer>) {
    let scene = asset_server.load("models/sponza/sponza.gltf#Scene0");
    commands
        .spawn_bundle(SceneBundle {
            transform: Transform::from_xyz(0.0, 1.0, 0.0),            
            scene: scene.clone(),
            ..default()
        })
        .insert(Name::new("Clock Tower"))
        // This marker tells the BVH system to build nested children
        // for this entity, the handle is used to wait till asset is loaded
        .insert(BvhInitWithChildren(scene));
}

#[allow(dead_code)]
pub fn setup_cameras(mut commands: Commands) {
    //commands.spawn_bundle(UiCameraBundle::default());
    commands
        .spawn_bundle(Camera3dBundle {
            transform: Transform::from_xyz(0.0, 2.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        })
        .insert(CameraController::default());
        //.insert(BvhCamera::new(1024, 1024));
}
