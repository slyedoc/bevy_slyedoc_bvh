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
        .add_startup_system(load_cursor)
        .add_system(fire_cursor_ray.before(BvhPlugin::run_raycasts))
        .add_system(handle_cursor_ray.after(BvhPlugin::run_raycasts))
        .run();
}

#[derive(Component)]
pub struct Cursor;

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
    .insert(Name::new("Target"));


}

fn load_cursor(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands
        .spawn_bundle(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::UVSphere {
                radius: 0.1,
                sectors: 12,
                stacks: 12,
            })),
            material: materials.add(StandardMaterial {
                base_color: Color::RED,
                //unlit: true,
                ..default()
            }),
            visibility: Visibility { is_visible: false },
            ..default()
        })
        .insert(Cursor)
        .insert(Name::new("Cursor"));
}

fn load_clock_tower(mut commands: Commands, asset_server: ResMut<AssetServer>) {
    let clock = asset_server.load("models/clock-tower/scene.glb#Scene0");
    commands
        .spawn_bundle(TransformBundle {
            local: Transform::from_xyz(0.0, 5.0, -10.0).with_scale(Vec3::splat(0.001)), // scale it down so we can see it
            global: GlobalTransform::identity(),
        })
        .with_children(|parent| {
            parent.spawn_scene(clock);
        })
        .insert(BvhInit)
        .insert(Name::new("Clock Tower"));
}

fn fire_cursor_ray(
    windows: Res<Windows>,
    camera_query: Query<(&GlobalTransform, &Camera), With<Camera3d>>,
    mut raycasts: EventWriter<Raycast>
) {
    if let Some(window) = windows.get_primary() {
        if let Some(mouse_pos) = window.cursor_position() {
            if let Ok((trans, cam)) = camera_query.get_single() {
                raycasts.send(Raycast(Ray::from_screenspace(mouse_pos, window, cam, trans)))
            }
        }
    }
}

fn handle_cursor_ray(
    mut raycast_result: EventReader<RaycastResult>,
    mut cursor: Query<(&mut Transform, &mut Visibility), With<Cursor>>,
) {
    // Handle hit
    for hit in raycast_result.iter() {
        let (mut cursor_trans, mut cursor_vis) = cursor.single_mut();
        if let Some(_e) = hit.entity {   
            cursor_trans.translation = hit.world_position;
            cursor_vis.is_visible = true;
        }
        else {
            cursor_vis.is_visible = false;
        }
    }
}
