use bevy::{prelude::*, app::AppExit};

pub struct ExitPlugin;

impl Plugin for ExitPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(update_escape);
    }
}

fn update_escape(
    mut keys: ResMut<Input<KeyCode>>,
    mut app_exit: EventWriter<AppExit>,
) {
    if keys.just_pressed(KeyCode::Escape) {
       app_exit.send(AppExit);
       keys.clear()
    }
}
