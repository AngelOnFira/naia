use std::{
    collections::{HashMap, VecDeque},
    hash::Hash,
    time::Duration,
};

use naia_socket_shared::Instant;

use crate::{
    world::{
        entity::{
            error::EntityError,
            local_entity::{HostEntity, OwnedLocalEntity, RemoteEntity},
        },
        local_entity_map::LocalEntityMap,
    },
    EntityAndLocalEntityConverter, EntityDoesNotExistError, KeyGenerator,
};

pub struct LocalWorldManager<E: Copy + Eq + Hash> {
    user_key: u64,
    host_entity_generator: KeyGenerator<u16>,
    entity_map: LocalEntityMap<E>,
    reserved_entities: HashMap<E, HostEntity>,
    reserved_entity_ttl: Duration,
    reserved_entities_ttls: VecDeque<(Instant, E)>,
}

impl<E: Copy + Eq + Hash> LocalWorldManager<E> {
    pub fn new(user_key: u64) -> Self {
        Self {
            user_key,
            host_entity_generator: KeyGenerator::new(Duration::from_secs(60)),
            entity_map: LocalEntityMap::new(),
            reserved_entities: HashMap::new(),
            reserved_entity_ttl: Duration::from_secs(60),
            reserved_entities_ttls: VecDeque::new(),
        }
    }

    // Host entities

    /// Attempts to reserve a host entity for the given world entity.
    ///
    /// Returns an error if the entity has already been reserved.
    /// Consider using this method instead of `host_reserve_entity` for non-panicking error handling.
    pub fn try_host_reserve_entity(&mut self, world_entity: &E) -> Result<HostEntity, EntityError> {
        self.try_process_reserved_entity_timeouts()?;

        if self.reserved_entities.contains_key(world_entity) {
            return Err(EntityError::EntityAlreadyReserved {
                entity_id: "world entity".to_string(),
            });
        }
        let host_entity = self.generate_host_entity();
        self.entity_map
            .insert_with_host_entity(*world_entity, host_entity);
        self.reserved_entities.insert(*world_entity, host_entity);
        self.reserved_entities_ttls.push_back((Instant::now(), *world_entity));
        Ok(host_entity)
    }

    /// Reserves a host entity for the given world entity.
    ///
    /// # Panics
    ///
    /// Panics if the entity has already been reserved.
    /// Consider using `try_host_reserve_entity` for non-panicking error handling.
    pub fn host_reserve_entity(&mut self, world_entity: &E) -> HostEntity {
        self.try_host_reserve_entity(world_entity)
            .expect("World Entity has already reserved Local Entity!")
    }

    /// Attempts to process expired entity reservations.
    ///
    /// Returns an error if the timeout queue is corrupted (entity in queue but not in map).
    fn try_process_reserved_entity_timeouts(&mut self) -> Result<(), EntityError> {
        let now = Instant::now();

        loop {
            let Some((timeout, _)) = self.reserved_entities_ttls.front() else {
                break;
            };
            if timeout.elapsed(&now) < self.reserved_entity_ttl {
                break;
            }
            let (_, world_entity) = self.reserved_entities_ttls.pop_front()
                .ok_or(EntityError::ReservationQueueCorrupted)?;

            if self.reserved_entities.remove(&world_entity).is_none() {
                return Err(EntityError::ReservationExpired {
                    entity_id: "world entity".to_string(),
                });
            }
        }
        Ok(())
    }

    /// Processes expired entity reservations.
    ///
    /// # Panics
    ///
    /// Panics if the reservation timeout queue is corrupted (entity in queue but not in map).
    fn process_reserved_entity_timeouts(&mut self) {
        self.try_process_reserved_entity_timeouts()
            .expect("Reserved Entity does not exist!")
    }

    pub fn remove_reserved_host_entity(&mut self, world_entity: &E) -> Option<HostEntity> {
        self.reserved_entities.remove(world_entity)
    }

    pub(crate) fn generate_host_entity(&mut self) -> HostEntity {
        HostEntity::new(self.host_entity_generator.generate())
    }

