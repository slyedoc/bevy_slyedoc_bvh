mod helpers;
use bevy::{prelude::*, render::camera::Camera3d, window::PresentMode};
use bevy_slyedoc_bvh::prelude::*;
use helpers::*;

// Most likely you want the Cursor Plugin in the HelpersPlugins as a reference
// This show how you could use the bvh data without using events, doesn't use TBAS
fn main() {
    App::new()
        .insert_resource(WindowDescriptor {
            present_mode: PresentMode::Fifo,
            ..default()
        })
        .add_plugins(DefaultPlugins)
        // not cursor plugin or events in this example
        //.add_plugins(HelperPlugins) 
        .add_plugin(BvhPlugin)
        .add_startup_system(setup_cameras)
        .add_startup_system(load_enviroment)
        .add_startup_system(load_cursor)
        .add_system(update_cursor)
        .run();
}

#[derive(Component)]
pub struct Cursor;

fn setup_cameras(
    mut commands: Commands,
) {
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


    //  Yellow target
    commands.spawn_bundle(PbrBundle {
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

fn update_cursor(
    windows: Res<Windows>,
    camera_query: Query<(&GlobalTransform, &Camera), With<Camera3d>>,
    mut cursor: Query<(&mut Transform, &mut Visibility), With<Cursor>>,
    query: Query<(Entity, &GlobalTransform, &Tris, &InvTrans, &Aabb, &BvhHandle)>,
    bvh_vec: Res<BvhVec>
) {
    let mut target_entity = None;
    if let Some(window) = windows.get_primary() {
        if let Some(mouse_pos) = window.cursor_position() {
            if let Ok((trans, cam)) = camera_query.get_single() {
                let mut ray = Ray::from_screenspace(mouse_pos, window, cam, trans);
                let mut t = ray.t;

                for (e, _trans, tris, inv_trans, bounds, bvh_handle) in query.iter() {
                    if ray.intersect_aabb(bounds.bmin, bounds.bmax) != 1e30f32 {
                        let bvh = bvh_vec.get(bvh_handle);
                        bvh.intersect(&mut ray, &tris.0, inv_trans );
                    }
                    // TODO: just have interset return the closest intersection
                    // We got closer, update target
                    if t != ray.t {
                        target_entity = Some((e, ray));
                        t = ray.t;
                    }
                }
            }
        }
    }

    // Handle hit
    if let Ok((mut trans, mut vis)) = cursor.get_single_mut() {
        if let Some((_e, ray)) = target_entity {
            // e is the etity we hit, if we need to so something with it, send event or set resource
            trans.translation = ray.origin + (ray.direction * ray.t);
            vis.is_visible = true;
        } else {
            vis.is_visible = false;
        }

    }
}