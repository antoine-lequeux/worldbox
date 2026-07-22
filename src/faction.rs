use bevy::prelude::*;

use crate::{engine::rendering::MacroMapEntity, entity::Human};

// Identifies which faction an entity belongs to.
// Absence of this component means the entity has no faction.
#[derive(Component, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct FactionId(pub u32);

// Data stored for each faction.
pub struct FactionDef
{
    pub name: String,
    pub color: [u8; 3],
}

// Central resource holding all factions.
#[derive(Resource, Default)]
pub struct FactionRegistry
{
    factions: Vec<Option<FactionDef>>,
    // IDs queued for removal.
    pub(crate) deleted_ids: Vec<u32>,
}

impl FactionRegistry
{
    // Creates a new faction and returns its FactionId.
    pub fn add(&mut self, name: impl Into<String>, color: [u8; 3]) -> FactionId
    {
        let id = self.factions.len() as u32;
        self.factions
            .push(Some(FactionDef { name: name.into(), color }));
        return FactionId(id);
    }

    // Deletes the faction with the given ID.
    pub fn remove(&mut self, id: FactionId)
    {
        if let Some(slot) = self.factions.get_mut(id.0 as usize)
        {
            if slot.is_some()
            {
                *slot = None;
                self.deleted_ids.push(id.0);
            }
        }
    }

    pub fn get(&self, id: FactionId) -> Option<&FactionDef>
    {
        return self.factions.get(id.0 as usize)?.as_ref();
    }

    pub fn get_mut(&mut self, id: FactionId) -> Option<&mut FactionDef>
    {
        return self.factions.get_mut(id.0 as usize)?.as_mut();
    }

    pub fn color(&self, id: FactionId) -> Option<[u8; 3]>
    {
        return self.get(id).map(|f| f.color);
    }

    pub fn active_ids(&self) -> Vec<FactionId>
    {
        return self
            .factions
            .iter()
            .enumerate()
            .filter_map(|(i, slot)| slot.as_ref().map(|_| FactionId(i as u32)))
            .collect();
    }
}

// Stores the current display color of a building entity.
#[derive(Component, Clone, Debug)]
pub struct BuildingColor
{
    pub color: [u8; 3],
}

impl Default for BuildingColor
{
    fn default() -> Self
    {
        Self { color: [200, 200, 200] }
    }
}

// Removes FactionId from all entities whose faction was deleted this frame.
fn handle_faction_deletions(
    mut commands: Commands,
    mut registry: ResMut<FactionRegistry>,
    query: Query<(Entity, &FactionId)>,
)
{
    if registry.deleted_ids.is_empty()
    {
        return;
    }

    for (entity, faction_id) in &query
    {
        if registry.deleted_ids.contains(&faction_id.0)
        {
            commands.entity(entity).remove::<FactionId>();
        }
    }

    // Clear without triggering change detection, so sync_colors_on_registry_change
    // is not executed unnecessarily.
    registry.bypass_change_detection().deleted_ids.clear();
}

// Syncs colors when an entity gains or changes its FactionId component.
fn sync_on_faction_id_change(
    registry: Res<FactionRegistry>,
    mut human_query: Query<(&FactionId, &mut MacroMapEntity), (Changed<FactionId>, With<Human>)>,
    mut building_query: Query<
        (&FactionId, &mut BuildingColor),
        (Changed<FactionId>, Without<Human>),
    >,
)
{
    for (faction_id, mut dot) in &mut human_query
    {
        if let Some(color) = registry.color(*faction_id)
        {
            dot.color = [color[0], color[1], color[2], 255];
        }
    }

    for (faction_id, mut bcolor) in &mut building_query
    {
        if let Some(color) = registry.color(*faction_id)
        {
            bcolor.color = color;
        }
    }
}

// Resyncs all faction members when the FactionRegistry itself changes.
fn sync_colors_on_registry_change(
    registry: Res<FactionRegistry>,
    mut human_query: Query<(&FactionId, &mut MacroMapEntity), With<Human>>,
    mut building_query: Query<(&FactionId, &mut BuildingColor), Without<Human>>,
)
{
    if !registry.is_changed()
    {
        return;
    }

    for (faction_id, mut dot) in &mut human_query
    {
        if let Some(color) = registry.color(*faction_id)
        {
            dot.color = [color[0], color[1], color[2], 255];
        }
    }

    for (faction_id, mut bcolor) in &mut building_query
    {
        if let Some(color) = registry.color(*faction_id)
        {
            bcolor.color = color;
        }
    }
}

pub struct FactionPlugin;

impl Plugin for FactionPlugin
{
    fn build(&self, app: &mut App)
    {
        app.init_resource::<FactionRegistry>().add_systems(
            Update,
            (
                handle_faction_deletions,
                (sync_on_faction_id_change, sync_colors_on_registry_change),
            )
                .chain(),
        );
    }
}
