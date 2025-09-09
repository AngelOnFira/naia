use log::warn;

use crate::{
    messages::channels::receivers::indexed_message_reader::IndexedMessageReader,
    world::{host::host_world_manager::SubCommandId, local_world_manager::LocalWorldManager, entity::local_entity::RemoteEntity},
    BitReader, ComponentKind, ComponentKinds, EntityMessage, EntityMessageType, EntityAuthStatus,
    HostEntity, MessageIndex, Serde, SerdeErr, Tick, OwnedLocalEntity
};

pub struct RemoteWorldReader;

impl RemoteWorldReader {

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
        world_manager: &mut LocalWorldManager,
        component_kinds: &ComponentKinds,
        tick: &Tick,
        reader: &mut BitReader,
    ) -> Result<(), SerdeErr> {
        // read entity updates
        Self::read_updates(world_manager, component_kinds, tick, reader)?;

        // read entity messages
        Self::read_messages(
            world_manager,
            component_kinds,
            reader,
        )?;

        Ok(())
    }

    /// Read incoming Entity messages.
    fn read_messages(
        world_manager: &mut LocalWorldManager,
        component_kinds: &ComponentKinds,
        reader: &mut BitReader,
    ) -> Result<(), SerdeErr> {
        let mut last_read_id: Option<MessageIndex> = None;

        loop {
            // read message continue bit
            let message_continue = bool::de(reader)?;
            if !message_continue {
                break;
            }

            Self::read_message(world_manager, component_kinds, reader, &mut last_read_id)?;
        }

        Ok(())
    }

    /// Read the bits corresponding to the EntityMessage and adds the [`EntityMessage`]
    /// to an internal buffer.
    ///
    /// We can use a UnorderedReliableReceiver buffer because the messages have already been
    /// ordered by the client's jitter buffer
    fn read_message(
        world_manager: &mut LocalWorldManager,
        component_kinds: &ComponentKinds,
        reader: &mut BitReader,
        last_read_id: &mut Option<MessageIndex>,
    ) -> Result<(), SerdeErr> {
        let message_id = Self::read_message_index(reader, last_read_id)?;

        let message_type = EntityMessageType::de(reader)?;

        match message_type {
            EntityMessageType::Spawn => {
                // read remote entity
                let remote_entity = RemoteEntity::de(reader)?;

                world_manager.receiver_buffer_message(
                    message_id,
                    EntityMessage::Spawn(remote_entity.copy_to_owned()),
                );
            }
            EntityMessageType::Despawn => {
                // read local entity
                let local_entity = OwnedLocalEntity::de(reader)?.to_reversed();

                world_manager.receiver_buffer_message(message_id, EntityMessage::Despawn(local_entity));
            }
            EntityMessageType::InsertComponent => {
                // read local entity
                let local_entity = OwnedLocalEntity::de(reader)?.to_reversed();
                
                // read component
                let converter = world_manager.entity_converter();
                let new_component = component_kinds.read(reader, converter)?;
                let new_component_kind = new_component.kind();

                world_manager.receiver_buffer_message(
                    message_id,
                    EntityMessage::InsertComponent(local_entity, new_component_kind),
                );
                world_manager.insert_received_component(
                    &local_entity,
                    &new_component_kind,
                    new_component,
                );
            }
            EntityMessageType::RemoveComponent => {
                // read local entity
                let local_entity = OwnedLocalEntity::de(reader)?.to_reversed();
                
                // read component kind
                let component_kind = ComponentKind::de(component_kinds, reader)?;

                world_manager.receiver_buffer_message(
                    message_id,
                    EntityMessage::RemoveComponent(local_entity, component_kind),
                );
            }
            EntityMessageType::Publish => {

                // read subcommand id
                let sub_command_id = SubCommandId::de(reader)?;

                // read local entity
                let local_entity = OwnedLocalEntity::de(reader)?.to_reversed();

                world_manager.receiver_buffer_message(message_id, EntityMessage::Publish(sub_command_id, local_entity));
            }
            EntityMessageType::Unpublish => {

                // read subcommand id
                let sub_command_id = SubCommandId::de(reader)?;

                // read local entity
                let local_entity = OwnedLocalEntity::de(reader)?.to_reversed();

                world_manager.receiver_buffer_message(message_id, EntityMessage::Unpublish(sub_command_id, local_entity));
            }
            EntityMessageType::EnableDelegation => {

                // read subcommand id
                let sub_command_id = SubCommandId::de(reader)?;

                // read local entity
                let local_entity = OwnedLocalEntity::de(reader)?.to_reversed();

                world_manager.receiver_buffer_message(message_id, EntityMessage::EnableDelegation(sub_command_id, local_entity));
            }
            EntityMessageType::DisableDelegation => {

                // this command is only ever received by clients, regarding server-owned entities

                // read subcommand id
                let sub_command_id = SubCommandId::de(reader)?;

                // read remote entity
                let remote_entity = RemoteEntity::de(reader)?;

                world_manager.receiver_buffer_message(message_id, EntityMessage::DisableDelegation(sub_command_id, remote_entity.copy_to_owned()));
            }
            EntityMessageType::SetAuthority => {
                
                // this command is only ever received by clients, regarding server-owned entities

                // read subcommand id
                let sub_command_id = SubCommandId::de(reader)?;

                // read remote entity
                let remote_entity = RemoteEntity::de(reader)?;

                // read auth status
                let auth_status = EntityAuthStatus::de(reader)?;

                world_manager.receiver_buffer_message(
                    message_id,
                    EntityMessage::SetAuthority(sub_command_id, remote_entity.copy_to_owned(), auth_status),
                );
            }
            
            // below are response-type messages
            EntityMessageType::RequestAuthority => {

                // this command is only read by the server, regarding server-owned entities

                // read subcommand id
                let sub_command_id = SubCommandId::de(reader)?;
                
                // read host entity
                let host_entity = HostEntity::de(reader)?;
                
                // read remote entity value
                let remote_entity = RemoteEntity::de(reader)?;

                world_manager.receiver_buffer_message(
                    message_id,
                    EntityMessage::RequestAuthority(
                        sub_command_id,
                        host_entity.copy_to_owned(),
                        remote_entity,
                    ),
                );
            }
            EntityMessageType::ReleaseAuthority => {

                // this command is only read by the server, regarding server-owned entities

                // read subcommand id
                let sub_command_id = SubCommandId::de(reader)?;

                // read host entity
                let host_entity = HostEntity::de(reader)?;

                world_manager.receiver_buffer_message(message_id, EntityMessage::ReleaseAuthority(sub_command_id, host_entity.copy_to_owned()));
            }
            EntityMessageType::EnableDelegationResponse => {
                
                // this command is only read by the server, regarding server-owned entities

                // read subcommand id
                let sub_command_id = SubCommandId::de(reader)?;

                // read host entity
                let host_entity = HostEntity::de(reader)?;

                world_manager.receiver_buffer_message(message_id, EntityMessage::EnableDelegationResponse(sub_command_id, host_entity.copy_to_owned()));
            }
            EntityMessageType::MigrateResponse => {
                
                // this command is only ever received by clients, regarding newly delegated server-owned entities

                // read subcommand id
                let sub_command_id = SubCommandId::de(reader)?;
                
                // read old host entity
                let old_host_entity = HostEntity::de(reader)?;
                
                // read new remote entity
                let new_host_entity = HostEntity::de(reader)?;

                world_manager.receiver_buffer_message(
                    message_id,
                    EntityMessage::MigrateResponse(
                        sub_command_id,
                        old_host_entity.copy_to_owned(),
                        new_host_entity,
                    ),
                );
            }
            EntityMessageType::Noop => {
                world_manager.receiver_buffer_message(message_id, EntityMessage::Noop);
            }
        }

        Ok(())
    }

    /// Read component updates from raw bits
    fn read_updates(
        world_manager: &mut LocalWorldManager,
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

            Self::read_update(
                world_manager,
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
        world_manager: &mut LocalWorldManager,
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
            if world_manager.contains_remote_entity(remote_entity) {
                let global_entity = *world_manager.global_entity_from_remote(remote_entity).unwrap();

                world_manager.insert_received_update(*tick, &global_entity, component_update);
            } else {
                warn!("read_update(): SKIPPED READ UPDATE!");
            }
        }

        Ok(())
    }
}
