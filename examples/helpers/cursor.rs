use bevy::{prelude::*, render::camera::Camera3d};
use bevy_slyedoc_bvh::{
    prelude::*,
    BvhSetup, 
};

pub struct CursorPlugin;

impl Plugin for CursorPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(load_cursor)
            .add_system_to_stage(CoreStage::PostUpdate, move_cursor.after(BvhSetup));
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
                base_color: Color::rgba(1.0, 0.0, 0.0, 0.5),
                unlit: true,
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

                // test ray agaist tlas
                ray.intersect_tlas(&tlas);

                // see if we hit
                if let Some(_e) = ray.entity {
                    // we could do something with the entity here
                    cursor_trans.translation = ray.origin + ray.direction * ray.t;
                    cursor_vis.is_visible = true;
                } else {
                    cursor_vis.is_visible = false;
                }
            }
        }
    }
}
