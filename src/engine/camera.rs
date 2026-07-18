use bevy::prelude::*;

use super::consts::{CHUNK_SIZE, MAP_HEIGHT, MAP_WIDTH, TILE_SIZE};

// Half-extents of the map in world-space pixels.
const HALF_MAP_W: f32 = (MAP_WIDTH * CHUNK_SIZE * TILE_SIZE) as f32 / 2.0;
const HALF_MAP_H: f32 = (MAP_HEIGHT * CHUNK_SIZE * TILE_SIZE) as f32 / 2.0;

// State attached to the main camera for inertia-based panning and zooming.
#[derive(Component)]
pub struct MainCamera
{
    // Current panning velocity for inertia after drag ends.
    pub pan_velocity: Vec2,
    // Current scroll-wheel zoom velocity.
    pub scroll_velocity: f32,
    // True while the user is actively dragging the camera.
    pub is_dragging: bool,
    // The previous frame's translation, used to compute smoothed pan velocity.
    pub last_pos: Vec2,
}

impl Default for MainCamera
{
    fn default() -> Self
    {
        return Self {
            pan_velocity: Vec2::ZERO,
            scroll_velocity: 0.0,
            is_dragging: false,
            last_pos: Vec2::ZERO,
        };
    }
}

// Spawns the 2D camera and an invisible full-screen sprite that captures drag and scroll events.
pub fn setup_camera(mut commands: Commands)
{
    commands.spawn((Camera2d, MainCamera::default()));

    // Invisible sprite used as a pickable surface for pointer events.
    commands
        .spawn((
            Sprite {
                custom_size: Some(Vec2::new(100000.0, 100000.0)),
                color: Color::NONE,
                ..default()
            },
            Transform::from_xyz(0.0, 0.0, -1.0),
            Pickable::default(),
        ))
        .observe(
            |event: On<Pointer<DragStart>>,
             mut camera_query: Query<(&mut MainCamera, &Transform)>| {
                if event.button == PointerButton::Primary
                {
                    return;
                }
                if let Ok((mut camera, transform)) = camera_query.single_mut()
                {
                    camera.is_dragging = true;
                    camera.pan_velocity = Vec2::ZERO;
                    camera.last_pos = transform.translation.truncate();
                }
            },
        )
        .observe(
            |drag: On<Pointer<Drag>>, mut camera_query: Query<(&mut Transform, &Projection)>| {
                if drag.button == PointerButton::Primary
                {
                    return;
                }
                if let Ok((mut transform, projection)) = camera_query.single_mut()
                {
                    if let Projection::Orthographic(ortho) = projection
                    {
                        let pan_delta = Vec2::new(-drag.delta.x, drag.delta.y) * ortho.scale;
                        transform.translation.x += pan_delta.x;
                        transform.translation.y += pan_delta.y;

                        // Clamp to map bounds.
                        if transform.translation.x < -HALF_MAP_W
                        {
                            transform.translation.x = -HALF_MAP_W;
                        }
                        else if transform.translation.x > HALF_MAP_W
                        {
                            transform.translation.x = HALF_MAP_W;
                        }
                        if transform.translation.y < -HALF_MAP_H
                        {
                            transform.translation.y = -HALF_MAP_H;
                        }
                        else if transform.translation.y > HALF_MAP_H
                        {
                            transform.translation.y = HALF_MAP_H;
                        }
                    }
                }
            },
        )
        .observe(|event: On<Pointer<DragEnd>>, mut camera_query: Query<&mut MainCamera>| {
            if event.button == PointerButton::Primary
            {
                return;
            }
            if let Ok(mut camera) = camera_query.single_mut()
            {
                camera.is_dragging = false;
            }
        })
        .observe(|scroll: On<Pointer<Scroll>>, mut camera_query: Query<&mut MainCamera>| {
            if let Ok(mut camera) = camera_query.single_mut()
            {
                camera.scroll_velocity -= scroll.y * 2.0; // Multiply for sensitivity
                camera.scroll_velocity = camera.scroll_velocity.clamp(-25.0, 25.0);
            }
        });
}

// Applies zoom and pan inertia each frame.
pub fn update_camera(
    mut camera_query: Query<(&mut Transform, &mut Projection, &mut MainCamera)>,
    time: Res<Time>,
)
{
    if let Ok((mut transform, mut projection, mut main_cam)) = camera_query.single_mut()
    {
        let dt = time.delta_secs();

        // Smooth zoom: apply scroll velocity to orthographic scale.
        if let Projection::Orthographic(ref mut ortho) = *projection
        {
            if main_cam.scroll_velocity.abs() > 0.001
            {
                ortho.scale += main_cam.scroll_velocity * dt;
                ortho.scale = ortho.scale.clamp(0.1, 20.0);

                // Exponential decay for smooth deceleration.
                main_cam.scroll_velocity *= (-10.0_f32 * dt).exp();
            }
            else
            {
                main_cam.scroll_velocity = 0.0;
            }
        }

        // Pan inertia: compute velocity while dragging, or continue sliding after release.
        if main_cam.is_dragging
        {
            let current_pos = transform.translation.truncate();
            let raw_vel = (current_pos - main_cam.last_pos) / dt.max(0.001);

            // Smooth the velocity with an exponential moving average.
            let smoothing = 1.0 - (-20.0_f32 * dt).exp();
            main_cam.pan_velocity = main_cam.pan_velocity.lerp(raw_vel, smoothing);
            main_cam.last_pos = current_pos;
        }
        else
        {
            if main_cam.pan_velocity.length_squared() > 1.0
            {
                transform.translation.x += main_cam.pan_velocity.x * dt;
                transform.translation.y += main_cam.pan_velocity.y * dt;

                // Clamp to map bounds and zero velocity on clamped axes.
                if transform.translation.x < -HALF_MAP_W
                {
                    transform.translation.x = -HALF_MAP_W;
                    main_cam.pan_velocity.x = 0.0;
                }
                else if transform.translation.x > HALF_MAP_W
                {
                    transform.translation.x = HALF_MAP_W;
                    main_cam.pan_velocity.x = 0.0;
                }
                if transform.translation.y < -HALF_MAP_H
                {
                    transform.translation.y = -HALF_MAP_H;
                    main_cam.pan_velocity.y = 0.0;
                }
                else if transform.translation.y > HALF_MAP_H
                {
                    transform.translation.y = HALF_MAP_H;
                    main_cam.pan_velocity.y = 0.0;
                }

                main_cam.pan_velocity *= (-3.0_f32 * dt).exp();
            }
            else
            {
                main_cam.pan_velocity = Vec2::ZERO;
            }
        }
    }
}

pub struct CameraPlugin;

impl Plugin for CameraPlugin
{
    fn build(&self, app: &mut App)
    {
        app.add_systems(PostStartup, setup_camera)
            .add_systems(Update, update_camera);
    }
}
