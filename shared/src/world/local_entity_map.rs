use std::collections::HashMap;

use crate::{world::{
    local_entity_record::LocalEntityRecord,
    entity::local_entity::{HostEntity, OwnedLocalEntity, RemoteEntity}}, EntityDoesNotExistError, GlobalEntity, LocalEntityAndGlobalEntityConverter};

pub struct LocalEntityMap {
    global_to_local: HashMap<GlobalEntity, LocalEntityRecord>,
    host_to_global: HashMap<HostEntity, GlobalEntity>,
    remote_to_global: HashMap<RemoteEntity, GlobalEntity>,
}

impl LocalEntityAndGlobalEntityConverter for LocalEntityMap {
    fn global_entity_to_host_entity(
        &self,
        global_entity: &GlobalEntity,
    ) -> Result<HostEntity, EntityDoesNotExistError> {
        if let Some(record) = self.global_to_local.get(global_entity) {
            if let Some(host) = record.host() {
                return Ok(host);
            }
        }
        Err(EntityDoesNotExistError)
    }

    fn global_entity_to_remote_entity(
        &self,
        global_entity: &GlobalEntity,
    ) -> Result<RemoteEntity, EntityDoesNotExistError> {
        if let Some(record) = self.global_to_local.get(global_entity) {
            if let Some(remote) = record.remote() {
                return Ok(remote);
            }
        }
        Err(EntityDoesNotExistError)
    }

    fn global_entity_to_owned_entity(
        &self,
        global_entity: &GlobalEntity,
    ) -> Result<OwnedLocalEntity, EntityDoesNotExistError> {
        if let Some(record) = self.global_to_local.get(global_entity) {
            if let Some(remote) = record.remote() {
                return Ok(OwnedLocalEntity::Remote(remote.value()));
            } else if let Some(host) = record.host() {
                return Ok(OwnedLocalEntity::Host(host.value()));
            }
        }
        Err(EntityDoesNotExistError)
    }

    fn host_entity_to_global_entity(
        &self,
        host_entity: &HostEntity,
    ) -> Result<GlobalEntity, EntityDoesNotExistError> {
        if let Some(global_entity) = self.host_to_global.get(host_entity) {
            return Ok(*global_entity);
        }
        Err(EntityDoesNotExistError)
    }

    fn remote_entity_to_global_entity(
        &self,
        remote_entity: &RemoteEntity,
    ) -> Result<GlobalEntity, EntityDoesNotExistError> {
        if let Some(global_entity) = self.remote_to_global.get(remote_entity) {
            return Ok(*global_entity);
        }
        Err(EntityDoesNotExistError)
    }
}

impl LocalEntityMap {
    pub fn new() -> Self {
        Self {
            global_to_local: HashMap::new(),
            host_to_global: HashMap::new(),
            remote_to_global: HashMap::new(),
        }
    }

    pub fn insert_with_host_entity(&mut self, global_entity: GlobalEntity, host_entity: HostEntity) -> Option<HostEntity> {
        let mut old_host_entity_opt = None;
        if let Some(record) = self.global_to_local.get_mut(&global_entity) {
            if let Some(old_host_entity) = record.set_host(host_entity) {
                old_host_entity_opt = Some(old_host_entity);
            }
        } else {
            self.global_to_local.insert(global_entity, LocalEntityRecord::new_with_host(host_entity));
        }
        self.host_to_global.insert(host_entity, global_entity);
        old_host_entity_opt
    }

    pub fn insert_with_remote_entity(&mut self, global_entity: GlobalEntity, remote: RemoteEntity) {

        if let Some(old_global_entity) = self.remote_to_global.get(&remote) {
            if old_global_entity == &global_entity {
                panic!("Already inserted remote entity {:?} for this global entity: {:?}", remote, global_entity);
            }
            let old_record = self.global_to_local.get_mut(old_global_entity).expect("Expected record for old global entity");
            if old_record.is_only_remote() {
                panic!("Remote entity {:?} is already associated with global entity {:?}, but it is not associated with a host entity. Cannot overwrite.", remote, old_global_entity);
            }
            // remote is using a newly generated remote entity for this global entity
            // but we've kept the old remote entity in the map for another global entity, for trailing messages to be able to map entityproperties
            // at this point, those trailing messages are probably already processed
            // so, clear the remote entity from the old global entity record
            old_record.clear_remote();
            self.remote_to_global.remove(&remote);
        }

        if let Some(record) = self.global_to_local.get_mut(&global_entity) {
            record.set_remote(remote);
        } else {
            self.global_to_local
                .insert(global_entity, LocalEntityRecord::new_with_remote(remote));
        }
        self.remote_to_global.insert(remote, global_entity);
    }

    pub fn global_entity_from_remote(&self, remote_entity: &RemoteEntity) -> Option<&GlobalEntity> {
        self.remote_to_global.get(remote_entity)
    }

    pub fn global_entity_from_host(&self, host_entity: &HostEntity) -> Option<&GlobalEntity> {
        self.host_to_global.get(host_entity)
    }

    pub fn remove_by_global_entity(&mut self, global_entity: &GlobalEntity) -> Option<LocalEntityRecord> {
        // info!("Removing global entity: {:?}", global_entity);
        let record_opt = self.global_to_local.remove(global_entity);
        if let Some(record) = &record_opt {
            if let Some(host) = record.host() {
                self.host_to_global.remove(&host);
            }
            if let Some(remote) = record.remote() {
                self.remote_to_global.remove(&remote);
            }
        }
        record_opt
    }

    pub fn set_primary_to_remote(
        &mut self,
        global_entity: &GlobalEntity,
    ) {
        let Some(record) = self.global_to_local.get_mut(global_entity) else {
            panic!("no record exists for entity");
        };
        if record.host().is_some() && record.remote().is_some() {
            record.set_primary_to_remote();
        } else {
            panic!("record does not have dual host and remote entity");
        }
    }

    pub fn set_primary_to_host(&mut self, global_entity: &GlobalEntity) {
        let Some(record) = self.global_to_local.get_mut(global_entity) else {
            panic!("no record exists for entity");
        };
        if record.host().is_some() && record.remote().is_some() {
            record.set_primary_to_host();
        } else {
            panic!("record does not have dual host and remote entity");
        }
    }

    pub fn has_both_host_and_remote_entity(&self, global_entity: &GlobalEntity) -> bool {
        if let Some(record) = self.global_to_local.get(global_entity) {
            if record.host().is_some() && record.remote().is_some() {
                return true;
            }
        }
        return false;
    }

    pub fn contains_global_entity(&self, global_entity: &GlobalEntity) -> bool {
        self.global_to_local.contains_key(global_entity)
    }

    pub fn contains_host_entity(&self, host_entity: &HostEntity) -> bool {
        self.host_to_global.contains_key(host_entity)
    }

    pub fn contains_remote_entity(&self, remote_entity: &RemoteEntity) -> bool {
        self.remote_to_global.contains_key(remote_entity)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&GlobalEntity, &LocalEntityRecord)> {
        self.global_to_local.iter()
    }
}