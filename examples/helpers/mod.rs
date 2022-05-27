use bevy::{app::PluginGroupBuilder, prelude::PluginGroup};
mod camera_controller;
mod exit;
mod overlay;
use bevy_inspector_egui::WorldInspectorPlugin;
pub use camera_controller::*;
pub use exit::*;
pub use overlay::*;

pub struct HelperPlugins;

impl PluginGroup for HelperPlugins {
    fn build(&mut self, group: &mut PluginGroupBuilder) {
        group.add(WorldInspectorPlugin::new());
        // Local
        group.add(CameraControllerPlugin);
        group.add(OverlayPlugin);
        group.add(ExitPlugin);
    }
}
