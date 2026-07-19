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
    // The target scale for the camera zoom.
    pub target_scale: f32,
    // The current index in the zoom_levels array.
    pub target_zoom_level: usize,
    // The defined list of possible zoom scales.
    pub zoom_levels: Vec<f32>,
    // Accumulator for scroll events to handle trackpads and normal mice.
    pub scroll_accum: f32,
    // True while the user is actively dragging the camera.
    pub is_dragging: bool,
    // The previous frame's translation, used to compute smoothed pan velocity.
    pub last_pos: Vec2,
}

impl Default for MainCamera
{
    fn default() -> Self
    {
        let mut levels = vec![0.15_f32];
        let mut current = 0.15_f32;
        let mut offset = 0.05_f32;
        while current < 60.0
        {
            current += offset;
            levels.push(current);
            offset *= 1.15;
        }

        // Find index closest to 1.0
        let mut default_idx = 0;
        let mut min_diff = f32::MAX;
        for (i, &lvl) in levels.iter().enumerate()
        {
            let diff = (lvl - 1.0_f32).abs();
            if diff < min_diff
            {
                min_diff = diff;
                default_idx = i;
            }
        }

        return Self {
            pan_velocity: Vec2::ZERO,
            target_scale: levels[default_idx],
            target_zoom_level: default_idx,
            zoom_levels: levels,
            scroll_accum: 0.0,
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
                camera.scroll_accum -= scroll.y;

                while camera.scroll_accum >= 1.0
                {
                    camera.target_zoom_level = camera
                        .target_zoom_level
                        .saturating_add(1)
                        .min(camera.zoom_levels.len() - 1);
                    camera.scroll_accum -= 1.0;
                }
                while camera.scroll_accum <= -1.0
                {
                    camera.target_zoom_level = camera.target_zoom_level.saturating_sub(1);
                    camera.scroll_accum += 1.0;
                }

                camera.target_scale = camera.zoom_levels[camera.target_zoom_level];
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

        // Interpolate orthographic scale toward the target scale.
        if let Projection::Orthographic(ref mut ortho) = *projection
        {
            let diff = main_cam.target_scale - ortho.scale;
            if diff.abs() > 0.0001
            {
                let max_speed = 50.0;
                let speed = (diff.abs() * 15.0).min(max_speed);
                let step = speed * diff.signum() * dt;

                if step.abs() >= diff.abs()
                {
                    ortho.scale = main_cam.target_scale;
                }
                else
                {
                    ortho.scale += step;
                }
            }
            else
            {
                ortho.scale = main_cam.target_scale;
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
