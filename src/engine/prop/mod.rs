pub mod colored_sprites;
pub mod spawn;

use std::collections::HashMap;

use bevy::prelude::*;

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
}

// Identifies the kind of prop (used as a component and registry key).
#[derive(Component, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum PropType
{
    HouseTier0,
    HouseTier1,
    HouseTier2,
    HouseTier3,
    HouseTier4,
    HouseTier5,
    HouseTier6,
    HumanImperialWalking,
    HumanForestWalking,
    HumanNorthernWalking,
    HumanTribalWalking,
}

// The variation column chosen at spawn time.
// Stored as a component so animation and macro-map systems can look it up.
#[derive(Component, Clone, Copy, Debug)]
pub struct VariationIndex(pub u32);

// Link to the child entity holding the macro sprite.
#[derive(Component, Clone, Copy, Debug)]
pub struct MacroSpriteChild(pub Entity);

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
            PropType::HouseTier0,
            PropDefinition {
                sheet_id: SpritesheetID::HouseTier0,
                size_tiles: UVec2::new(2, 2),
                sprite: PropSprite::Static,
                variation_count: 1,
            },
        );

        props.insert(
            PropType::HouseTier1,
            PropDefinition {
                sheet_id: SpritesheetID::HouseTier1,
                size_tiles: UVec2::new(2, 2),
                sprite: PropSprite::Static,
                variation_count: 4,
            },
        );

        props.insert(
            PropType::HouseTier2,
            PropDefinition {
                sheet_id: SpritesheetID::HouseTier2,
                size_tiles: UVec2::new(2, 2),
                sprite: PropSprite::Static,
                variation_count: 4,
            },
        );

        props.insert(
            PropType::HouseTier3,
            PropDefinition {
                sheet_id: SpritesheetID::HouseTier3,
                size_tiles: UVec2::new(2, 2),
                sprite: PropSprite::Static,
                variation_count: 4,
            },
        );

        props.insert(
            PropType::HouseTier4,
            PropDefinition {
                sheet_id: SpritesheetID::HouseTier4,
                size_tiles: UVec2::new(3, 3),
                sprite: PropSprite::Static,
                variation_count: 4,
            },
        );

        props.insert(
            PropType::HouseTier5,
            PropDefinition {
                sheet_id: SpritesheetID::HouseTier5,
                size_tiles: UVec2::new(4, 4),
                sprite: PropSprite::Static,
                variation_count: 4,
            },
        );

        props.insert(
            PropType::HouseTier6,
            PropDefinition {
                sheet_id: SpritesheetID::HouseTier6,
                size_tiles: UVec2::new(4, 4),
                sprite: PropSprite::Static,
                variation_count: 4,
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
                },
            );
        }

        return Self { props };
    }
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
        (
            &PropType,
            &VariationIndex,
            &AnimationState,
            &mut Sprite,
            Option<&MacroSpriteChild>,
        ),
        Changed<AnimationState>,
    >,
    mut child_sprites: Query<&mut Sprite, Without<PropType>>,
)
{
    for (prop_type, variation_index, anim, mut sprite, macro_child) in &mut query
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

        if let Some(MacroSpriteChild(child_ent)) = macro_child
        {
            if let Ok(mut child_sprite) = child_sprites.get_mut(*child_ent)
            {
                if let Some(atlas) = &mut child_sprite.texture_atlas
                {
                    atlas.index = index;
                }
            }
        }
    }
}

pub struct PropPlugin;

impl Plugin for PropPlugin
{
    fn build(&self, app: &mut App)
    {
        app.init_resource::<PropRegistry>()
            .init_resource::<colored_sprites::ColoredSpriteCache>()
            .add_systems(Update, (update_animations, sync_sprite_frame).chain())
            .add_systems(
                Update,
                (colored_sprites::apply_building_colors, colored_sprites::gc_colored_sprites)
                    .chain(),
            );
    }
}
