use std::net::SocketAddr;

use naia_socket_shared::Instant;

use crate::{
    types::{HostType, PacketIndex},
    world::{
        entity::entity_converters::GlobalWorldManagerType,
        host::{
            host_world_manager::HostWorldManager,
            entity_update_manager::EntityUpdateManager
        },
        local_world_manager::LocalWorldManager,
    },
    EntityConverterMut, LocalEntityAndGlobalEntityConverter, PacketNotifiable, RemoteWorldManager
};

pub struct WorldManager {
    pub local: LocalWorldManager,
    pub host: HostWorldManager,
    pub remote: RemoteWorldManager,
    pub updater: EntityUpdateManager,
}

impl WorldManager {
    pub fn new(
        address: &Option<SocketAddr>,
        host_type: HostType,
        user_key: u64,
        global_world_manager: &dyn GlobalWorldManagerType,
    ) -> Self {
        Self {
            local: LocalWorldManager::new(user_key),
            host: HostWorldManager::new(host_type),
            remote: RemoteWorldManager::new(host_type),
            updater: EntityUpdateManager::new(address, global_world_manager),
        }
    }

    pub(crate) fn entity_converter(&self) -> &dyn LocalEntityAndGlobalEntityConverter {
        self.local.entity_converter()
    }

    pub(crate) fn entity_converter_mut<'a, 'b>(
        &'b mut self,
        global_world_manager: &'a dyn GlobalWorldManagerType
    ) -> EntityConverterMut<'a, 'b> {
        EntityConverterMut::new(global_world_manager, &mut self.local)
    }

    pub fn collect_messages(&mut self, now: &Instant, rtt_millis: &f32) {
        self.host
            .handle_dropped_command_packets(now);
        self.updater
            .handle_dropped_update_packets(now, rtt_millis);
    }
}

impl PacketNotifiable for WorldManager {
    fn notify_packet_delivered(&mut self, packet_index: PacketIndex) {
        self.host.notify_packet_delivered(packet_index);
        self.updater.notify_packet_delivered(packet_index);
    }
}