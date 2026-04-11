use std::time::Duration;

use bevy::{diagnostic::FrameTimeDiagnosticsPlugin, prelude::*, time::common_conditions::on_timer};

// Marker component for the FPS counter text entity.
#[derive(Component)]
struct FpsText;

// Spawns the FPS counter text in the top-left corner.
fn setup_fps_counter(mut commands: Commands)
{
    commands.spawn((
        Text::new("FPS: --"),
        TextFont { font_size: 16.0, ..default() },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(5.0),
            left: Val::Px(5.0),
            ..default()
        },
        FpsText,
    ));
}

// Reads the latest FPS diagnostic and updates the text.
fn update_fps_counter(
    diagnostics: Res<bevy::diagnostic::DiagnosticsStore>,
    mut query: Query<&mut Text, With<FpsText>>,
)
{
    if let Ok(mut text) = query.single_mut()
    {
        if let Some(fps) = diagnostics
            .get(&bevy::diagnostic::FrameTimeDiagnosticsPlugin::FPS)
            .and_then(|d| d.smoothed())
        {
            **text = format!("FPS: {fps:.1}");
        }
    }
}

pub struct FpsPlugin;

impl Plugin for FpsPlugin
{
    fn build(&self, app: &mut App)
    {
        app.add_plugins(FrameTimeDiagnosticsPlugin::default())
            .add_systems(Startup, setup_fps_counter)
            .add_systems(Update, update_fps_counter.run_if(on_timer(Duration::from_secs(1))));
    }
}
