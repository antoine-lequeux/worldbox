use bevy::prelude::*;

pub mod camera;
pub mod consts;
pub mod coords;
pub mod fps;
pub mod mapgen;
pub mod painting;
pub mod prop;
pub mod rendering;
pub mod spritesheet;
pub mod tile;

pub use consts::*;
pub use coords::GridPos;
pub use mapgen::MapData;
pub use rendering::{MacroMapEntity, StandardRenderLayer};

pub struct EnginePlugin;

impl Plugin for EnginePlugin
{
    fn build(&self, app: &mut App)
    {
        app.add_plugins(camera::CameraPlugin)
            .add_plugins(spritesheet::SpritesheetPlugin)
            .add_plugins(prop::PropPlugin)
            .add_plugins(prop::spawn::SpawnPlugin)
            .add_plugins(rendering::RenderingPlugin)
            .add_plugins(painting::PaintingPlugin)
            .add_plugins(mapgen::MapGenPlugin)
            .add_plugins(fps::FpsPlugin)
            .add_systems(PostUpdate, coords::sync_grid_positions);
    }
}
