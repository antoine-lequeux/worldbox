use bevy::prelude::*;

#[derive(Component)]
pub struct MainCamera
{
    pub pan_velocity: Vec2,
    pub scroll_velocity: f32,
    pub is_dragging: bool,
}

impl Default for MainCamera
{
    fn default() -> Self
    {
        Self { pan_velocity: Vec2::ZERO, scroll_velocity: 0.0, is_dragging: false }
    }
}

pub fn setup_camera(mut commands: Commands)
{
    commands.spawn((Camera2d, MainCamera::default()));

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
        .observe(|_: On<Pointer<DragStart>>, mut camera_query: Query<&mut MainCamera>| {
            if let Ok(mut camera) = camera_query.single_mut()
            {
                camera.is_dragging = true;
                camera.pan_velocity = Vec2::ZERO;
            }
        })
        .observe(
            |drag: On<Pointer<Drag>>,
             mut camera_query: Query<(&mut MainCamera, &mut Transform, &Projection)>,
             time: Res<Time>| {
                if let Ok((mut camera, mut transform, projection)) = camera_query.single_mut()
                {
                    if let Projection::Orthographic(ortho) = projection
                    {
                        let pan_delta = Vec2::new(-drag.delta.x, drag.delta.y) * ortho.scale;
                        transform.translation.x += pan_delta.x;
                        transform.translation.y += pan_delta.y;

                        let dt = time.delta_secs().max(0.001);
                        camera.pan_velocity = pan_delta / dt;
                    }
                }
            },
        )
        .observe(|_: On<Pointer<DragEnd>>, mut camera_query: Query<&mut MainCamera>| {
            if let Ok(mut camera) = camera_query.single_mut()
            {
                camera.is_dragging = false;
            }
        })
        .observe(|scroll: On<Pointer<Scroll>>, mut camera_query: Query<&mut MainCamera>| {
            if let Ok(mut camera) = camera_query.single_mut()
            {
                camera.scroll_velocity -= scroll.y * 1.5; // Multiply for sensitivity
                camera.scroll_velocity = camera.scroll_velocity.clamp(-15.0, 15.0);
            }
        });
}

pub fn update_camera(
    mut camera_query: Query<(&mut Transform, &mut Projection, &mut MainCamera)>,
    time: Res<Time>,
)
{
    if let Ok((mut transform, mut projection, mut main_cam)) = camera_query.single_mut()
    {
        let dt = time.delta_secs();

        if let Projection::Orthographic(ref mut ortho) = *projection
        {
            if main_cam.scroll_velocity.abs() > 0.001
            {
                ortho.scale += main_cam.scroll_velocity * dt;
                ortho.scale = ortho.scale.clamp(0.1, 15.0);

                main_cam.scroll_velocity *= (-10.0_f32 * dt).exp();
            }
            else
            {
                main_cam.scroll_velocity = 0.0;
            }
        }

        if !main_cam.is_dragging
        {
            if main_cam.pan_velocity.length_squared() > 1.0
            {
                transform.translation.x += main_cam.pan_velocity.x * dt;
                transform.translation.y += main_cam.pan_velocity.y * dt;

                main_cam.pan_velocity *= (-3.0_f32 * dt).exp();
            }
            else
            {
                main_cam.pan_velocity = Vec2::ZERO;
            }
        }
    }
}
