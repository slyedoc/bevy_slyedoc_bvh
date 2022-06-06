#![allow(clippy::type_complexity)]
use bevy::{
    input::{mouse::MouseMotion, Input},
    prelude::*,
    window::Windows,
};
pub struct CameraControllerPlugin;

impl Plugin for CameraControllerPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system_to_stage( CoreStage::PostUpdate, setup_camera_controller)
            .add_system_to_stage( CoreStage::PostUpdate, update_camera_controller.after(setup_camera_controller));
    }
}

#[derive(Component)]
pub struct CameraController {
    pub enabled: bool,
    pub sensitivity: f32,
    pub key_forward: KeyCode,
    pub key_back: KeyCode,
    pub key_left: KeyCode,
    pub key_right: KeyCode,
    pub key_up: KeyCode,
    pub key_down: KeyCode,
    pub key_run: KeyCode,
    pub mouse_look: MouseButton,
    pub walk_speed: f32,
    pub run_speed: f32,
    pub friction: f32,
    pub pitch: f32,
    pub yaw: f32,
    pub velocity: Vec3,
    pub position_smoothness: f32,
    pub rotation_smoothness: f32,
}

impl Default for CameraController {
    fn default() -> Self {
        Self {
            enabled: true,
            sensitivity: 0.5,
            key_forward: KeyCode::W,
            key_back: KeyCode::S,
            key_left: KeyCode::A,
            key_right: KeyCode::D,
            key_up: KeyCode::E,
            key_down: KeyCode::Q,
            key_run: KeyCode::LShift,
            mouse_look: MouseButton::Right,
            walk_speed: 10.0,
            run_speed: 30.0,
            friction: 0.3,
            pitch: 0.0,
            yaw:  0.0,
            velocity: Vec3::ZERO,
            position_smoothness: 1.0,
            rotation_smoothness: 100.0,
        }
    }
}

fn setup_camera_controller(
    mut query: Query<(&Transform, &mut CameraController), (Added<CameraController>, With<Camera>)>,
) {
    for (transform, mut controller) in query.iter_mut() {
        // TODO: pretty sure controller uses of pitch and yaw is flipped
        let (pitch, yaw, _roll) = yaw_pitch_roll(transform.rotation);
        controller.pitch = pitch;
        controller.yaw = yaw;
    }
}

fn update_camera_controller(
    time: Res<Time>,
    mut mouse_events: EventReader<MouseMotion>,
    key_input: Res<Input<KeyCode>>,
    mouse_input: Res<Input<MouseButton>>,
    mut query: Query<(&mut Transform, &mut CameraController), With<Camera>>,
    mut windows: ResMut<Windows>,
) {
    let dt = time.delta_seconds();
    if let Some(window) = windows.get_primary_mut() {
        for (mut transform, mut controller) in query.iter_mut() {
            if !controller.enabled {
                continue;
            }
    
            // Handle key input
            let mut axis_input = Vec3::ZERO;
            if key_input.pressed(controller.key_forward) {
                axis_input.z += 1.0;
            }
            if key_input.pressed(controller.key_back) {
                axis_input.z -= 1.0;
            }
            if key_input.pressed(controller.key_right) {
                axis_input.x += 1.0;
            }
            if key_input.pressed(controller.key_left) {
                axis_input.x -= 1.0;
            }
            if key_input.pressed(controller.key_up) {
                axis_input.y += 1.0;
            }
            if key_input.pressed(controller.key_down) {
                axis_input.y -= 1.0;
            }
    
            // Apply movement update
            if axis_input != Vec3::ZERO {
                let max_speed = if key_input.pressed(controller.key_run) {
                    controller.run_speed
                } else {
                    controller.walk_speed
                };
                controller.velocity = axis_input.normalize() * max_speed;
            } else {
                let friction = controller.friction.clamp(0.0, 1.0);
                controller.velocity *= 1.0 - friction;
                if controller.velocity.length_squared() < 1e-6 {
                    controller.velocity = Vec3::ZERO;
                }
            }
            let forward = transform.forward();
            let right = transform.right();
            transform.translation += controller.velocity.x * dt * right
                + controller.velocity.y * dt * Vec3::Y
                + controller.velocity.z * dt * forward;
    
            // Handle mouse look on mouse button
            let mut mouse_delta = Vec2::ZERO;
            if mouse_input.pressed(controller.mouse_look) {
                #[cfg(not(target="wasm32"))]
                window.set_cursor_lock_mode(true);
                window.set_cursor_visibility(false);
            }
            if mouse_input.just_released(controller.mouse_look) {
                #[cfg(not(target="wasm32"))]
                window.set_cursor_lock_mode(false);
                window.set_cursor_visibility(true);
            }
            if mouse_input.pressed(controller.mouse_look) {
                for mouse_event in mouse_events.iter() {
                    mouse_delta += mouse_event.delta;
                }
            }
    
            if mouse_delta != Vec2::ZERO {
                // Apply look update
                let (pitch, yaw) = (
                    (controller.pitch - mouse_delta.y * 0.5 * controller.sensitivity * dt).clamp(
                        -0.99 * std::f32::consts::FRAC_PI_2,
                        0.99 * std::f32::consts::FRAC_PI_2,
                    ),
                    controller.yaw - mouse_delta.x * controller.sensitivity * dt,
                );
    
                // Apply smoothing, code needs improvement
                let target = Quat::from_euler(EulerRot::ZYX, 0.0, yaw, pitch);
                transform.rotation = transform.rotation.lerp(target, 0.5);
    
                controller.pitch = pitch;
                controller.yaw = yaw;
            }
        }
    }

    
}

// from https://docs.rs/bevy-inspector-egui/0.6.1/src/bevy_inspector_egui/impls/quat.rs.html
#[allow(clippy::many_single_char_names)]
fn yaw_pitch_roll(q: Quat) -> (f32, f32, f32) {
    let [x, y, z, w] = *q.as_ref();

    let yaw = (2.0 * (y * z + w * x)).atan2( w * w - x * x - y * y + z * z);
    let pitch = (-2.0 * (x * z - w * y)).asin();
    let roll = (2.0 * (x * y + w * z)).atan2( w * w + x * x - y * y - z * z);

    (yaw, pitch, roll)
}
