use std::collections::{HashMap, HashSet};

use bevy::prelude::*;

use crate::{
    engine::{
        prop::{PropRegistry, PropType},
        rendering::color_utils::colorize_image,
        spritesheet::{SpritesheetID, SpritesheetRegistry},
    },
    faction::BuildingColor,
};

#[derive(Resource, Default)]
pub struct ColoredSpriteCache
{
    cache: HashMap<(SpritesheetID, [u8; 3]), Handle<Image>>,
    active_set: HashSet<(SpritesheetID, [u8; 3])>,
}

impl ColoredSpriteCache
{
    pub fn get_or_create(
        &mut self,
        sheet_id: SpritesheetID,
        color: [u8; 3],
        images: &mut Assets<Image>,
        sheets: &SpritesheetRegistry,
    ) -> Option<Handle<Image>>
    {
        let key = (sheet_id, color);
        if let Some(handle) = self.cache.get(&key)
        {
            return Some(handle.clone());
        }

        let base_handle = sheets.images.get(&sheet_id)?;
        let base_image = images.get(base_handle)?;

        let new_image = colorize_image(base_image, color);
        let new_handle = images.add(new_image);

        self.cache.insert(key, new_handle.clone());

        return Some(new_handle);
    }
}

pub fn apply_building_colors(
    mut cache: ResMut<ColoredSpriteCache>,
    mut images: ResMut<Assets<Image>>,
    prop_registry: Res<PropRegistry>,
    sheet_registry: Res<SpritesheetRegistry>,
    mut query: Query<(&PropType, &BuildingColor, &mut Sprite), Changed<BuildingColor>>,
)
{
    if sheet_registry.images.is_empty()
    {
        return;
    }

    for (prop_type, bcolor, mut sprite) in &mut query
    {
        let Some(def) = prop_registry.props.get(prop_type)
        else
        {
            continue;
        };
        let sheet_id = def.sheet_id;
        let color = bcolor.color;

        if let Some(new_handle) = cache.get_or_create(sheet_id, color, &mut images, &sheet_registry)
        {
            sprite.image = new_handle;
        }
    }
}

pub fn gc_colored_sprites(
    mut cache: ResMut<ColoredSpriteCache>,
    query: Query<(&PropType, &BuildingColor)>,
    prop_registry: Res<PropRegistry>,
)
{
    cache.active_set.clear();

    for (prop_type, bcolor) in &query
    {
        if let Some(def) = prop_registry.props.get(prop_type)
        {
            cache.active_set.insert((def.sheet_id, bcolor.color));
        }
    }

    let ColoredSpriteCache { cache: sprite_cache, active_set } = &mut *cache;
    sprite_cache.retain(|key, _| active_set.contains(key));
}
