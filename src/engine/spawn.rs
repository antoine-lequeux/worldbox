use bevy::prelude::*;

use crate::engine::{
    coords::{GridPos, grid_to_prop_world, sync_grid_positions},
    prop::{PropRegistry, PropType},
    rendering::StandardRenderLayer,
    spritesheet::{AtlasLayoutState, SpritesheetRegistry},
};

type DeferredSpawner = Box<dyn FnOnce(&mut World) + Send + Sync>;

#[derive(Resource, Default)]
struct DeferredSpawns(Vec<DeferredSpawner>);

struct SpawnProp<B: Bundle>
{
    prop_type: PropType,
    pos: GridPos,
    extra: B,
}

impl<B: Bundle> Command for SpawnProp<B>
{
    fn apply(self, world: &mut World)
    {
        if !world.resource::<AtlasLayoutState>().done
        {
            world
                .resource_mut::<DeferredSpawns>()
                .0
                .push(Box::new(move |world| {
                    spawn_prop_inner(world, self.prop_type, self.pos, self.extra);
                }));
            return;
        }
        spawn_prop_inner(world, self.prop_type, self.pos, self.extra);
    }
}

fn spawn_prop_inner<B: Bundle>(world: &mut World, prop_type: PropType, pos: GridPos, extra: B)
{
    let (bundle, world_pos) = {
        let prop_registry = world.resource::<PropRegistry>();
        let sheet_registry = world.resource::<SpritesheetRegistry>();
        let size_tiles = prop_registry
            .props
            .get(&prop_type)
            .map(|d| d.size_tiles)
            .unwrap_or(UVec2::ONE);
        let bundle = prop_registry.sprite_bundle(prop_type, sheet_registry);
        let world_pos = grid_to_prop_world(*pos, size_tiles);
        (bundle, world_pos)
    };
    world.spawn((bundle, Transform::from_translation(world_pos), pos, StandardRenderLayer, extra));
}

fn flush_deferred_spawns(world: &mut World)
{
    let spawners = std::mem::take(&mut world.resource_mut::<DeferredSpawns>().0);
    for spawner in spawners
    {
        spawner(world);
    }
}

pub trait SpawnPropExt
{
    fn spawn_prop(&mut self, prop_type: PropType, pos: GridPos, extra: impl Bundle);
}

impl SpawnPropExt for Commands<'_, '_>
{
    fn spawn_prop(&mut self, prop_type: PropType, pos: GridPos, extra: impl Bundle)
    {
        self.queue(SpawnProp { prop_type, pos, extra });
    }
}

pub struct SpawnPlugin;

impl Plugin for SpawnPlugin
{
    fn build(&self, app: &mut App)
    {
        app.init_resource::<DeferredSpawns>()
            .add_systems(
                Update,
                flush_deferred_spawns.run_if(|s: Res<AtlasLayoutState>, d: Res<DeferredSpawns>| {
                    s.done && !d.0.is_empty()
                }),
            )
            .add_systems(PostUpdate, sync_grid_positions);
    }
}
