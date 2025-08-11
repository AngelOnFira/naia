use log::warn;

use crate::{
    messages::channels::receivers::indexed_message_reader::IndexedMessageReader,
    world::{entity::local_entity::RemoteEntity, local_world_manager::LocalWorldManager},
    BitReader, ComponentKind, ComponentKinds, EntityMessage, EntityMessageReceiver,
    EntityMessageType, EntityAuthStatus, HostEntity, LocalEntityAndGlobalEntityConverter,
    MessageIndex, Serde, SerdeErr, Tick, HostType, OwnedLocalEntity,
    RemoteWorldManager
};

pub struct RemoteWorldReader {
    receiver: EntityMessageReceiver<RemoteEntity>,
}

impl RemoteWorldReader {
    pub fn new(host_type: HostType) -> Self {
        Self {
            receiver: EntityMessageReceiver::new(host_type),
        }
    }

    pub fn take_incoming_events(&mut self) -> Vec<EntityMessage<RemoteEntity>> {
        self.receiver.receive_messages()
    }

    pub fn track_hosts_redundant_remote_entity(
        &mut self,
        remote_entity: &RemoteEntity,
        component_kinds: &Vec<ComponentKind>,
    ) {
        self.receiver
            .track_hosts_redundant_remote_entity(remote_entity, component_kinds);
    }

    pub fn untrack_hosts_redundant_remote_entity(&mut self, remote_entity: &RemoteEntity) {
        if self.receiver.host_has_redundant_remote_entity(remote_entity) {
            self.receiver.untrack_hosts_redundant_remote_entity(remote_entity);
        }
    }

    // Reading

    fn read_message_index(
        reader: &mut BitReader,
        last_index_opt: &mut Option<MessageIndex>,
    ) -> Result<MessageIndex, SerdeErr> {
        // read index
        let current_index = IndexedMessageReader::read_message_index(reader, last_index_opt)?;

        *last_index_opt = Some(current_index);

        Ok(current_index)
    }

    pub fn read_world_events(
        &mut self,
        local_world_manager: &mut LocalWorldManager,
        remote_world_manager: &mut RemoteWorldManager,
        component_kinds: &ComponentKinds,
        tick: &Tick,
        reader: &mut BitReader,
    ) -> Result<(), SerdeErr> {
        // read entity updates
        self.read_updates(local_world_manager, remote_world_manager, component_kinds, tick, reader)?;

        // read entity messages
        self.read_messages(
            local_world_manager.entity_converter(),
            component_kinds,
            remote_world_manager,
            reader,
        )?;

        Ok(())
    }

    /// Read incoming Entity messages.
    fn read_messages(
        &mut self,
        converter: &dyn LocalEntityAndGlobalEntityConverter,
        component_kinds: &ComponentKinds,
        remote_world_manager: &mut RemoteWorldManager,
        reader: &mut BitReader,
    ) -> Result<(), SerdeErr> {
        let mut last_read_id: Option<MessageIndex> = None;

        loop {
            // read message continue bit
            let message_continue = bool::de(reader)?;
            if !message_continue {
                break;
            }

            self.read_message(converter, component_kinds, remote_world_manager, reader, &mut last_read_id)?;
        }

        Ok(())
    }

