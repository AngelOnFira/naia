use std::{
    collections::{HashMap, VecDeque},
    time::Duration,
    hash::Hash,
};

use naia_socket_shared::Instant;

use crate::{world::{
    entity::{local_entity::{HostEntity, RemoteEntity}, in_scope_entities::GlobalEntityReserver},
    local_entity_map::LocalEntityMap,
}, GlobalEntity, GlobalEntitySpawner, GlobalWorldManagerType, KeyGenerator, LocalEntityAndGlobalEntityConverter};

pub struct LocalWorldManager {
    user_key: u64,
    host_entity_generator: KeyGenerator<u16>,
    entity_map: LocalEntityMap,
    reserved_host_entities: HashMap<GlobalEntity, HostEntity>,
    reserved_host_entity_ttl: Duration,
    reserved_host_entities_ttls: VecDeque<(Instant, GlobalEntity)>,
}

impl LocalWorldManager {
    pub fn new(user_key: u64) -> Self {
        Self {
            user_key,
            host_entity_generator: KeyGenerator::new(Duration::from_secs(60)),
            entity_map: LocalEntityMap::new(),
            reserved_host_entities: HashMap::new(),
            reserved_host_entity_ttl: Duration::from_secs(60),
            reserved_host_entities_ttls: VecDeque::new(),
        }
    }

    pub fn entity_converter(&self) -> &dyn LocalEntityAndGlobalEntityConverter {
        &self.entity_map
    }

    pub fn global_entity_reserver<'a, 'b, 'c, E: Copy + Eq + Hash + Send + Sync>(
        &'c mut self, global_entity_manager: &'a dyn GlobalWorldManagerType,
        global_entity_spawner: &'b mut dyn GlobalEntitySpawner<E>
    ) -> GlobalEntityReserver<'a, 'b, 'c, E> {
        GlobalEntityReserver::new(global_entity_manager, global_entity_spawner, &mut self.entity_map)
    }

    // Host entities

    pub fn host_reserve_entity(&mut self, global_entity: &GlobalEntity) -> HostEntity {
        self.process_reserved_entity_timeouts();

        if self.reserved_host_entities.contains_key(global_entity) {
            panic!("Global Entity has already reserved Local Entity!");
        }
        let host_entity = self.generate_host_entity();
        if let Some(old_host_entity) = self.entity_map
            .insert_with_host_entity(*global_entity, host_entity) {
            self.recycle_host_entity(old_host_entity);
        }
        self.reserved_host_entities.insert(*global_entity, host_entity);
        host_entity
    }

    fn process_reserved_entity_timeouts(&mut self) {
        let now = Instant::now();

        loop {
            let Some((timeout, _)) = self.reserved_host_entities_ttls.front() else {
                break;
            };
            if timeout.elapsed(&now) < self.reserved_host_entity_ttl {
                break;
            }
            let (_, global_entity) = self.reserved_host_entities_ttls.pop_front().unwrap();
            let Some(_) = self.reserved_host_entities.remove(&global_entity) else {
                panic!("Reserved Entity does not exist!");
            };
        }
    }

    pub fn remove_reserved_host_entity(
        &mut self,
        global_entity: &GlobalEntity,
    ) -> Option<HostEntity> {
        self.reserved_host_entities.remove(global_entity)
    }

    pub(crate) fn generate_host_entity(&mut self) -> HostEntity {
        HostEntity::new(self.host_entity_generator.generate())
    }

    pub(crate) fn insert_host_entity(
        &mut self,
        global_entity: GlobalEntity,
        host_entity: HostEntity,
    ) {
        if self.entity_map.contains_host_entity(&host_entity) {
            panic!("Local Entity already exists!");
        }

        if let Some(old_host_entity) = self.entity_map.insert_with_host_entity(global_entity, host_entity) {
            self.recycle_host_entity(old_host_entity);
        }
    }

    pub fn insert_remote_entity(
        &mut self,
        global_entity: &GlobalEntity,
        remote_entity: RemoteEntity,
    ) {
        self.entity_map
            .insert_with_remote_entity(*global_entity, remote_entity);
    }

    pub(crate) fn remove_by_global_entity(&mut self, global_entity: &GlobalEntity) {
        let record = self
            .entity_map
            .remove_by_global_entity(global_entity)
            .expect("Attempting to despawn entity which does not exist!");
        let host_entity = record.host().unwrap();
        self.recycle_host_entity(host_entity);
    }

    pub fn remove_by_remote_entity(&mut self, remote_entity: &RemoteEntity) -> GlobalEntity {
        let global_entity = *(self
            .entity_map
            .global_entity_from_remote(remote_entity)
            .expect("Attempting to despawn entity which does not exist!"));
        let record = self
            .entity_map
            .remove_by_global_entity(&global_entity)
            .expect("Attempting to despawn entity which does not exist!");
        if let Some(host_entity) = record.host() {
            self.recycle_host_entity(host_entity);
        }
        global_entity
    }

    pub(crate) fn recycle_host_entity(&mut self, host_entity: HostEntity) {
        self.host_entity_generator.recycle_key(&host_entity.value());
    }

    // Remote entities

    pub fn has_remote_entity(&self, remote_entity: &RemoteEntity) -> bool {
        self.entity_map.contains_remote_entity(remote_entity)
    }

    pub(crate) fn global_entity_from_remote(&self, remote_entity: &RemoteEntity) -> GlobalEntity {
        if let Some(global_entity) = self.entity_map.global_entity_from_remote(remote_entity) {
            return *global_entity;
        } else {
            panic!(
                "Attempting to get global entity for local entity which does not exist!: `{:?}`",
                remote_entity
            );
        }
    }

    pub(crate) fn remote_entities(&self) -> Vec<GlobalEntity> {
        self.entity_map
            .iter()
            .filter(|(_, record)| record.is_only_remote())
            .map(|(global_entity, _)| *global_entity)
            .collect::<Vec<GlobalEntity>>()
    }

    // Misc

    pub fn has_both_host_and_remote_entity(&self, global_entity: &GlobalEntity) -> bool {
        self.entity_map
            .has_both_host_and_remote_entity(global_entity)
    }

    pub fn has_global_entity(&self, global_entity: &GlobalEntity) -> bool {
        self.entity_map.contains_global_entity(global_entity)
    }

    pub fn set_primary_to_remote(&mut self, global_entity: &GlobalEntity) {
        self.entity_map.set_primary_to_remote(global_entity);
    }

    pub fn set_primary_to_host(&mut self, global_entity: &GlobalEntity) {
        self.entity_map.set_primary_to_host(global_entity);
    }

    pub fn get_user_key(&self) -> &u64 {
        &self.user_key
    }
}
