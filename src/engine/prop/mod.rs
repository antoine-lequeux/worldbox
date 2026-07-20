pub mod spawn;

use std::collections::HashMap;

use bevy::{asset::LoadState, prelude::*};

use crate::engine::spritesheet::{SpritesheetID, SpritesheetRegistry};

// Bundle inserted when spawning a prop entity.
#[derive(Bundle)]
pub struct PropSpriteBundle
{
    pub sprite: Sprite,
    pub prop_type: PropType,
    pub variation_index: VariationIndex,
    pub anim: AnimationState,
}

impl PropRegistry
{
    // Builds a PropSpriteBundle for the given prop type and variation.
    pub fn sprite_bundle(
        &self,
        prop_type: PropType,
        variation: u32,
        sheets: &SpritesheetRegistry,
    ) -> PropSpriteBundle
    {
        let def = self
            .props
            .get(&prop_type)
            .unwrap_or_else(|| panic!("PropType '{:?}' not registered", prop_type));
        let sheet = sheets
            .get(def.sheet_id)
            .unwrap_or_else(|| panic!("Sheet '{:?}' not registered", def.sheet_id));

        // Atlas index for frame 0 of the chosen variation.
        // grid.x = variation columns, grid.y = frame rows.
        // atlas_index = frame_row * grid.x + variation_col
        let origin = def.sprite.frame_origin(variation, 0);
        let atlas_index = (origin.y * sheet.grid.x + origin.x) as usize;

        return PropSpriteBundle {
            sprite: Sprite::from_atlas_image(
                sheets.images[&def.sheet_id].clone(),
                TextureAtlas { layout: sheets.layouts[&def.sheet_id].clone(), index: atlas_index },
            ),
            prop_type,
            variation_index: VariationIndex(variation),
            anim: AnimationState::default(),
        };
    }
}

// Describes the animation layout of a prop in its spritesheet.
#[derive(Clone, Debug)]
pub enum PropSprite
{
    Static,
    // Frames are stacked in successive rows within the chosen variation column.
    Animated
    {
        // Total number of animation frames (number of rows used).
        frame_count: u32,
        // Seconds between frame advances.
        period: f32,
    },
}

impl PropSprite
{
    // Returns the total number of animation frames (1 for static).
    pub fn frame_count(&self) -> usize
    {
        return match self
        {
            Self::Static => 1,
            Self::Animated { frame_count, .. } => *frame_count as usize,
        };
    }

    // Returns the animation period in seconds, or None for static sprites.
    pub fn period(&self) -> Option<f32>
    {
        return match self
        {
            Self::Static => None,
            Self::Animated { period, .. } => Some(*period),
        };
    }

    // Returns the spritesheet grid origin (col = variation, row = frame) for a
    // given variation index and animation frame.
    pub fn frame_origin(&self, variation: u32, frame: usize) -> UVec2
    {
        return UVec2::new(variation, frame as u32);
    }
}

// Full definition of a prop: spritesheet, size, animation, and macro map colors.
#[derive(Clone, Debug)]
pub struct PropDefinition
{
    // Which spritesheet this prop's sprites live in.
    pub sheet_id: SpritesheetID,
    // Size in world tiles.
    pub size_tiles: UVec2,
    // Sprite layout (static or animated).
    pub sprite: PropSprite,
    // Number of variation columns available in the spritesheet.
    pub variation_count: u32,
    // Whether macro map colors should be sampled from the sprite image.
    pub sample_macro_colors: bool,
    // Precomputed macro colors. Layout: [variation][frame][tile_row * size_tiles.x + tile_col].
    pub macro_colors: Vec<Vec<Vec<[u8; 4]>>>,
}

// Identifies the kind of prop (used as a component and registry key).
#[derive(Component, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum PropType
{
    HouseTier1,
    HouseTier2,
    HouseTier3,
    HumanImperialWalking,
    HumanForestWalking,
    HumanNorthernWalking,
    HumanTribalWalking,
}

// The variation column chosen at spawn time.
// Stored as a component so animation and macro-map systems can look it up.
#[derive(Component, Clone, Copy, Debug)]
pub struct VariationIndex(pub u32);

// Tracks the current animation frame and elapsed time for animated props.
#[derive(Component, Clone, Debug, Default)]
pub struct AnimationState
{
    pub current_frame: usize,
    pub elapsed: f32,
}

