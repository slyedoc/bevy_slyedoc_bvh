mod camera_controller;
mod cursor;
mod exit;
mod overlay;

use bevy::{app::PluginGroupBuilder, prelude::*};
use bevy_inspector_egui::WorldInspectorPlugin;
use bevy_slyedoc_bvh::{BvhInitWithChildren, BvhInit, prelude::BvhCamera};
pub use camera_controller::*;
pub use cursor::*;
pub use exit::*;
pub use overlay::*;
pub struct HelperPlugins;

impl PluginGroup for HelperPlugins {
    fn build(&mut self, group: &mut PluginGroupBuilder) {
        // Editor
        group.add(WorldInspectorPlugin::new());

        // Quality of life plugins
        group.add(CameraControllerPlugin);
        group.add(OverlayPlugin);
        group.add(ExitPlugin);

        // Simple 3d cursor to test bvh
        group.add(CursorPlugin);
    }
}

// Adding a few system here to make reusing them easy

#[allow(dead_code)]
pub fn load_enviroment(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // light
    commands.spawn_bundle(DirectionalLightBundle {
        transform: Transform::from_xyz(10.0, 10.0, 3.0),
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
        .spawn_bundle(TransformBundle {
            // scale it down so we can see it
            local: Transform::from_xyz(0.0, 4.0, -10.0).with_scale(Vec3::splat(0.001)), 
            global: GlobalTransform::identity(),
        })
        .with_children(|parent| {
            parent.spawn_scene(clock.clone());
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
        .spawn_bundle(TransformBundle {
            local: Transform::from_xyz(0.0, 1.0, 0.0), 
            global: GlobalTransform::identity(),
        })
        .with_children(|parent| {
            parent.spawn_scene(scene.clone());
        })
        .insert(Name::new("Clock Tower"))
        // This marker tells the BVH system to build nested children
        // for this entity, the handle is used to wait till asset is loaded
        .insert(BvhInitWithChildren(scene));
}


#[allow(dead_code)]
pub fn setup_cameras(mut commands: Commands) {
    commands.spawn_bundle(UiCameraBundle::default());
    commands
        .spawn_bundle(PerspectiveCameraBundle {
            transform: Transform::from_xyz(0.0, 2.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        })
        .insert(CameraController::default())
        .insert(BvhCamera::new(512, 512));
}
