use std::collections::{HashMap, HashSet};

use bevy::prelude::*;

use crate::{
    engine::{
        prop::{MacroSpriteChild, PropRegistry, PropType},
        rendering::color_utils::colorize_image,
        spritesheet::{SpritesheetID, SpritesheetRegistry},
    },
    faction::BuildingColor,
};

#[derive(Clone)]
pub struct CachedVariants
{
    pub main: Handle<Image>,
    pub macro_img: Option<Handle<Image>>,
}

#[derive(Resource, Default)]
pub struct ColoredSpriteCache
{
    cache: HashMap<(SpritesheetID, [u8; 3]), CachedVariants>,
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
    ) -> Option<CachedVariants>
    {
        let key = (sheet_id, color);
        if let Some(variants) = self.cache.get(&key)
        {
            return Some(variants.clone());
        }

        let base_handle = sheets.images.get(&sheet_id)?;
        let base_image = images.get(base_handle)?;
        let new_image = colorize_image(base_image, color);
        let main = images.add(new_image);

        let mut macro_img = None;
        if let Some(base_macro_handle) = sheets.macro_images.get(&sheet_id)
        {
            if let Some(base_macro_image) = images.get(base_macro_handle)
            {
                let new_macro = colorize_image(base_macro_image, color);
                macro_img = Some(images.add(new_macro));
            }
        }

        let variants = CachedVariants { main, macro_img };
        self.cache.insert(key, variants.clone());

        return Some(variants);
    }
}

pub fn apply_building_colors(
    mut cache: ResMut<ColoredSpriteCache>,
    mut images: ResMut<Assets<Image>>,
    prop_registry: Res<PropRegistry>,
    sheet_registry: Res<SpritesheetRegistry>,
    mut query: Query<
        (&PropType, &BuildingColor, &mut Sprite, Option<&MacroSpriteChild>),
        Changed<BuildingColor>,
    >,
    mut child_sprites: Query<&mut Sprite, Without<PropType>>,
)
{
    if sheet_registry.images.is_empty()
    {
        return;
    }

    for (prop_type, bcolor, mut sprite, macro_child) in &mut query
    {
        let Some(def) = prop_registry.props.get(prop_type)
        else
        {
            continue;
        };
        let sheet_id = def.sheet_id;
        let color = bcolor.color;

        if let Some(variants) = cache.get_or_create(sheet_id, color, &mut images, &sheet_registry)
        {
            sprite.image = variants.main;

            if let Some(macro_handle) = variants.macro_img
            {
                if let Some(MacroSpriteChild(child_ent)) = macro_child
                {
                    if let Ok(mut child_sprite) = child_sprites.get_mut(*child_ent)
                    {
                        child_sprite.image = macro_handle;
                    }
                }
            }
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
