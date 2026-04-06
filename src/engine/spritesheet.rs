use std::collections::HashMap;

use bevy::{asset::LoadState, prelude::*};

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum SpritesheetID
{
    Terrain,
    HumanImperialWalking,
    House,
}

// All spritesheets must be registered here.
// The 'grid' parameter is the number of columns/rows in the sheet.
pub fn register_spritesheets(mut sheets: ResMut<SpritesheetRegistry>)
{
    sheets.register(SpritesheetID::Terrain, "art/sprites/tileset.png", UVec2::new(16, 8));
    sheets.register(SpritesheetID::House, "art/sprites/house.png", UVec2::new(1, 1));
    sheets.register(
        SpritesheetID::HumanImperialWalking,
        "art/sprites/human_imperial_walking.png",
        UVec2::new(4, 1),
    );
}

#[derive(Clone, Debug)]
pub struct SpritesheetDef
{
    pub path: &'static str,
    // Number of columns/rows in the sheet.
    pub grid: UVec2,
}

impl SpritesheetDef
{
    #[inline]
    pub fn sprite_index(&self, col: u32, row: u32) -> u32
    {
        return row * self.grid.x + col;
    }
}

#[derive(Resource, Default)]
pub struct SpritesheetRegistry
{
    sheets: HashMap<SpritesheetID, SpritesheetDef>,
    order: Vec<SpritesheetID>,
    pub layouts: HashMap<SpritesheetID, Handle<TextureAtlasLayout>>,
    pub images: HashMap<SpritesheetID, Handle<Image>>,
}

impl SpritesheetRegistry
{
    pub fn register(&mut self, id: SpritesheetID, path: &'static str, grid: UVec2)
    {
        // grid = (columns, rows).
        self.sheets.insert(id, SpritesheetDef { path, grid });
        if !self.order.contains(&id)
        {
            self.order.push(id);
        }
    }

    pub fn get(&self, id: SpritesheetID) -> Option<&SpritesheetDef>
    {
        return self.sheets.get(&id);
    }

    pub fn iter_ordered(&self) -> impl Iterator<Item = (SpritesheetID, &SpritesheetDef)>
    {
        return self
            .order
            .iter()
            .map(|id| (*id, self.sheets.get(id).unwrap()));
    }
}

#[derive(Resource, Default)]
pub struct AtlasLayoutState
{
    pub done: bool,
}

pub fn start_loading_sheets(
    mut sheet_registry: ResMut<SpritesheetRegistry>,
    asset_server: Res<AssetServer>,
)
{
    let entries: Vec<_> = sheet_registry
        .iter_ordered()
        .map(|(id, def)| (id, def.path))
        .collect();

    for (id, path) in entries
    {
        sheet_registry
            .images
            .insert(id, asset_server.load(path.to_owned()));
    }
}

pub fn build_atlas_layouts(
    mut sheet_registry: ResMut<SpritesheetRegistry>,
    mut layout_assets: ResMut<Assets<TextureAtlasLayout>>,
    mut state: ResMut<AtlasLayoutState>,
    images: Res<Assets<Image>>,
    asset_server: Res<AssetServer>,
)
{
    // Wait until every image is loaded.
    let all_ready = sheet_registry
        .images
        .values()
        .all(|h| matches!(asset_server.get_load_state(h), Some(LoadState::Loaded)));
    if !all_ready
    {
        return;
    }

    let entries: Vec<_> = sheet_registry
        .iter_ordered()
        .map(|(id, def)| (id, def.grid))
        .collect();

    for (id, grid) in entries
    {
        let image = images.get(&sheet_registry.images[&id]).unwrap();

        // Derive sprite pixel size from actual image dimensions.
        let sprite_px = UVec2::new(image.width() / grid.x, image.height() / grid.y);

        let layout = TextureAtlasLayout::from_grid(sprite_px, grid.x, grid.y, None, None);
        sheet_registry.layouts.insert(id, layout_assets.add(layout));
    }

    state.done = true;
}

pub struct SpritesheetPlugin;

impl Plugin for SpritesheetPlugin
{
    fn build(&self, app: &mut App)
    {
        app.init_resource::<SpritesheetRegistry>()
            .init_resource::<AtlasLayoutState>()
            .add_systems(Startup, (register_spritesheets, start_loading_sheets).chain())
            .add_systems(Update, build_atlas_layouts.run_if(|s: Res<AtlasLayoutState>| !s.done));
    }
}
