
use bevy::{
    prelude::*,
    window::{PresentMode, WindowResized},
};
use bvh::prelude::*;
use raytrace::*;
mod helpers;

fn main() {
    App::new()
        // .insert_resource(WgpuSettings {
        //     //features: WgpuFeatures::default(),
        //     ..default()
        // })
        .insert_resource(WindowDescriptor {
            present_mode: PresentMode::Fifo,
            ..default()
        })
        .add_plugins(DefaultPlugins)
        .add_plugin(helpers::HelperPlugin) // See cusor plugin in helper plugins
        .add_plugin(BvhPlugin)
        .add_plugin(RaytracePlugin)
        .add_startup_system(helpers::setup_cameras)
        .add_startup_system(helpers::load_enviroment)
        //.add_startup_system(helpers::load_sponza)
        .add_startup_system(setup)
        .add_system(resize_sprite)
        .run();
}

// Marker for Sprite for resize
#[derive(Component)]
struct RtSprite;

fn setup(
    mut commands: Commands,
    window: Res<WindowDescriptor>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut rt_materials: ResMut<Assets<RtMaterial>>,
    mut asset_server: ResMut<AssetServer>,
) {
    let size = UVec2::new(window.width as u32, window.height as u32);

    // create the image
    commands
        .spawn_bundle(MaterialMeshBundle::<RtMaterial> {
            transform: Transform::from_xyz(0.0, 1.0, 0.0),
            mesh: meshes.add(Mesh::from(shape::Cube::new(2.0))),
            material: rt_materials.add(RtMaterial {
                background_texture: asset_server.load("images/sky_19.hdr"),
            }),
            ..default()
        })
        .insert(RtSprite);
    // commands.spawn_bundle(Camera2dBundle::default());
}

fn resize_sprite(
    mut commands: Commands,    
    mut resize_event: EventReader<WindowResized>,
    mut rt_camera: ResMut<RtCamera>,
) {
    for resize in resize_event.iter() {
        //info!("resize {:?}", resize);        
        rt_camera.size = UVec2::new(resize.width as u32, resize.height as u32);
    }
}
