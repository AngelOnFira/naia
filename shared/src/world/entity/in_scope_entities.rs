use std::{hash::Hash, collections::HashSet};

use crate::{world::local_entity_map::LocalEntityMap, EntityDoesNotExistError, GlobalEntity, GlobalEntitySpawner, HostEntity, LocalEntityAndGlobalEntityConverter, OwnedLocalEntity, RemoteEntity};

pub trait InScopeEntities {
    fn has_entity(&self, global_entity: &GlobalEntity) -> bool;
}

pub trait InScopeEntitiesMut: InScopeEntities + LocalEntityAndGlobalEntityConverter {
    fn get_or_reserve_global_entity_set_from_remote_entity_set(
        &mut self,
        remote_entities: HashSet<RemoteEntity>,
    ) -> Result<HashSet<GlobalEntity>, EntityDoesNotExistError>;
}

pub struct GlobalEntityReserver<'a, 'b, 'c, E: Copy + Eq + Hash + Send + Sync> {
    global_world_manager: &'a dyn InScopeEntities,
    global_entity_spawner: &'b mut dyn GlobalEntitySpawner<E>,
    local_entity_map: &'c mut LocalEntityMap,
}

impl<'a, 'b, 'c, E: Copy + Eq + Hash + Send + Sync> GlobalEntityReserver<'a, 'b, 'c, E> {
    pub fn new(
        global_world_manager: &'a dyn InScopeEntities,
        global_entity_spawner: &'b mut dyn GlobalEntitySpawner<E>,
        local_entity_map: &'c mut LocalEntityMap,
    ) -> Self {
        Self {
            global_world_manager,
            global_entity_spawner,
            local_entity_map,
        }
    }
}

impl<'a, 'b, 'c, E: Copy + Eq + Hash + Send + Sync> LocalEntityAndGlobalEntityConverter for GlobalEntityReserver<'a, 'b, 'c, E> {
    fn global_entity_to_host_entity(&self, global_entity: &GlobalEntity) -> Result<HostEntity, EntityDoesNotExistError> {
        self.local_entity_map.global_entity_to_host_entity(global_entity)
    }

    fn global_entity_to_remote_entity(&self, global_entity: &GlobalEntity) -> Result<RemoteEntity, EntityDoesNotExistError> {
        self.local_entity_map.global_entity_to_remote_entity(global_entity)
    }

    fn global_entity_to_owned_entity(&self, global_entity: &GlobalEntity) -> Result<OwnedLocalEntity, EntityDoesNotExistError> {
        self.local_entity_map.global_entity_to_owned_entity(global_entity)
    }

    fn host_entity_to_global_entity(&self, host_entity: &HostEntity) -> Result<GlobalEntity, EntityDoesNotExistError> {
        self.local_entity_map.host_entity_to_global_entity(host_entity)
    }

    fn remote_entity_to_global_entity(&self, remote_entity: &RemoteEntity) -> Result<GlobalEntity, EntityDoesNotExistError> {
        self.local_entity_map.remote_entity_to_global_entity(remote_entity)
    }
}

impl<'a, 'b, 'c, E: Copy + Eq + Hash + Send + Sync> InScopeEntities for GlobalEntityReserver<'a, 'b, 'c, E> {
    fn has_entity(&self, global_entity: &GlobalEntity) -> bool {
        self.global_world_manager.has_entity(global_entity)
    }
}

impl<'a, 'b, 'c, E: Copy + Eq + Hash + Send + Sync> InScopeEntitiesMut for GlobalEntityReserver<'a, 'b, 'c, E> {
    fn get_or_reserve_global_entity_set_from_remote_entity_set(
        &mut self,
        remote_entities: HashSet<RemoteEntity>,
    ) -> Result<HashSet<GlobalEntity>, EntityDoesNotExistError> {
        let mut global_entities = HashSet::new();
        for remote_entity in remote_entities {
            if let Ok(global_entity) = self.local_entity_map.remote_entity_to_global_entity(&remote_entity) {
                global_entities.insert(global_entity);
            } else {
                let global_entity = self.global_entity_spawner.reserve_global_entity(remote_entity);
                self.local_entity_map.insert_with_remote_entity(global_entity, remote_entity);
                global_entities.insert(global_entity);
            }
    }
        Ok(global_entities)
    }
}