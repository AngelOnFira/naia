
use crate::{HostEntity, OwnedLocalEntity, RemoteEntity};

#[derive(Debug)]
pub struct LocalEntityRecord {
    entity: OwnedLocalEntity,
    old_remote_entity: Option<RemoteEntity>,
    delegated: bool,
}

impl LocalEntityRecord {
    pub fn new_host_owned_entity(entity: HostEntity) -> Self {
        Self {
            entity: OwnedLocalEntity::new_host(entity),
            old_remote_entity: None,
            delegated: false,
        }
    }

    pub fn new_remote_owned_entity(entity: RemoteEntity) -> Self {
        Self {
            entity: OwnedLocalEntity::new_remote(entity),
            old_remote_entity: None,
            delegated: false,
        }
    }

    pub fn is_host_owned(&self) -> bool {
        self.entity.is_host()
    }

    pub fn has_remote_entity(&self) -> bool {
        self.entity.is_remote() || self.old_remote_entity.is_some()
    }

    pub fn is_delegated(&self) -> bool {
        self.delegated
    }

    pub(crate) fn host_entity(&self) -> HostEntity {
        self.entity.host()
    }

    pub(crate) fn remote_entity(&self) -> RemoteEntity {
        if self.entity.is_remote() {
            self.entity.remote()
        } else {
            let Some(old_remote) = self.old_remote_entity else {
                panic!("No remote entity exists");
            };
            old_remote
        }
    }

    pub(crate) fn owned_entity(&self) -> OwnedLocalEntity {
        self.entity
    }

    // should return old host entity if it exists
    pub(crate) fn set_host(&mut self, new_host_entity: HostEntity) -> Option<HostEntity> {

        if self.entity.is_host() {
            panic!("Attempting to set host entity when entity is already host owned");
        }
        if self.old_remote_entity.is_some() {
            panic!("Attempting to set host entity when old remote entity already exists");
        }

        self.old_remote_entity = Some(self.entity.remote());
        self.entity = OwnedLocalEntity::new_host(new_host_entity);

        None
    }

    pub(crate) fn set_remote(&mut self, _remote: RemoteEntity) {
        todo!();
        //     // if let Some(remote_entity) = &self.remote {
        //     //     warn!("Overwriting existing remote entity: {:?}", remote_entity);
        //     // }
        //     self.remote = Some(remote);
        //     self.primary = Some(LocalEntity::Remote);
        // }
    }

    pub(crate) fn clear_remote(&mut self) {
        todo!();
        // if let Some(LocalEntity::Remote) = self.primary {
        //     panic!("Attempting to clear remote entity while it is primary");
        // }
        // if self.remote.is_some() {
        //     self.remote = None;
        // } else {
        //     panic!("Attempting to clear remote entity but no remote entity exists");
        // }
    }

    pub(crate) fn set_host_owned(&mut self) {
        todo!();
        // if self.host.is_some() {
        //     self.primary = Some(LocalEntity::Host);
        // } else {
        //     panic!("Attempting to set primary to host but no host entity exists");
        // }
    }

    pub(crate) fn set_remote_owned(&mut self) {
        todo!();
        // if self.remote.is_some() {
        //     self.primary = Some(LocalEntity::Remote);
        // } else {
        //     panic!("Attempting to set primary to remote but no remote entity exists");
        // }
    }

    pub(crate) fn is_only_remote(&self) -> bool {
        todo!();
        // if let Some(LocalEntity::Remote) = self.primary {
        //     if self.remote.is_some() {
        //         return true;
        //     } else {
        //         panic!("entity record has a remote primary but no remote entity");
        //     }
        // }
        // return false;
    }
}