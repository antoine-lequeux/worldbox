use std::collections::HashMap;

use bevy::{asset::LoadState, prelude::*};

use crate::engine::spritesheet::{SpritesheetID, SpritesheetRegistry};

#[derive(Bundle)]
pub struct PropSpriteBundle
{
    pub sprite: Sprite,
    pub prop_type: PropType,
    pub anim: AnimationState,
}

impl PropRegistry
{
    pub fn sprite_bundle(
        &self,
        prop_type: PropType,
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

        let origin = def.sprite.frame_origin(0);
        let atlas_index = (origin.y * sheet.grid.x + origin.x) as usize;

        return PropSpriteBundle {
            sprite: Sprite::from_atlas_image(
                sheets.images[&def.sheet_id].clone(),
                TextureAtlas { layout: sheets.layouts[&def.sheet_id].clone(), index: atlas_index },
            ),
            prop_type,
            anim: AnimationState::default(),
        };
    }
}

#[derive(Clone, Debug)]
pub enum PropSprite
{
    Static
    {
        // Top-left of the sprite in grid-index coordinates: (0,0), (0,1)...
        origin: UVec2,
    },
    Animated
    {
        // Grid-index origin for each frame, in order.
        frames: Vec<UVec2>,
        // Seconds between frame advances.
        period: f32,
    },
}

impl PropSprite
{
    pub fn frame_count(&self) -> usize
    {
        return match self
        {
            Self::Static { .. } => 1,
            Self::Animated { frames, .. } => frames.len(),
        };
    }

    pub fn frame_origin(&self, frame: usize) -> UVec2
    {
        return match self
        {
            Self::Static { origin } => *origin,
            Self::Animated { frames, .. } => frames[frame % frames.len()],
        };
    }

    pub fn period(&self) -> Option<f32>
    {
        return match self
        {
            Self::Static { .. } => None,
            Self::Animated { period, .. } => Some(*period),
        };
    }
}

#[derive(Clone, Debug)]
pub struct PropDefinition
{
    pub sheet_id: SpritesheetID,
    // Size in world tiles.
    pub size_tiles: UVec2,
    pub sprite: PropSprite,
    pub sample_macro_colors: bool,
    // Layout: macro_colors[frame][tile_row * tile_size.x + tile_col]
    pub macro_colors: Vec<Vec<[u8; 4]>>,
}

impl PropDefinition
{
    pub fn frame_count(&self) -> usize
    {
        return self.sprite.frame_count();
    }
}

#[derive(Component, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum PropType
{
    House,
    HumanAnimation,
}

#[derive(Component, Clone, Debug, Default)]
pub struct AnimationState
{
    pub current_frame: usize,
    pub elapsed: f32,
}

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
            PropType::House,
            PropDefinition {
                sheet_id: SpritesheetID::House,
                size_tiles: UVec2::new(2, 2),
                sprite: PropSprite::Static { origin: UVec2::new(0, 0) },
                sample_macro_colors: true,
                macro_colors: Vec::new(),
            },
        );

        props.insert(
            PropType::HumanAnimation,
            PropDefinition {
                sheet_id: SpritesheetID::HumanImperialWalking,
                size_tiles: UVec2 { x: 1, y: 1 },
                sprite: PropSprite::Animated {
                    frames: vec![
                        UVec2::new(0, 0),
                        UVec2::new(1, 0),
                        UVec2::new(2, 0),
                        UVec2::new(3, 0),
                    ],
                    period: 0.08,
                },
                sample_macro_colors: false,
                macro_colors: Vec::new(),
            },
        );

        return Self { props };
    }
}

impl PropRegistry
{
    pub fn get_prop_data(&self, prop_type: PropType, frame: usize) -> Option<(IVec2, &[[u8; 4]])>
    {
        let def = self.props.get(&prop_type)?;
        if def.macro_colors.is_empty()
        {
            return None;
        }
        let colors = def.macro_colors.get(frame % def.macro_colors.len())?;
        return Some((
            IVec2::new(def.size_tiles.x as i32, def.size_tiles.y as i32),
            colors.as_slice(),
        ));
    }
}

#[derive(Resource, Default)]
pub struct PropSamplingState
{
    pub done: bool,
}

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

        let cell_px = UVec2::new(image.width() / sheet.grid.x, image.height() / sheet.grid.y);
        let mut all_frames: Vec<Vec<[u8; 4]>> = Vec::new();

        for frame_idx in 0 .. def.sprite.frame_count()
        {
            let origin = def.sprite.frame_origin(frame_idx);
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

            all_frames.push(frame_colors);
        }

        def.macro_colors = all_frames;
    }

    state.done = true;
}

// Average RGBA of a rectangular region in an image.
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
            anim.current_frame = (anim.current_frame + 1) % def.frame_count();
        }
    }
}

pub fn sync_sprite_frame(
    prop_registry: Res<PropRegistry>,
    sheet_registry: Res<SpritesheetRegistry>,
    mut query: Query<(&PropType, &AnimationState, &mut Sprite), Changed<AnimationState>>,
)
{
    for (prop_type, anim, mut sprite) in &mut query
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

        let origin = def.sprite.frame_origin(anim.current_frame);
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
