use bevy::prelude::*;

mod autotile;
pub mod border_outline;
pub mod macro_map;
pub mod tilemap;

pub use macro_map::MacroMapDot;
pub use tilemap::StandardRenderLayer;

// Groups all rendering-related plugins (tilemap, macro map, border outline).
pub struct RenderingPlugin;

impl Plugin for RenderingPlugin
{
    fn build(&self, app: &mut App)
    {
        app.add_plugins(tilemap::CustomTilemapPlugin)
            .add_plugins(macro_map::MacroMapPlugin)
            .add_plugins(border_outline::BorderOutlinePlugin);
    }
}
