use bevy::{prelude::*, render::camera::Camera3d};
use bevy_slyedoc_bvh::prelude::*;
pub struct CursorPlugin;

impl Plugin for CursorPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(load_cursor)
            .add_system_to_stage(CoreStage::PostUpdate, move_cursor.after(BvhSystems::Setup));
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
                sectors: 30,
                stacks: 30,
            })),
            material: materials.add(StandardMaterial {
                base_color: Color::rgba(1.0, 0.0, 0.0, 0.2),
                unlit: true,
                alpha_mode: AlphaMode::Blend,
                ..default()
            }),
            visibility: Visibility { is_visible: false },
            ..default()
        })
        .insert(Cursor)
        .insert(Name::new("Cursor"));
}

fn move_cursor(
    windows: Res<Windows>,
    camera_query: Query<(&GlobalTransform, &Camera), With<Camera3d>>,
    mut cusror_query: Query<(&mut Transform, &mut Visibility), With<Cursor>>,
    tlas: Res<Tlas>,
) {
    if let Some(window) = windows.get_primary() {
        if let Some(mouse_pos) = window.cursor_position() {
            if let Ok((trans, cam)) = camera_query.get_single() {
                // get the cursor
                let (mut cursor_trans, mut cursor_vis) = cusror_query.single_mut();

                // create a ray
                let mut ray = Ray::from_screenspace(mouse_pos, window, cam, trans);

                // test ray agaist tlas and see if we hit
                if let Some(hit) = ray.intersect_tlas(&tlas) {
                    // we could do something with the entity here
                    cursor_trans.translation = ray.origin + ray.direction * hit.distance;
                    cursor_vis.is_visible = true;
                } else {
                    cursor_vis.is_visible = false;
                }
            }
        }
    }
}
