
mod camera_controller;
mod exit;
mod overlay;
mod cursor;

use bevy::{app::PluginGroupBuilder, prelude::PluginGroup};
use bevy_inspector_egui::WorldInspectorPlugin;
pub use camera_controller::*;
pub use exit::*;
pub use overlay::*;
pub use cursor::*;

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
