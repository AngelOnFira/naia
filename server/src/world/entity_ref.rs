use std::hash::Hash;

use naia_shared::{EntityAuthStatus, ReplicaRefWrapper, ReplicatedComponent, WorldRefType};

use crate::{ReplicationConfig, Server};

// EntityRef
pub struct EntityRef<'s, E: Copy + Eq + Hash + Send + Sync + std::fmt::Debug, W: WorldRefType<E>> {
    server: &'s Server<E>,
    world: W,
    entity: E,
}

impl<'s, E: Copy + Eq + Hash + Send + Sync + std::fmt::Debug, W: WorldRefType<E>> EntityRef<'s, E, W> {
    pub fn new(server: &'s Server<E>, world: W, entity: &E) -> Self {
        EntityRef {
            server,
            world,
            entity: *entity,
        }
    }

    pub fn id(&self) -> E {
        self.entity
    }

    pub fn has_component<R: ReplicatedComponent>(&self) -> bool {
        self.world.has_component::<R>(&self.entity)
    }

    pub fn component<R: ReplicatedComponent>(&self) -> Option<ReplicaRefWrapper<'_, R>> {
        self.world.component::<R>(&self.entity)
    }

    pub fn replication_config(&self) -> Option<ReplicationConfig> {
        self.server.entity_replication_config(&self.entity)
    }

    pub fn authority(&self) -> Option<EntityAuthStatus> {
        self.server.entity_authority_status(&self.entity)
    }
}
