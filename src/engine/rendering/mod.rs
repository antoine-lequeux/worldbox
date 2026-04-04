use bevy::prelude::*;

pub mod macro_map;
pub mod tilemap;

pub use macro_map::MacroMapDot;
pub use tilemap::StandardRenderLayer;

pub struct RenderingPlugin;

impl Plugin for RenderingPlugin
{
    fn build(&self, app: &mut App)
    {
        app.add_plugins(tilemap::CustomTilemapPlugin)
            .add_plugins(macro_map::MacroMapPlugin);
    }
}
