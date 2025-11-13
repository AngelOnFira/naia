use std::{collections::HashMap, hash::Hash};

use crate::{
    world::{
        delegation::{
            auth_channel::{EntityAuthAccessor, EntityAuthChannel, EntityAuthMutator},
            entity_auth_status::{EntityAuthStatus, HostEntityAuthStatus},
        },
        entity::error::EntityAuthError,
    },
    HostType,
};

pub struct HostAuthHandler<E: Copy + Eq + Hash> {
    auth_channels: HashMap<E, (EntityAuthMutator, EntityAuthAccessor)>,
}

impl<E: Copy + Eq + Hash + std::fmt::Debug> HostAuthHandler<E> {
    pub fn new() -> Self {
        Self {
            auth_channels: HashMap::new(),
        }
    }

    /// Register an entity with the auth handler
    ///
    /// # Panics
    ///
    /// Panics if the entity is already registered.
    /// Consider using `try_register_entity` for non-panicking error handling.
    pub fn register_entity(&mut self, host_type: HostType, entity: &E) -> EntityAuthAccessor {
        self.try_register_entity(host_type, entity)
            .expect("Entity cannot register with Server more than once!")
    }

    /// Register an entity with the auth handler
    ///
    /// Returns an error if the entity is already registered.
    pub fn try_register_entity(
        &mut self,
        host_type: HostType,
        entity: &E,
    ) -> Result<EntityAuthAccessor, EntityAuthError> {
        if self.auth_channels.contains_key(&entity) {
            return Err(EntityAuthError::EntityAlreadyRegistered {
                entity_id: format!("{:?}", entity),
            });
        }

        let (mutator, accessor) = EntityAuthChannel::new_channel(host_type);

        self.auth_channels
            .insert(*entity, (mutator, accessor.clone()));

        Ok(accessor)
    }

    pub fn deregister_entity(&mut self, entity: &E) {
        self.auth_channels.remove(&entity);
    }

    /// Get the auth accessor for an entity
    ///
    /// # Panics
    ///
    /// Panics if the entity is not registered.
    /// Consider using `try_get_accessor` for non-panicking error handling.
    pub fn get_accessor(&self, entity: &E) -> EntityAuthAccessor {
        self.try_get_accessor(entity)
            .expect("Entity must be registered with Server before it can receive messages!")
    }

    /// Get the auth accessor for an entity
    ///
    /// Returns an error if the entity is not registered.
    pub fn try_get_accessor(&self, entity: &E) -> Result<EntityAuthAccessor, EntityAuthError> {
        let (_, receiver) = self.auth_channels.get(&entity).ok_or_else(|| {
            EntityAuthError::EntityNotRegistered {
                entity_id: format!("{:?}", entity),
                operation: "get_accessor",
            }
        })?;

        Ok(receiver.clone())
    }

    pub fn auth_status(&self, entity: &E) -> Option<HostEntityAuthStatus> {
        if let Some((_, receiver)) = self.auth_channels.get(&entity) {
            return Some(receiver.auth_status());
        }

        return None;
    }

    /// Set the auth status for an entity
    ///
    /// # Panics
    ///
    /// Panics if the entity is not registered.
    /// Consider using `try_set_auth_status` for non-panicking error handling.
    pub fn set_auth_status(&self, entity: &E, auth_status: EntityAuthStatus) {
        self.try_set_auth_status(entity, auth_status)
            .expect("Entity must be registered with Server before it can be mutated!")
    }

    /// Set the auth status for an entity
    ///
    /// Returns an error if the entity is not registered.
    pub fn try_set_auth_status(
        &self,
        entity: &E,
        auth_status: EntityAuthStatus,
    ) -> Result<(), EntityAuthError> {
        let (sender, _) = self.auth_channels.get(&entity).ok_or_else(|| {
            EntityAuthError::EntityNotRegistered {
                entity_id: format!("{:?}", entity),
                operation: "set_auth_status",
            }
        })?;

        sender.set_auth_status(auth_status);
        Ok(())
    }
}
