use bevy::{
    core_pipeline::clear_color::ClearColorConfig,
    input::touch::TouchPhase,
    prelude::*,
    render::{camera::ScalingMode, view::RenderLayers},
    sprite::MaterialMesh2dBundle,
    sprite::Mesh2dHandle,
    utils::HashMap,
};

use crate::{
    component::MainCamera,
    input::{to_u8_angle, vec_to_angle, view_to_world, PlayerInput, FIRE, MOVE},
    DebugState,
};

pub struct TouchPlugin;
impl Plugin for TouchPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(TouchMovement::default())
            .add_systems(Startup, init_virtual_joystick)
            .add_systems(
                Update,
                (
                    process_touch_events,
                    update_virtual_joystick,
                    debug_touches.run_if(in_state(DebugState::On)),
                ),
            );
    }
}

#[derive(Component)]
struct JoystickInner;

#[derive(Component)]
struct JoystickOuter;

#[derive(Component)]
struct JoystickCamera;

/// display touch controls when they're in use, updating them to the position and existence of the joystick
fn update_virtual_joystick(
    mut q_inner: Query<
        (&mut Transform, &mut Visibility),
        (With<JoystickInner>, Without<JoystickOuter>),
    >,
    mut q_outer: Query<
        (&mut Transform, &mut Visibility),
        (With<JoystickOuter>, Without<JoystickInner>),
    >,
    touches: Res<TouchMovement>,
    q_camera: Query<(&Camera, &GlobalTransform), With<JoystickCamera>>,
) {
    let (mut i_t, mut i_v) = q_inner.single_mut();
    let (mut o_t, mut o_v) = q_outer.single_mut();
    let (camera, c_transform) = q_camera.single();
    // check if a joystick exists
    match touches.stick_id {
        Some(sid) if touches.fingers.contains_key(&sid) => {
            let finger = touches.fingers.get(&sid).unwrap();
            let delta = (finger.pos - finger.start_pos).clamp_length_max(32.);
            i_t.translation =
                view_to_world(finger.start_pos + delta, camera, c_transform).extend(0.);
            o_t.translation = view_to_world(finger.start_pos, camera, c_transform).extend(0.);
            *i_v = Visibility::Visible;
            *o_v = Visibility::Visible;
        }
        // hide the virtual joystick if it doesn't
        _ => {
            *i_v = Visibility::Hidden;
            *o_v = Visibility::Hidden;
        }
    }
}

/// creates a separate render layer and camera for two joystick icons that appear when touch controls are in use
fn init_virtual_joystick(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    asset_server: Res<AssetServer>,
) {
    commands
        .spawn(JoystickInner)
        .insert(MaterialMesh2dBundle {
            mesh: Mesh2dHandle(meshes.add(shape::Quad::new(Vec2::splat(64.)).into())),
            material: materials.add(ColorMaterial {
                color: Color::WHITE,
                texture: Some(asset_server.load("joystick_inner.png")),
            }),
            ..default()
        })
        .insert(RenderLayers::layer(1));
    commands
        .spawn(JoystickOuter)
        .insert(MaterialMesh2dBundle {
            mesh: Mesh2dHandle(meshes.add(shape::Quad::new(Vec2::splat(128.)).into())),
            material: materials.add(ColorMaterial {
                color: Color::WHITE,
                texture: Some(asset_server.load("joystick_ring.png")),
            }),
            ..default()
        })
        .insert(RenderLayers::layer(1));
    // static screen-sized joystick camera
    commands
        .spawn(JoystickCamera)
        .insert(Camera2dBundle {
            projection: OrthographicProjection {
                scaling_mode: ScalingMode::WindowSize(1.0),
                ..default()
            },
            camera: Camera {
                order: 2,
                ..default()
            },
            camera_2d: Camera2d {
                clear_color: ClearColorConfig::None,
            },
            ..default()
        })
        .insert(RenderLayers::layer(1));
}

#[derive(Reflect, Debug)]
pub struct TouchFinger {
    start_pos: Vec2,
    pos: Vec2,
    tap: bool, // set to false when the finger goes past the tap distance
}

