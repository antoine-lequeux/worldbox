use bevy::prelude::*;

use super::{PropRegistry, PropType};
use crate::engine::{
    coords::GridPos,
    mapgen::MapData,
    rendering::{
        StandardRenderLayer,
        macro_map::{MacroRenderLayer, MapMode},
    },
    spritesheet::{AtlasLayoutState, SpritesheetRegistry},
};

// Boxed closure that spawns a prop once atlas layouts are ready.
type DeferredSpawner = Box<dyn FnOnce(&mut World) + Send + Sync>;

// Queue of prop spawn operations waiting for atlas layouts to be built.
#[derive(Resource, Default)]
struct DeferredSpawns(Vec<DeferredSpawner>);

// Command that spawns a prop entity, deferring if atlases aren't ready yet.
struct SpawnProp<B: Bundle>
{
    prop_type: PropType,
    pos: GridPos,
    // 0 for random
    variation: u32,
    extra: B,
}

impl<B: Bundle> Command for SpawnProp<B>
{
    fn apply(self, world: &mut World)
    {
        let variation_count = world
            .resource::<PropRegistry>()
            .props
            .get(&self.prop_type)
            .map(|d| d.variation_count)
            .unwrap_or(1)
            .max(1);

        let resolved = if self.variation == 0
        {
            rand::random::<u32>() % variation_count
        }
        else
        {
            (self.variation - 1).min(variation_count - 1)
        };

        if !world.resource::<AtlasLayoutState>().done
        {
            world
                .resource_mut::<DeferredSpawns>()
                .0
                .push(Box::new(move |world| {
                    spawn_prop_inner(world, self.prop_type, self.pos, resolved, self.extra);
                }));
            return;
        }
        spawn_prop_inner(world, self.prop_type, self.pos, resolved, self.extra);
    }
}

fn spawn_prop_inner<B: Bundle>(
    world: &mut World,
    prop_type: PropType,
    pos: GridPos,
    variation: u32,
    extra: B,
)
{
    let map_mode = *world.resource::<State<MapMode>>().get();
    let standard_vis =
        if map_mode == MapMode::Standard { Visibility::Inherited } else { Visibility::Hidden };
    let macro_vis =
        if map_mode == MapMode::Macro { Visibility::Inherited } else { Visibility::Hidden };

    let (bundle, macro_bundle, world_pos) = {
        let prop_registry = world.resource::<PropRegistry>();
        let sheet_registry = world.resource::<SpritesheetRegistry>();
        let map_data = world.resource::<MapData>();

        let def = prop_registry.props.get(&prop_type).unwrap();
        let size_tiles = def.size_tiles;
        let sheet_id = def.sheet_id;

        let bundle = prop_registry.sprite_bundle(prop_type, variation, sheet_registry);
        let world_pos = map_data.grid_to_prop_world(*pos, size_tiles);

        let macro_bundle = sheet_registry.macro_images.get(&sheet_id).map(|macro_img| {
            let layout = sheet_registry.macro_layouts.get(&sheet_id).unwrap().clone();
            let index = bundle.sprite.texture_atlas.as_ref().unwrap().index;
            let ts = map_data.tile_size as f32;
            (
                Sprite {
                    image: macro_img.clone(),
                    custom_size: Some(Vec2::new(
                        size_tiles.x as f32 * ts,
                        size_tiles.y as f32 * ts,
                    )),
                    texture_atlas: Some(TextureAtlas { layout, index }),
                    ..default()
                },
                Transform::from_translation(world_pos.with_z(1.1)),
                MacroRenderLayer,
                macro_vis,
            )
        });

        (bundle, macro_bundle, world_pos)
    };

    let parent = world
        .spawn((
            bundle,
            Transform::from_translation(world_pos),
            pos,
            StandardRenderLayer,
            standard_vis,
            extra,
        ))
        .id();

    if let Some(macro_bundle) = macro_bundle
    {
        let child = world.spawn(macro_bundle).id();
        world
            .entity_mut(parent)
            .insert(super::MacroSpriteChild(child));
    }
}

// Flushes any deferred spawn operations once atlas layouts are available.
fn flush_deferred_spawns(world: &mut World)
{
    let spawners = std::mem::take(&mut world.resource_mut::<DeferredSpawns>().0);
    for spawner in spawners
    {
        spawner(world);
    }
}

// Extension trait for Commands to spawn props by type, grid position, and variation.
pub trait SpawnPropExt
{
    fn spawn_prop(&mut self, prop_type: PropType, pos: GridPos, variation: u32, extra: impl Bundle);
}

impl SpawnPropExt for Commands<'_, '_>
{
    fn spawn_prop(&mut self, prop_type: PropType, pos: GridPos, variation: u32, extra: impl Bundle)
    {
        self.queue(SpawnProp { prop_type, pos, variation, extra });
    }
}

pub struct SpawnPlugin;

impl Plugin for SpawnPlugin
{
    fn build(&self, app: &mut App)
    {
        app.init_resource::<DeferredSpawns>().add_systems(
            Update,
            flush_deferred_spawns.run_if(|s: Res<AtlasLayoutState>, d: Res<DeferredSpawns>| {
                s.done && !d.0.is_empty()
            }),
        );
    }
}