    /// Attempts to insert a host entity for the given world entity.
    ///
    /// Returns an error if the host entity already exists.
    pub fn try_insert_host_entity(&mut self, world_entity: E, host_entity: HostEntity) -> Result<(), EntityError> {
        if self.entity_map.contains_host_entity(&host_entity) {
            return Err(EntityError::EntityAlreadyRegisteredAs {
                entity_id: format!("{:?}", host_entity),
                existing_type: "HostEntity",
            });
        }

        self.entity_map
            .insert_with_host_entity(world_entity, host_entity);
        Ok(())
    }

    /// Inserts a host entity for the given world entity.
    ///
    /// # Panics
    ///
    /// Panics if the host entity already exists.
    /// Consider using `try_insert_host_entity` for non-panicking error handling.
    pub(crate) fn insert_host_entity(&mut self, world_entity: E, host_entity: HostEntity) {
        self.try_insert_host_entity(world_entity, host_entity)
            .expect("Local Entity already exists!")
    }

    /// Attempts to insert a remote entity for the given world entity.
    ///
    /// Returns an error if the remote entity already exists.
    pub fn try_insert_remote_entity(&mut self, world_entity: &E, remote_entity: RemoteEntity) -> Result<(), EntityError> {
        if self.entity_map.contains_remote_entity(&remote_entity) {
            return Err(EntityError::EntityAlreadyRegisteredAs {
                entity_id: format!("{:?}", remote_entity),
                existing_type: "RemoteEntity",
            });
        }

        self.entity_map
            .insert_with_remote_entity(*world_entity, remote_entity);
        Ok(())
    }

    /// Inserts a remote entity for the given world entity.
    ///
    /// # Panics
    ///
    /// Panics if the remote entity already exists.
    /// Consider using `try_insert_remote_entity` for non-panicking error handling.
    pub fn insert_remote_entity(&mut self, world_entity: &E, remote_entity: RemoteEntity) {
        self.try_insert_remote_entity(world_entity, remote_entity)
            .expect(&format!("Remote Entity `{:?}` already exists!", remote_entity))
    }

    /// Attempts to remove an entity by its world entity identifier.
    ///
    /// Returns an error if the entity doesn't exist or doesn't have a host entity.
    pub fn try_remove_by_world_entity(&mut self, world_entity: &E) -> Result<(), EntityError> {
        let record = self
            .entity_map
            .remove_by_world_entity(world_entity)
            .ok_or_else(|| EntityError::EntityNotFound {
                context: "attempting to despawn entity which does not exist",
            })?;

        let host_entity = record.host().ok_or_else(|| EntityError::MissingHostEntity {
            entity_id: "world entity".to_string(),
        })?;

        self.recycle_host_entity(host_entity);
        Ok(())
    }

    /// Removes an entity by its world entity identifier.
    ///
    /// # Panics
    ///
    /// Panics if the entity doesn't exist or doesn't have a host entity.
    /// Consider using `try_remove_by_world_entity` for non-panicking error handling.
    pub(crate) fn remove_by_world_entity(&mut self, world_entity: &E) {
        self.try_remove_by_world_entity(world_entity)
            .expect("Attempting to despawn entity which does not exist!")
    }

    /// Attempts to remove an entity by its remote entity identifier.
    ///
    /// Returns the world entity on success, or an error if the entity doesn't exist.
    pub fn try_remove_by_remote_entity(&mut self, remote_entity: &RemoteEntity) -> Result<E, EntityError> {
        let world_entity = *self
            .entity_map
            .world_entity_from_remote(remote_entity)
            .ok_or_else(|| EntityError::EntityNotFound {
                context: "attempting to despawn remote entity which does not exist",
            })?;

        let record = self
            .entity_map
            .remove_by_world_entity(&world_entity)
            .ok_or_else(|| EntityError::EntityNotFound {
                context: "attempting to despawn entity which does not exist",
            })?;

        if let Some(host_entity) = record.host() {
            self.recycle_host_entity(host_entity);
        }
        Ok(world_entity)
    }

    /// Removes an entity by its remote entity identifier.
    ///
    /// # Panics
    ///
    /// Panics if the entity doesn't exist.
    /// Consider using `try_remove_by_remote_entity` for non-panicking error handling.
    pub fn remove_by_remote_entity(&mut self, remote_entity: &RemoteEntity) -> E {
        self.try_remove_by_remote_entity(remote_entity)
            .expect("Attempting to despawn entity which does not exist!")
    }

