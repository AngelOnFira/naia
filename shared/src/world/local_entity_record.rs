
use crate::{HostEntity, OwnedLocalEntity, RemoteEntity};

#[derive(Debug)]
pub struct LocalEntityRecord {
    entity: OwnedLocalEntity,
    delegated: bool,
}

impl LocalEntityRecord {
    pub fn new_host_owned_entity(entity: HostEntity) -> Self {
        Self {
            entity: OwnedLocalEntity::new_host(entity),
            delegated: false,
        }
    }

    pub fn new_remote_owned_entity(entity: RemoteEntity) -> Self {
        Self {
            entity: OwnedLocalEntity::new_remote(entity),
            delegated: false,
        }
    }

    pub fn is_host_owned(&self) -> bool {
        self.entity.is_host()
    }

    pub fn is_remote_owned(&self) -> bool {
        self.entity.is_remote()
    }

    pub fn is_delegated(&self) -> bool {
        self.delegated
    }

    pub(crate) fn host_entity(&self) -> HostEntity {
        self.entity.host()
    }

    pub(crate) fn remote_entity(&self) -> RemoteEntity {
        self.entity.remote()
    }

    pub(crate) fn owned_entity(&self) -> OwnedLocalEntity {
        self.entity
    }

    // should return old host entity if it exists
    pub(crate) fn set_host(&mut self, _new_host_entity: HostEntity) -> Option<HostEntity> {
        todo!();
        // let old_host_entity_opt = self.host.take();
        // // if let Some(old_host_entity) = &old_host_entity_opt {
        // //     warn!("Overwriting existing host entity: {:?}", old_host_entity);
        // // }
        // self.host = Some(new_host_entity);
        // self.primary = Some(LocalEntity::Host);
        // old_host_entity_opt
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