// Central registry mapping each PropType to its definition.
#[derive(Resource)]
pub struct PropRegistry
{
    pub props: HashMap<PropType, PropDefinition>,
}

impl Default for PropRegistry
{
    fn default() -> Self
    {
        let mut props = HashMap::new();

        props.insert(
            PropType::HouseTier1,
            PropDefinition {
                sheet_id: SpritesheetID::HouseTier1,
                size_tiles: UVec2::new(2, 2),
                sprite: PropSprite::Static,
                variation_count: 4,
                sample_macro_colors: true,
                macro_colors: Vec::new(),
            },
        );

        props.insert(
            PropType::HouseTier2,
            PropDefinition {
                sheet_id: SpritesheetID::HouseTier2,
                size_tiles: UVec2::new(2, 2),
                sprite: PropSprite::Static,
                variation_count: 4,
                sample_macro_colors: true,
                macro_colors: Vec::new(),
            },
        );

        props.insert(
            PropType::HouseTier3,
            PropDefinition {
                sheet_id: SpritesheetID::HouseTier3,
                size_tiles: UVec2::new(2, 2),
                sprite: PropSprite::Static,
                variation_count: 4,
                sample_macro_colors: true,
                macro_colors: Vec::new(),
            },
        );

        let human_sprite = PropSprite::Animated { frame_count: 4, period: 0.08 };

        for (prop_type, sheet_id) in [
            (PropType::HumanImperialWalking, SpritesheetID::HumanImperialWalking),
            (PropType::HumanForestWalking, SpritesheetID::HumanForestWalking),
            (PropType::HumanNorthernWalking, SpritesheetID::HumanNorthernWalking),
            (PropType::HumanTribalWalking, SpritesheetID::HumanTribalWalking),
        ]
        {
            props.insert(
                prop_type,
                PropDefinition {
                    sheet_id,
                    size_tiles: UVec2::new(1, 1),
                    sprite: human_sprite.clone(),
                    variation_count: 1,
                    sample_macro_colors: false,
                    macro_colors: Vec::new(),
                },
            );
        }

        return Self { props };
    }
}

impl PropRegistry
{
    // Returns the size and per tile macro colors for a prop at a given frame and variation.
    pub fn get_prop_data(
        &self,
        prop_type: PropType,
        frame: usize,
        variation: u32,
    ) -> Option<(IVec2, &[[u8; 4]])>
    {
        let def = self.props.get(&prop_type)?;
        if def.macro_colors.is_empty()
        {
            return None;
        }
        let variation_data = def.macro_colors.get(variation as usize)?;
        let frame_data = variation_data.get(frame % variation_data.len())?;
        return Some((
            IVec2::new(def.size_tiles.x as i32, def.size_tiles.y as i32),
            frame_data.as_slice(),
        ));
    }
}

// Tracks whether prop macro color sampling has been completed.
#[derive(Resource, Default)]
pub struct PropSamplingState
{
    pub done: bool,
}

// Samples average pixel colors from prop sprite images to populate macro_colors.
// Layout produced: macro_colors[variation][frame][tile].
pub fn finish_prop_sampling(
    mut prop_registry: ResMut<PropRegistry>,
    sheet_registry: Res<SpritesheetRegistry>,
    mut state: ResMut<PropSamplingState>,
    images: Res<Assets<Image>>,
    asset_server: Res<AssetServer>,
)
{
    if state.done
    {
        return;
    }

    let all_ready = sheet_registry
        .images
        .values()
        .all(|h| matches!(asset_server.get_load_state(h), Some(LoadState::Loaded)));
    if !all_ready
    {
        return;
    }

    for def in prop_registry.props.values_mut()
    {
        if !def.sample_macro_colors || !def.macro_colors.is_empty()
        {
            continue;
        }
        let Some(sheet) = sheet_registry.get(def.sheet_id)
        else
        {
            continue;
        };
        let Some(handle) = sheet_registry.images.get(&def.sheet_id)
        else
        {
            continue;
        };
        let Some(image) = images.get(handle)
        else
        {
            continue;
        };

        // Cell size in pixels: one cell per (variation, frame) pair.
        // grid.x = variation columns, grid.y = frame rows.
        let cell_px = UVec2::new(image.width() / sheet.grid.x, image.height() / sheet.grid.y);

        let mut all_variations: Vec<Vec<Vec<[u8; 4]>>> = Vec::new();

        for variation in 0 .. def.variation_count
        {
            let mut variation_frames: Vec<Vec<[u8; 4]>> = Vec::new();

            for frame_idx in 0 .. def.sprite.frame_count()
            {
                let origin = def.sprite.frame_origin(variation, frame_idx);
                let sprite_px = origin * cell_px;
                let tile_px = cell_px / def.size_tiles;

                let mut frame_colors: Vec<[u8; 4]> = Vec::new();

                for ty in 0 .. def.size_tiles.y
                {
                    for tx in 0 .. def.size_tiles.x
                    {
                        frame_colors.push(sample_average(
                            image,
                            sprite_px.x + tx * tile_px.x,
                            sprite_px.y + ty * tile_px.y,
                            tile_px.x,
                            tile_px.y,
                        ));
                    }
                }

                variation_frames.push(frame_colors);
            }

            all_variations.push(variation_frames);
        }

        def.macro_colors = all_variations;
    }

    state.done = true;
}