    pub(crate) fn recycle_host_entity(&mut self, host_entity: HostEntity) {
        self.host_entity_generator.recycle_key(&host_entity.value());
    }

    // Remote entities

    pub fn has_remote_entity(&self, remote_entity: &RemoteEntity) -> bool {
        self.entity_map.contains_remote_entity(remote_entity)
    }

    /// Attempts to get the world entity for a given remote entity.
    ///
    /// Returns an error if the remote entity doesn't exist.
    pub fn try_world_entity_from_remote(&self, remote_entity: &RemoteEntity) -> Result<E, EntityError> {
        self.entity_map
            .world_entity_from_remote(remote_entity)
            .copied()
            .ok_or_else(|| EntityError::EntityNotFound {
                context: "attempting to get world entity for remote entity which does not exist",
            })
    }

    /// Gets the world entity for a given remote entity.
    ///
    /// # Panics
    ///
    /// Panics if the remote entity doesn't exist.
    /// Consider using `try_world_entity_from_remote` for non-panicking error handling.
    pub(crate) fn world_entity_from_remote(&self, remote_entity: &RemoteEntity) -> E {
        self.try_world_entity_from_remote(remote_entity)
            .expect(&format!(
                "Attempting to get world entity for local entity which does not exist!: `{:?}`",
                remote_entity
            ))
    }

    pub(crate) fn remote_entities(&self) -> Vec<E> {
        self.entity_map
            .iter()
            .filter(|(_, record)| record.is_only_remote())
            .map(|(world_entity, _)| *world_entity)
            .collect::<Vec<E>>()
    }

    // Misc

    pub fn has_both_host_and_remote_entity(&self, world_entity: &E) -> bool {
        self.entity_map
            .has_both_host_and_remote_entity(world_entity)
    }

    pub fn has_world_entity(&self, world_entity: &E) -> bool {
        self.entity_map.contains_world_entity(world_entity)
    }

    pub fn remove_redundant_host_entity(&mut self, world_entity: &E) {
        if let Some(host_entity) = self.entity_map.remove_redundant_host_entity(world_entity) {
            self.recycle_host_entity(host_entity);
        }
    }

    pub fn remove_redundant_remote_entity(&mut self, world_entity: &E) -> RemoteEntity {
        self.entity_map.remove_redundant_remote_entity(world_entity)
    }

    pub fn get_user_key(&self) -> &u64 {
        &self.user_key
    }
}

impl<E: Copy + Eq + Hash> EntityAndLocalEntityConverter<E> for LocalWorldManager<E> {
    fn entity_to_host_entity(
        &self,
        world_entity: &E,
    ) -> Result<HostEntity, EntityDoesNotExistError> {
        if let Some(local_entity) = self.entity_map.get_host_entity(world_entity) {
            return Ok(local_entity);
        } else {
            return Err(EntityDoesNotExistError);
        }
    }

    fn entity_to_remote_entity(
        &self,
        world_entity: &E,
    ) -> Result<RemoteEntity, EntityDoesNotExistError> {
        if let Some(local_entity) = self.entity_map.get_remote_entity(world_entity) {
            return Ok(local_entity);
        } else {
            return Err(EntityDoesNotExistError);
        }
    }

    fn entity_to_owned_entity(
        &self,
        world_entity: &E,
    ) -> Result<OwnedLocalEntity, EntityDoesNotExistError> {
        if let Some(local_entity) = self.entity_map.get_owned_entity(world_entity) {
            return Ok(local_entity);
        } else {
            return Err(EntityDoesNotExistError);
        }
    }

    fn host_entity_to_entity(
        &self,
        host_entity: &HostEntity,
    ) -> Result<E, EntityDoesNotExistError> {
        if let Some(entity) = self.entity_map.world_entity_from_host(host_entity) {
            return Ok(*entity);
        } else {
            return Err(EntityDoesNotExistError);
        }
    }

    fn remote_entity_to_entity(
        &self,
        remote_entity: &RemoteEntity,
    ) -> Result<E, EntityDoesNotExistError> {
        if let Some(entity) = self.entity_map.world_entity_from_remote(remote_entity) {
            return Ok(*entity);
        } else {
            return Err(EntityDoesNotExistError);
        }
    }
}
