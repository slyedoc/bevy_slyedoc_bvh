use bevy::{
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin},
    prelude::*,
};
use bevy_slyedoc_bvh::BvhStats;

pub struct OverlayPlugin;

impl Plugin for OverlayPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(FrameTimeDiagnosticsPlugin::default())
            .add_startup_system(setup_overlay)
            .add_system(update_fps)
            .add_system(update_bvh_tri_count)
            .add_system(update_render_time)
            .add_system(update_ray_count);
    }
}

#[derive(Component)]
struct FpsText;

#[derive(Component)]
struct TriCountText;

#[derive(Component)]
struct RenderTimeText;

#[derive(Component)]
struct RayCountText;

const UI_SIZE: f32 = 30.0;
fn setup_overlay(mut commands: Commands, asset_server: ResMut<AssetServer>) {
    let ui_font = asset_server.load("fonts/FiraSans-Bold.ttf");

    commands
        .spawn_bundle(TextBundle {
            style: Style {
                align_self: AlignSelf::FlexEnd,
                position_type: PositionType::Absolute,
                position: Rect {
                    bottom: Val::Px(10.0),
                    right: Val::Px(10.0),
                    ..Default::default()
                },

                ..Default::default()
            },
            text: Text {
                sections: vec![
                    TextSection {
                        value: "Tri Count: ".to_string(),
                        style: TextStyle {
                            font: ui_font.clone(),
                            font_size: UI_SIZE,
                            color: Color::WHITE,
                        },
                    },
                    TextSection {
                        value: "".to_string(),
                        style: TextStyle {
                            font: ui_font.clone(),
                            font_size: UI_SIZE,
                            color: Color::GOLD,
                        },
                    },
                ],
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Name::new("ui Tri Count"))
        .insert(TriCountText);

    commands
        .spawn_bundle(TextBundle {
            style: Style {
                align_self: AlignSelf::FlexStart,
                position_type: PositionType::Absolute,
                position: Rect {
                    bottom: Val::Px(50.0),
                    left: Val::Px(10.0),
                    ..Default::default()
                },

                ..Default::default()
            },
            text: Text {
                sections: vec![
                    TextSection {
                        value: "BVH Render ".to_string(),
                        style: TextStyle {
                            font: ui_font.clone(),
                            font_size: UI_SIZE,
                            color: Color::WHITE,
                        },
                    },
                    TextSection {
                        value: "".to_string(),
                        style: TextStyle {
                            font: ui_font.clone(),
                            font_size: UI_SIZE,
                            color: Color::GOLD,
                        },
                    },
                ],
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Name::new("ui Render Time"))
        .insert(RenderTimeText);


        commands
        .spawn_bundle(TextBundle {
            style: Style {
                align_self: AlignSelf::FlexStart,
                position_type: PositionType::Absolute,
                position: Rect {
                    bottom: Val::Px(100.0),
                    left: Val::Px(10.0),
                    ..Default::default()
                },

                ..Default::default()
            },
            text: Text {
                sections: vec![
                    TextSection {
                        value: "Rays ".to_string(),
                        style: TextStyle {
                            font: ui_font.clone(),
                            font_size: UI_SIZE,
                            color: Color::WHITE,
                        },
                    },
                    TextSection {
                        value: "".to_string(),
                        style: TextStyle {
                            font: ui_font.clone(),
                            font_size: UI_SIZE,
                            color: Color::GOLD,
                        },
                    },
                ],
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Name::new("ui Ray Count"))
        .insert(RayCountText);

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
        .insert(Name::new("ui FPS"))
        .insert(FpsText);
}

fn update_fps(diagnostics: Res<Diagnostics>, mut query: Query<&mut Text, With<FpsText>>) {
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

fn update_bvh_tri_count(mut query: Query<&mut Text, With<TriCountText>>, stats: Res<BvhStats>) {
    for mut text in query.iter_mut() {
        // Update the value of the second section
        text.sections[1].value = stats.tri_count.to_string();
    }
}

fn update_render_time(mut query: Query<&mut Text, With<RenderTimeText>>, stats: Res<BvhStats>) {
    for mut text in query.iter_mut() {
        // Update the value of the second section
        text.sections[1].value = format!("{:.2} ms", stats.camera_time.as_millis());
    }
}

fn update_ray_count(mut query: Query<&mut Text, With<RayCountText>>, stats: Res<BvhStats>) {
    for mut text in query.iter_mut() {
        // Update the value of the second section
        text.sections[1].value = format!("{:.1} Mps", stats.ray_count as f32 / stats.camera_time.as_micros() as f32);
    }
}