impl TouchFinger {
    fn delta(&self) -> Vec2 {
        self.pos - self.start_pos
    }

    fn dist(&self) -> f32 {
        self.pos.distance(self.start_pos)
    }
}

#[derive(Resource, Reflect, Default, Debug)]
pub struct TouchMovement {
    fingers: HashMap<u64, TouchFinger>,
    stick_id: Option<u64>,
    fire_touch: Option<Vec2>, // the position of the last touch
}

/// process touch events into the touch movement resource
fn process_touch_events(mut touch_res: ResMut<TouchMovement>, mut events: EventReader<TouchInput>) {
    const TAP_MAX_DISTANCE: f32 = 10.;

    for touch in events.iter() {
        let id = &touch.id;
        let finger: Option<&mut TouchFinger> = touch_res.fingers.get_mut(id);
        info!(
            "touch event {:?} => [{}]{:?}",
            touch.phase, id, touch.position
        );
        match touch.phase {
            TouchPhase::Started => {
                // cache fingers on entry
                touch_res.fingers.insert(
                    touch.id,
                    TouchFinger {
                        start_pos: touch.position,
                        pos: touch.position,
                        tap: true,
                    },
                );
            }
            TouchPhase::Moved => {
                // update finger positions & register joysticks
                if let Some(finger) = finger {
                    finger.pos = touch.position;

                    // if this finger is moving more than the tap distance, it starts a joystick
                    if finger.dist() >= TAP_MAX_DISTANCE {
                        finger.tap = false;
                        if touch_res.stick_id.is_none() {
                            touch_res.stick_id = Some(*id);
                        }
                    }
                }
            }
            // remove fingers and joysticks if a finger lifts from the screen
            TouchPhase::Ended | TouchPhase::Canceled => {
                // fetch finger initial position
                let finger = touch_res
                    .fingers
                    .remove(id)
                    .expect("Finger lost in transit.");
                // remove it as the joystick if it is
                if touch_res.stick_id.is_some_and(|sid| sid == *id) {
                    touch_res.stick_id = None;
                }
                // fire if it was a tap
                if finger.tap {
                    touch_res.fire_touch = Some(touch.position);
                }
            }
        }
    }
}

impl TouchMovement {
    /// called during the ggrs input system. flushes all buffered inputs and returns inputs, if any
    pub fn drain(
        &mut self,
        player_pos: Vec2,
        default_angle: u8,
        view_to_world: impl Fn(Vec2) -> Vec2,
    ) -> Option<PlayerInput> {
        let mut btn = 0u8;
        let angle: u8 = if let Some(screen_pos) = self.fire_touch {
            btn |= FIRE;
            self.fire_touch = None;
            let delta = view_to_world(screen_pos) - player_pos;
            to_u8_angle(vec_to_angle(delta))
        } else {
            default_angle
        };

        let stick_dir: Option<u8> = self
            .stick_id
            .and_then(|sid| self.fingers.get(&sid))
            .map(|stick_finger| stick_finger.delta() * Vec2::new(1., -1.))
            .map(|delta| to_u8_angle(vec_to_angle(delta)));

        let dir = stick_dir.unwrap_or(0u8);
        if stick_dir.is_some() {
            btn |= MOVE;
        }

        // if nothing was hit
        if btn == 0 {
            None
        } else {
            Some(PlayerInput { dir, btn, angle })
        }
    }
}

/// in debug mode, render some gizmos to display the finger's touch positions
fn debug_touches(
    q_camera: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    touches: Res<Touches>,
    mut gizmos: Gizmos,
) {
    let Ok((camera, camera_transform)) = q_camera.get_single() else { return; };
    for touch in touches.iter() {
        let start = camera
            .viewport_to_world_2d(camera_transform, touch.start_position())
            .unwrap();
        let end = camera
            .viewport_to_world_2d(camera_transform, touch.position())
            .unwrap();
        gizmos.circle_2d(start, 4., Color::RED);
        gizmos.circle_2d(end, 4., Color::BLUE);
    }
}
