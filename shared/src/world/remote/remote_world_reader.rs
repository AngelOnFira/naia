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
            // Entity Creation
            EntityMessageType::Spawn => {
                // read entity
                let remote_entity = RemoteEntity::de(reader)?;

                world_manager.receive_message(
                    message_id,
                    EntityMessage::Spawn(remote_entity),
                );
            }
            // Entity Deletion
            EntityMessageType::Despawn => {
                // read all data
                let remote_entity = RemoteEntity::de(reader)?;

                world_manager.receive_message(message_id, EntityMessage::Despawn(remote_entity));
            }
            // Add Component to Entity
            EntityMessageType::InsertComponent => {
                // read all data
                let remote_entity = RemoteEntity::de(reader)?;
                let converter = world_manager.entity_converter();
                let new_component = component_kinds.read(reader, converter)?;
                let new_component_kind = new_component.kind();

                world_manager.receive_message(
                    message_id,
                    EntityMessage::InsertComponent(remote_entity, new_component_kind),
                );
                world_manager.insert_received_component(
                    &remote_entity,
                    &new_component_kind,
                    new_component,
                );
            }
            // Component Removal
            EntityMessageType::RemoveComponent => {
                // read all data
                let remote_entity = RemoteEntity::de(reader)?;
                let component_kind = ComponentKind::de(component_kinds, reader)?;

                world_manager.receive_message(
                    message_id,
                    EntityMessage::RemoveComponent(remote_entity, component_kind),
                );
            }
            // Former SystemChannel messages - now handled as EntityMessages
            // These generate EntityResponseEvent directly instead of going through EntityMessage
            EntityMessageType::Publish => {

                // read subcommand id
                let sub_command_id = SubCommandId::de(reader)?;

                // read entity
                let remote_entity = RemoteEntity::de(reader)?;

                world_manager.receive_message(message_id, EntityMessage::Publish(sub_command_id, remote_entity));
            }
            EntityMessageType::Unpublish => {

                // read subcommand id
                let sub_command_id = SubCommandId::de(reader)?;

                // read entity
                let remote_entity = RemoteEntity::de(reader)?;

                world_manager.receive_message(message_id, EntityMessage::Unpublish(sub_command_id, remote_entity));
            }
            EntityMessageType::EnableDelegation => {

                // read subcommand id
                let sub_command_id = SubCommandId::de(reader)?;

                // read entity
                let remote_entity = RemoteEntity::de(reader)?;

                world_manager.receive_message(message_id, EntityMessage::EnableDelegation(sub_command_id, remote_entity));
            }
            EntityMessageType::EnableDelegationResponse => {

                // read subcommand id
                let sub_command_id = SubCommandId::de(reader)?;

                // read entity
                let remote_entity = RemoteEntity::de(reader)?;

                world_manager.receive_message(message_id, EntityMessage::EnableDelegationResponse(sub_command_id, remote_entity));
            }
            EntityMessageType::MigrateResponse => {

                // read subcommand id
                let sub_command_id = SubCommandId::de(reader)?;
                // read entity
                let remote_entity = RemoteEntity::de(reader)?;
                // read new host entity value
                let new_host_entity_value = u16::de(reader)?;

                world_manager.receive_message(
                    message_id,
                    EntityMessage::MigrateResponse(
                        sub_command_id,
                        remote_entity,
                        HostEntity::new(new_host_entity_value),
                    ),
                );
            }
            EntityMessageType::RequestAuthority => {

                // read subcommand id
                let sub_command_id = SubCommandId::de(reader)?;
                // read entity
                let remote_entity = RemoteEntity::de(reader)?;
                // read remote entity value
                let remote_entity_value = u16::de(reader)?;

                world_manager.receive_message(
                    message_id,
                    EntityMessage::RequestAuthority(
                        sub_command_id,
                        remote_entity,
                        RemoteEntity::new(remote_entity_value),
                    ),
                );
            }
            EntityMessageType::ReleaseAuthority => {

                // read subcommand id
                let sub_command_id = SubCommandId::de(reader)?;

                // read entity
                let owned_entity = OwnedLocalEntity::de(reader)?;

                world_manager.receive_message(message_id, EntityMessage::ReleaseAuthority(sub_command_id, owned_entity));
            }
            EntityMessageType::DisableDelegation => {

                // read subcommand id
                let sub_command_id = SubCommandId::de(reader)?;

                // read entity
                let remote_entity = RemoteEntity::de(reader)?;

                world_manager.receive_message(message_id, EntityMessage::DisableDelegation(sub_command_id, remote_entity));
            }
            EntityMessageType::SetAuthority => {

                // read subcommand id
                let sub_command_id = SubCommandId::de(reader)?;

                // read entity
                let remote_entity = RemoteEntity::de(reader)?;

                // read auth status
                let auth_status = EntityAuthStatus::de(reader)?;

                world_manager.receive_message(
                    message_id,
                    EntityMessage::SetAuthority(sub_command_id, remote_entity, auth_status),
                );
            }
            EntityMessageType::Noop => {
                world_manager.receive_message(message_id, EntityMessage::Noop);
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
