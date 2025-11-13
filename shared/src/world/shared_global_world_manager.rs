use std::hash::Hash;
use std::marker::PhantomData;

use crate::{EntityError, EntityEvent, GlobalWorldManagerType, WorldMutType};

pub struct SharedGlobalWorldManager<E: Copy + Eq + Hash + Send + Sync> {
    phantom_e: PhantomData<E>,
}

impl<E: Copy + Eq + Hash + Send + Sync> SharedGlobalWorldManager<E> {
    /// Attempts to despawn all entities and generate corresponding events.
    ///
    /// Returns an error if there's an internal consistency issue between the
    /// GlobalWorldManager's component list and the actual world state.
    ///
    /// Consider using this method instead of `despawn_all_entities` for non-panicking error handling.
    pub fn try_despawn_all_entities<W: WorldMutType<E>>(
        world: &mut W,
        global_world_manager: &dyn GlobalWorldManagerType<E>,
        entities: Vec<E>,
    ) -> Result<Vec<EntityEvent<E>>, EntityError> {
        let mut output = Vec::new();

        for entity in entities {
            // Generate remove event for each component, handing references off just in
            // case
            if let Some(component_kinds) = global_world_manager.component_kinds(&entity) {
                for component_kind in component_kinds {
                    if let Some(component) =
                        world.remove_component_of_kind(&entity, &component_kind)
                    {
                        output.push(EntityEvent::<E>::RemoveComponent(entity, component));
                    } else {
                        return Err(EntityError::InternalConsistency {
                            context: "Global World Manager component list out of sync with world state",
                        });
                    }
                }
            }

            // Generate despawn event
            output.push(EntityEvent::DespawnEntity(entity));

            // Despawn entity
            world.despawn_entity(&entity);
        }

        Ok(output)
    }

    /// Despawns all entities and generates corresponding events.
    ///
    /// # Panics
    ///
    /// Panics if there's an internal consistency issue between the GlobalWorldManager's
    /// component list and the actual world state.
    ///
    /// Consider using `try_despawn_all_entities` for non-panicking error handling.
    pub fn despawn_all_entities<W: WorldMutType<E>>(
        world: &mut W,
        global_world_manager: &dyn GlobalWorldManagerType<E>,
        entities: Vec<E>,
    ) -> Vec<EntityEvent<E>> {
        Self::try_despawn_all_entities(world, global_world_manager, entities)
            .expect("Global World Manager must not have an accurate component list")
    }
}
