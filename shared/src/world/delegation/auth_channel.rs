use std::sync::{Arc, RwLock};

use crate::{
    world::{
        delegation::entity_auth_status::{EntityAuthStatus, HostEntityAuthStatus},
        entity::error::EntityAuthError,
    },
    HostType,
};

// EntityAuthChannel
#[derive(Clone)]
pub(crate) struct EntityAuthChannel {
    data: Arc<RwLock<EntityAuthData>>,
}

impl EntityAuthChannel {
    pub(crate) fn new_channel(host_type: HostType) -> (EntityAuthMutator, EntityAuthAccessor) {
        let channel = Self {
            data: Arc::new(RwLock::new(EntityAuthData::new(host_type))),
        };

        let sender = EntityAuthMutator::new(&channel);
        let receiver = EntityAuthAccessor::new(&channel);

        (sender, receiver)
    }

    /// Get the auth status (panicking version)
    ///
    /// # Panics
    ///
    /// Panics if the lock is poisoned.
    /// Consider using `try_auth_status` for non-panicking error handling.
    fn auth_status(&self) -> HostEntityAuthStatus {
        self.try_auth_status()
            .expect("Lock on AuthStatus is held by current thread.")
    }

    /// Get the auth status (non-panicking version)
    ///
    /// Returns an error if the lock is poisoned.
    fn try_auth_status(&self) -> Result<HostEntityAuthStatus, EntityAuthError> {
        let data = self.data.as_ref().read().map_err(|_| EntityAuthError::AuthLockPoisoned)?;
        Ok(data.auth_status())
    }

    /// Set the auth status (panicking version)
    ///
    /// # Panics
    ///
    /// Panics if the lock is poisoned.
    /// Consider using `try_set_auth_status` for non-panicking error handling.
    fn set_auth_status(&self, auth_status: EntityAuthStatus) {
        self.try_set_auth_status(auth_status)
            .expect("Lock on AuthStatus is held by current thread.")
    }

    /// Set the auth status (non-panicking version)
    ///
    /// Returns an error if the lock is poisoned.
    fn try_set_auth_status(&self, auth_status: EntityAuthStatus) -> Result<(), EntityAuthError> {
        let mut data = self.data.as_ref().write().map_err(|_| EntityAuthError::AuthLockPoisoned)?;
        data.set_auth_status(auth_status);
        Ok(())
    }
}

// EntityAuthData
struct EntityAuthData {
    host_type: HostType,
    status: EntityAuthStatus,
}

impl EntityAuthData {
    fn new(host_type: HostType) -> Self {
        let status = match host_type {
            HostType::Server => EntityAuthStatus::Available,
            HostType::Client => EntityAuthStatus::Requested,
        };
        Self { host_type, status }
    }

    fn auth_status(&self) -> HostEntityAuthStatus {
        HostEntityAuthStatus::new(self.host_type, self.status)
    }

    fn set_auth_status(&mut self, auth_status: EntityAuthStatus) {
        self.status = auth_status;
    }
}

// EntityAuthAccessor
#[derive(Clone)]
pub struct EntityAuthAccessor {
    channel: EntityAuthChannel,
}

impl EntityAuthAccessor {
    fn new(channel: &EntityAuthChannel) -> Self {
        Self {
            channel: channel.clone(),
        }
    }

    /// Get the auth status (panicking version)
    ///
    /// # Panics
    ///
    /// Panics if the lock is poisoned.
    /// Consider using `try_auth_status` for non-panicking error handling.
    pub(crate) fn auth_status(&self) -> HostEntityAuthStatus {
        self.channel.auth_status()
    }

    /// Get the auth status (non-panicking version)
    ///
    /// Returns an error if the lock is poisoned.
    pub(crate) fn try_auth_status(&self) -> Result<HostEntityAuthStatus, EntityAuthError> {
        self.channel.try_auth_status()
    }
}

// EntityAuthMutator
// no Clone necessary
pub(crate) struct EntityAuthMutator {
    channel: EntityAuthChannel,
}

impl EntityAuthMutator {
    fn new(channel: &EntityAuthChannel) -> Self {
        Self {
            channel: channel.clone(),
        }
    }

    /// Set the auth status (panicking version)
    ///
    /// # Panics
    ///
    /// Panics if the lock is poisoned.
    /// Consider using `try_set_auth_status` for non-panicking error handling.
    pub(crate) fn set_auth_status(&self, auth_status: EntityAuthStatus) {
        self.channel.set_auth_status(auth_status);
    }

    /// Set the auth status (non-panicking version)
    ///
    /// Returns an error if the lock is poisoned.
    pub(crate) fn try_set_auth_status(&self, auth_status: EntityAuthStatus) -> Result<(), EntityAuthError> {
        self.channel.try_set_auth_status(auth_status)
    }
}
