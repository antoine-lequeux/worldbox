use std::collections::HashMap;

use bevy::{asset::LoadState, prelude::*};

// Known spritesheet identifiers.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum SpritesheetID
{
    Terrain,
    HumanImperialWalking,
    HumanForestWalking,
    HumanNorthernWalking,
    HumanTribalWalking,
    HouseTier0,
    HouseTier1,
    HouseTier2,
    HouseTier3,
    HouseTier4,
    HouseTier5,
    HouseTier6,
}

// All spritesheets must be registered here.
//
// Grid convention:
//   grid.x = number of columns = number of variations
//   grid.y = number of rows = number of animation frames
pub fn register_spritesheets(mut sheets: ResMut<SpritesheetRegistry>)
{
    sheets.register(SpritesheetID::Terrain, "art/sprites/tileset.png", UVec2::new(1, 8));

    sheets.register(
        SpritesheetID::HumanImperialWalking,
        "art/sprites/human_imperial_walking.png",
        UVec2::new(1, 4),
    );
    sheets.register(
        SpritesheetID::HumanForestWalking,
        "art/sprites/human_forest_walking.png",
        UVec2::new(1, 4),
    );
    sheets.register(
        SpritesheetID::HumanNorthernWalking,
        "art/sprites/human_northern_walking.png",
        UVec2::new(1, 4),
    );
    sheets.register(
        SpritesheetID::HumanTribalWalking,
        "art/sprites/human_tribal_walking.png",
        UVec2::new(1, 4),
    );

    sheets.register(SpritesheetID::HouseTier0, "art/sprites/house_tier0.png", UVec2::new(1, 1));
    sheets.register(SpritesheetID::HouseTier1, "art/sprites/house_tier1.png", UVec2::new(4, 1));
    sheets.register(SpritesheetID::HouseTier2, "art/sprites/house_tier2.png", UVec2::new(4, 1));
    sheets.register(SpritesheetID::HouseTier3, "art/sprites/house_tier3.png", UVec2::new(4, 1));
    sheets.register(SpritesheetID::HouseTier4, "art/sprites/house_tier4.png", UVec2::new(4, 1));
    sheets.register(SpritesheetID::HouseTier5, "art/sprites/house_tier5.png", UVec2::new(4, 1));
    sheets.register(SpritesheetID::HouseTier6, "art/sprites/house_tier6.png", UVec2::new(4, 1));
}

// Metadata for a single spritesheet image.
#[derive(Clone, Debug)]
pub struct SpritesheetDef
{
    pub path: &'static str,
    // Number of columns (variations) and rows (animation frames) in the sheet grid.
    pub grid: UVec2,
}

// Central store for all spritesheet definitions, loaded image handles, and atlas layouts.
#[derive(Resource, Default)]
pub struct SpritesheetRegistry
{
    sheets: HashMap<SpritesheetID, SpritesheetDef>,
    // Insertion order for deterministic iteration.
    order: Vec<SpritesheetID>,
    // Atlas layouts built after images are loaded.
    pub layouts: HashMap<SpritesheetID, Handle<TextureAtlasLayout>>,
    // Image asset handles, populated at startup.
    pub images: HashMap<SpritesheetID, Handle<Image>>,
}

impl SpritesheetRegistry
{
    // Register a spritesheet with its asset path and grid dimensions.
    pub fn register(&mut self, id: SpritesheetID, path: &'static str, grid: UVec2)
    {
        // grid = (columns, rows).
        self.sheets.insert(id, SpritesheetDef { path, grid });
        if !self.order.contains(&id)
        {
            self.order.push(id);
        }
    }

    // Look up a spritesheet definition by ID.
    pub fn get(&self, id: SpritesheetID) -> Option<&SpritesheetDef>
    {
        return self.sheets.get(&id);
    }

    // Iterate over all registered spritesheets in insertion order.
    pub fn iter_ordered(&self) -> impl Iterator<Item = (SpritesheetID, &SpritesheetDef)>
    {
        return self
            .order
            .iter()
            .map(|id| (*id, self.sheets.get(id).unwrap()));
    }
}

// Tracks whether atlas layouts have been built (gates dependent systems).
#[derive(Resource, Default)]
pub struct AtlasLayoutState
{
    pub done: bool,
}

// Kicks off async loading for all registered spritesheet images.
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

// Runs each frame until all images are loaded, then builds TextureAtlasLayouts.
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
