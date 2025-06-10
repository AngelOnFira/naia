
use crate::{HostEntity, RemoteEntity};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum LocalEntity {
    Host,
    Remote,
}

#[derive(Debug)]
pub struct LocalEntityRecord {
    host: Option<HostEntity>,
    remote: Option<RemoteEntity>,
    primary: Option<LocalEntity>,
}

impl LocalEntityRecord {
    pub fn new_with_host(host: HostEntity) -> Self {
        Self {
            host: Some(host),
            remote: None,
            primary: Some(LocalEntity::Host),
        }
    }

    pub fn new_with_remote(remote: RemoteEntity) -> Self {
        Self {
            host: None,
            remote: Some(remote),
            primary: Some(LocalEntity::Remote),
        }
    }

    pub(crate) fn host(&self) -> Option<HostEntity> {
        self.host
    }

    pub(crate) fn remote(&self) -> Option<RemoteEntity> {
        self.remote
    }

    pub(crate) fn set_host(&mut self, new_host_entity: HostEntity) -> Option<HostEntity> {
        let old_host_entity_opt = self.host.take();
        // if let Some(old_host_entity) = &old_host_entity_opt {
        //     warn!("Overwriting existing host entity: {:?}", old_host_entity);
        // }
        self.host = Some(new_host_entity);
        self.primary = Some(LocalEntity::Host);
        old_host_entity_opt
    }

    pub(crate) fn set_remote(&mut self, remote: RemoteEntity) {
        // if let Some(remote_entity) = &self.remote {
        //     warn!("Overwriting existing remote entity: {:?}", remote_entity);
        // }
        self.remote = Some(remote);
        self.primary = Some(LocalEntity::Remote);
    }

    pub(crate) fn clear_remote(&mut self) {
        if let Some(LocalEntity::Remote) = self.primary {
            panic!("Attempting to clear remote entity while it is primary");
        }
        if self.remote.is_some() {
            self.remote = None;
        } else {
            panic!("Attempting to clear remote entity but no remote entity exists");
        }
    }

    pub(crate) fn set_primary_to_host(&mut self) {
        if self.host.is_some() {
            self.primary = Some(LocalEntity::Host);
        } else {
            panic!("Attempting to set primary to host but no host entity exists");
        }
    }

    pub(crate) fn set_primary_to_remote(&mut self) {
        if self.remote.is_some() {
            self.primary = Some(LocalEntity::Remote);
        } else {
            panic!("Attempting to set primary to remote but no remote entity exists");
        }
    }

    pub(crate) fn is_only_remote(&self) -> bool {
        if let Some(LocalEntity::Remote) = self.primary {
            if self.remote.is_some() {
                return true;
            } else {
                panic!("entity record has a remote primary but no remote entity");
            }
        }
        return false;
    }
}