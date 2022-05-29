mod helpers;
use bevy::{prelude::*, window::PresentMode};
use bevy_slyedoc_bvh::prelude::*;
use helpers::*;

// Example using BvhInitWithChildren for a scene load
fn main() {
    App::new()
        .insert_resource(WindowDescriptor {
            present_mode: PresentMode::Fifo,
            ..default()
        })
        .add_plugins(DefaultPlugins)
        .add_plugins(HelperPlugins) // See cusor plugin in helper plugins
        .add_plugin(BvhPlugin)
        .add_startup_system(helpers::setup_cameras)
        .add_startup_system(helpers::load_enviroment)
        .add_startup_system(helpers::load_clock_tower) // Check this function out 
        .run();
}