    /// Read the bits corresponding to the EntityMessage and adds the [`EntityMessage`]
    /// to an internal buffer.
    ///
    /// We can use a UnorderedReliableReceiver buffer because the messages have already been
    /// ordered by the client's jitter buffer
    fn read_message(
        &mut self,
        converter: &dyn LocalEntityAndGlobalEntityConverter,
        component_kinds: &ComponentKinds,
        remote_world_manager: &mut RemoteWorldManager,
        reader: &mut BitReader,
        last_read_id: &mut Option<MessageIndex>,
    ) -> Result<(), SerdeErr> {
        let message_id = Self::read_message_index(reader, last_read_id)?;

        let message_type = EntityMessageType::de(reader)?;

        match message_type {
            // Entity Creation
            EntityMessageType::Spawn => {
                // read entity
                let remote_entity = RemoteEntity::de(reader)?;

                self.receiver.buffer_message(
                    message_id,
                    EntityMessage::Spawn(remote_entity),
                );
            }
            // Entity Deletion
            EntityMessageType::Despawn => {
                // read all data
                let remote_entity = RemoteEntity::de(reader)?;

                self.receiver
                    .buffer_message(message_id, EntityMessage::Despawn(remote_entity));
            }
            // Add Component to Entity
            EntityMessageType::InsertComponent => {
                // read all data
                let remote_entity = RemoteEntity::de(reader)?;
                let new_component = component_kinds.read(reader, converter)?;
                let new_component_kind = new_component.kind();

                self.receiver.buffer_message(
                    message_id,
                    EntityMessage::InsertComponent(remote_entity, new_component_kind),
                );
                remote_world_manager.insert_received_component(
                    remote_entity,
                    new_component_kind,
                    new_component,
                );
            }
            // Component Removal
            EntityMessageType::RemoveComponent => {
                // read all data
                let remote_entity = RemoteEntity::de(reader)?;
                let component_kind = ComponentKind::de(component_kinds, reader)?;

                self.receiver.buffer_message(
                    message_id,
                    EntityMessage::RemoveComponent(remote_entity, component_kind),
                );
            }
            // Former SystemChannel messages - now handled as EntityMessages
            // These generate EntityResponseEvent directly instead of going through EntityMessage
            EntityMessageType::Publish => {
                // read entity
                let remote_entity = RemoteEntity::de(reader)?;

                self.receiver
                    .buffer_message(message_id, EntityMessage::Publish(remote_entity));
            }
            EntityMessageType::Unpublish => {
                // read entity
                let remote_entity = RemoteEntity::de(reader)?;

                self.receiver
                    .buffer_message(message_id, EntityMessage::Unpublish(remote_entity));
            }
            EntityMessageType::EnableDelegation => {
                // read entity
                let remote_entity = RemoteEntity::de(reader)?;

                self.receiver
                    .buffer_message(message_id, EntityMessage::EnableDelegation(remote_entity));
            }
            EntityMessageType::EnableDelegationResponse => {
                // read entity
                let remote_entity = RemoteEntity::de(reader)?;

                self.receiver
                    .buffer_message(message_id, EntityMessage::EnableDelegationResponse(remote_entity));
            }
            EntityMessageType::MigrateResponse => {
                // read entity
                let remote_entity = RemoteEntity::de(reader)?;
                // read new host entity value
                let new_host_entity_value = u16::de(reader)?;

                self.receiver.buffer_message(
                    message_id,
                    EntityMessage::MigrateResponse(
                        remote_entity,
                        HostEntity::new(new_host_entity_value),
                    ),
                );
            }
            EntityMessageType::RequestAuthority => {
                // read entity
                let remote_entity = RemoteEntity::de(reader)?;
                // read remote entity value
                let remote_entity_value = u16::de(reader)?;

                self.receiver.buffer_message(
                    message_id,
                    EntityMessage::RequestAuthority(
                        remote_entity,
                        RemoteEntity::new(remote_entity_value),
                    ),
                );
            }
            EntityMessageType::ReleaseAuthority => {
                // read entity
                let owned_entity = OwnedLocalEntity::de(reader)?;

                self.receiver.buffer_message(message_id, EntityMessage::ReleaseAuthority(owned_entity));
            }
            EntityMessageType::DisableDelegation => {
                // read entity
                let remote_entity = RemoteEntity::de(reader)?;

                self.receiver
                    .buffer_message(message_id, EntityMessage::DisableDelegation(remote_entity));
            }
            EntityMessageType::SetAuthority => {
                // read entity
                let remote_entity = RemoteEntity::de(reader)?;
                // read auth status
                let auth_status = EntityAuthStatus::de(reader)?;

                self.receiver.buffer_message(
                    message_id,
                    EntityMessage::SetAuthority(remote_entity, auth_status),
                );
            }
            EntityMessageType::Noop => {
                self.receiver.buffer_message(message_id, EntityMessage::Noop);
            }
        }

        Ok(())
    }

    /// Read component updates from raw bits
    fn read_updates(
        &mut self,
        local_world_manager: &LocalWorldManager,
        remote_world_manager: &mut RemoteWorldManager,
        component_kinds: &ComponentKinds,
        tick: &Tick,
        reader: &mut BitReader,
    ) -> Result<(), SerdeErr> {
        loop {
            // read update continue bit
            let update_continue = bool::de(reader)?;
            if !update_continue {
                break;
            }

            let remote_entity = RemoteEntity::de(reader)?;

            self.read_update(
                local_world_manager,
                remote_world_manager,
                component_kinds,
                tick,
                reader,
                &remote_entity,
            )?;
        }

        Ok(())
    }

    /// Read component updates from raw bits for a given entity
    fn read_update(
        &mut self,
        local_world_manager: &LocalWorldManager,
        remote_world_manager: &mut RemoteWorldManager,
        component_kinds: &ComponentKinds,
        tick: &Tick,
        reader: &mut BitReader,
        remote_entity: &RemoteEntity,
    ) -> Result<(), SerdeErr> {
        loop {
            // read update continue bit
            let component_continue = bool::de(reader)?;
            if !component_continue {
                break;
            }

            let component_update = component_kinds.read_create_update(reader)?;

            // At this point, the WorldChannel/EntityReceiver should guarantee the Entity is in scope, correct?
            if local_world_manager.has_remote_entity(remote_entity) {
                let world_entity = local_world_manager.global_entity_from_remote(remote_entity);

                remote_world_manager.insert_received_update(*tick, world_entity, component_update);
            } else {
                warn!("read_update(): SKIPPED READ UPDATE!");
            }
        }

        Ok(())
    }
}