// Computes the average RGBA color of a rectangular region in an image.
fn sample_average(image: &Image, px: u32, py: u32, w: u32, h: u32) -> [u8; 4]
{
    let count = (w * h) as u64;
    if count == 0
    {
        return [0, 0, 0, 0];
    }

    let img_w = image.width();
    let (mut r, mut g, mut b, mut a) = (0u64, 0u64, 0u64, 0u64);

    if let Some(data) = &image.data
    {
        for dy in 0 .. h
        {
            for dx in 0 .. w
            {
                let idx = ((py + dy) * img_w + (px + dx)) as usize * 4;
                if idx + 3 < data.len()
                {
                    r += data[idx] as u64;
                    g += data[idx + 1] as u64;
                    b += data[idx + 2] as u64;
                    a += data[idx + 3] as u64;
                }
            }
        }
    }

    return [(r / count) as u8, (g / count) as u8, (b / count) as u8, (a / count) as u8];
}

// Advances animation timers and updates the current frame index for all animated props.
pub fn update_animations(
    time: Res<Time>,
    prop_registry: Res<PropRegistry>,
    mut query: Query<(&PropType, &mut AnimationState)>,
)
{
    for (prop_type, mut anim) in &mut query
    {
        let Some(def) = prop_registry.props.get(prop_type)
        else
        {
            continue;
        };
        let Some(period) = def.sprite.period()
        else
        {
            continue;
        };

        anim.elapsed += time.delta_secs();
        if anim.elapsed >= period
        {
            anim.elapsed -= period;
            anim.current_frame = (anim.current_frame + 1) % def.sprite.frame_count();
        }
    }
}

// Updates each prop's Sprite atlas index when its AnimationState changes.
// Uses the stored VariationIndex to compute the correct atlas cell.
pub fn sync_sprite_frame(
    prop_registry: Res<PropRegistry>,
    sheet_registry: Res<SpritesheetRegistry>,
    mut query: Query<
        (&PropType, &VariationIndex, &AnimationState, &mut Sprite),
        Changed<AnimationState>,
    >,
)
{
    for (prop_type, variation_index, anim, mut sprite) in &mut query
    {
        let Some(def) = prop_registry.props.get(prop_type)
        else
        {
            continue;
        };
        let Some(sheet) = sheet_registry.get(def.sheet_id)
        else
        {
            continue;
        };

        // atlas_index = frame_row * variation_cols + variation_col
        let origin = def
            .sprite
            .frame_origin(variation_index.0, anim.current_frame);
        let index = (origin.y * sheet.grid.x + origin.x) as usize;

        if let Some(atlas) = &mut sprite.texture_atlas
        {
            atlas.index = index;
        }
    }
}

pub struct PropPlugin;

impl Plugin for PropPlugin
{
    fn build(&self, app: &mut App)
    {
        app.init_resource::<PropRegistry>()
            .init_resource::<PropSamplingState>()
            .add_systems(Update, finish_prop_sampling.run_if(|s: Res<PropSamplingState>| !s.done))
            .add_systems(Update, (update_animations, sync_sprite_frame).chain());
    }
}
