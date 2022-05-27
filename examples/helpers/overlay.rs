use bevy::{
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin},
    prelude::*,
};

pub struct OverlayPlugin;

impl Plugin for OverlayPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(FrameTimeDiagnosticsPlugin::default())
            .add_startup_system(setup_overlay)
            .add_system(update_fps);
    }
}

#[derive(Component)]
struct FpsText;

#[derive(Component)]
struct GameStateText;

const UI_SIZE: f32 = 30.0;
fn setup_overlay(
    mut commands: Commands,
    asset_server: ResMut<AssetServer>,
) {
    let ui_font = asset_server.load("fonts/FiraSans-Bold.ttf");

    // commands
    //     .spawn_bundle(TextBundle {
    //         style: Style {
    //             align_self: AlignSelf::FlexEnd,
    //             position_type: PositionType::Absolute,
    //             position: Rect {
    //                 bottom: Val::Px(10.0),
    //                 right: Val::Px(10.0),
    //                 ..Default::default()
    //             },

    //             ..Default::default()
    //         },
    //         text: Text {
    //             sections: vec![
    //                 TextSection {
    //                     value: "Game State: ".to_string(),
    //                     style: TextStyle {
    //                         font: ui_font,
    //                         font_size: UI_SIZE,
    //                         color: Color::WHITE,
    //                     },
    //                 },
    //                 TextSection {
    //                     value: "".to_string(),
    //                     style: TextStyle {
    //                         font: ui_font,
    //                         font_size: UI_SIZE,
    //                         color: Color::GOLD,
    //                     },
    //                 },
    //             ],
    //             ..Default::default()
    //         },
    //         ..Default::default()
    //     })
    //     .insert(Name::new("GameState"))
    //     .insert(GameStateText);

    // Rich text with multiple sections
    commands
        .spawn_bundle(TextBundle {
            style: Style {
                position_type: PositionType::Absolute,
                position: Rect {
                    left: Val::Px(10.0),
                    bottom: Val::Px(10.0),
                    ..Default::default()
                },
                align_self: AlignSelf::FlexEnd,
                ..Default::default()
            },
            // Use `Text` directly
            text: Text {
                // Construct a `Vec` of `TextSection`s
                sections: vec![
                    TextSection {
                        value: "FPS: ".to_string(),
                        style: TextStyle {
                            font: ui_font.clone(),
                            font_size: UI_SIZE,
                            color: Color::WHITE,
                        },
                    },
                    TextSection {
                        value: "".to_string(),
                        style: TextStyle {
                            font: ui_font,
                            font_size: UI_SIZE,
                            color: Color::GOLD,
                        },
                    },
                ],
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Name::new("FPS"))
        .insert(FpsText);
}

fn update_fps(
    diagnostics: Res<Diagnostics>,
    mut query: Query<&mut Text, With<FpsText>>,
) {
    for mut text in query.iter_mut() {
        
        if let Some(fps) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
            if let Some(average) = fps.average() {
                // Update the value of the second section
                text.sections[1].value = format!("{:.0}", average);
                text.sections[1].style.color = match average {
                    x if x >= 50.0 => Color::GREEN,
                    x if x > 40.0 && x < 50.0 => Color::YELLOW,
                    x if x <= 40.0 => Color::RED,
                    _ => Color::WHITE,
                };
            }
        }
    }
}
