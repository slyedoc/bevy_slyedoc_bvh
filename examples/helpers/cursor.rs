use bevy::{prelude::*, render::camera::Camera3d};
use bevy_slyedoc_bvh::{Raycast, prelude::Ray, RaycastResult};

pub struct CursorPlugin;

impl Plugin for CursorPlugin {
    fn build(&self, app: &mut App) {
        app
        .add_startup_system(load_cursor)
        .add_system_to_stage(CoreStage::PostUpdate, fire_cursor_ray.before(crate::BvhPlugin::run_raycasts))
        .add_system_to_stage(CoreStage::PostUpdate, handle_cursor_ray.after(crate::BvhPlugin::run_raycasts));
    }
}

#[derive(Component)]
pub struct Cursor;

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