use bevy::prelude::*;

pub mod autotile;
pub mod border_outline;
pub mod camera;
pub mod consts;
pub mod coords;
pub mod painting;
pub mod prop;
pub mod rendering;
pub mod spawn;
pub mod spritesheet;
pub mod tile;

pub use consts::*;
pub use coords::{GridPos, grid_to_world, world_to_grid};
pub use prop::PropType;
pub use rendering::{MacroMapDot, StandardRenderLayer};
pub use spawn::SpawnPropExt;

pub struct EnginePlugin;

impl Plugin for EnginePlugin
{
    fn build(&self, app: &mut App)
    {
        app.add_plugins(camera::CameraPlugin)
            .add_plugins(spritesheet::SpritesheetPlugin)
            .add_plugins(prop::PropPlugin)
            .add_plugins(spawn::SpawnPlugin)
            .add_plugins(rendering::RenderingPlugin)
            .add_plugins(autotile::AutotilePlugin)
            .add_plugins(painting::PaintingPlugin)
            .add_plugins(border_outline::BorderOutlinePlugin);
    }
}
