use std::{
    clone::Clone,
    collections::{HashMap, HashSet, VecDeque},
    hash::Hash,
};

use crate::{messages::channels::senders::indexed_message_writer::IndexedMessageWriter, world::{
    host::host_world_manager::CommandId,
    entity::entity_converters::GlobalWorldManagerType, local_world_manager::LocalWorldManager,
}, BitWrite, BitWriter, ComponentKind, ComponentKinds, ConstBitLength, EntityMessage, EntityMessageType, EntityAndGlobalEntityConverter, EntityConverterMut, GlobalEntity, HostWorldEvents, HostWorldManager, Instant, MessageIndex, PacketIndex, Serde, WorldRefType, EntityCommand};

pub struct HostWorldWriter;

impl HostWorldWriter {
    fn write_command_id(
        writer: &mut dyn BitWrite,
        last_id_opt: &mut Option<CommandId>,
        current_id: &CommandId,
    ) {
        IndexedMessageWriter::write_message_index(writer, last_id_opt, current_id);
        *last_id_opt = Some(*current_id);
    }

    pub fn write_into_packet<E: Copy + Eq + Hash + Send + Sync, W: WorldRefType<E>>(
        component_kinds: &ComponentKinds,
        now: &Instant,
        writer: &mut BitWriter,
        packet_index: &PacketIndex,
        world: &W,
        entity_converter: &dyn EntityAndGlobalEntityConverter<E>,
        global_world_manager: &dyn GlobalWorldManagerType,
        local_world_manager: &mut LocalWorldManager,
        has_written: &mut bool,
        host_manager: &mut HostWorldManager,
        world_events: &mut HostWorldEvents,
    ) {
        // write entity updates
        Self::write_updates(
            component_kinds,
            now,
            writer,
            &packet_index,
            world,
            entity_converter,
            global_world_manager,
            local_world_manager,
            has_written,
            host_manager,
            &mut world_events.next_send_updates,
        );

        // write entity commands
        Self::write_commands(
            component_kinds,
            now,
            writer,
            &packet_index,
            world,
            entity_converter,
            global_world_manager,
            local_world_manager,
            has_written,
            host_manager,
            &mut world_events.next_send_commands,
        );
    }

    fn write_commands<E: Copy + Eq + Hash + Send + Sync, W: WorldRefType<E>>(
        component_kinds: &ComponentKinds,
        now: &Instant,
        writer: &mut BitWriter,
        packet_index: &PacketIndex,
        world: &W,
        entity_converter: &dyn EntityAndGlobalEntityConverter<E>,
        global_world_manager: &dyn GlobalWorldManagerType,
        local_world_manager: &mut LocalWorldManager,
        has_written: &mut bool,
        host_manager: &mut HostWorldManager,
        next_send_commands: &mut VecDeque<(CommandId, EntityCommand)>,
    ) {
        let mut last_counted_id: Option<MessageIndex> = None;
        let mut last_written_id: Option<MessageIndex> = None;

        loop {
            if next_send_commands.is_empty() {
                break;
            }

            // check that we can write the next message
            let mut counter = writer.counter();
            // write CommandContinue bit
            true.ser(&mut counter);
            // write data
            Self::write_command(
                component_kinds,
                world,
                entity_converter,
                global_world_manager,
                local_world_manager,
                packet_index,
                &mut counter,
                &mut last_counted_id,
                false,
                host_manager,
                next_send_commands,
            );
            if counter.overflowed() {
                // if nothing useful has been written in this packet yet,
                // send warning about size of component being too big
                if !*has_written {
                    Self::warn_overflow_command(
                        component_kinds,
                        counter.bits_needed(),
                        writer.bits_free(),
                        next_send_commands,
                    );
                }
                break;
            }

            *has_written = true;

            // optimization
            host_manager.insert_sent_command_packet(
                packet_index,
                now.clone(),
            );

            // write CommandContinue bit
            true.ser(writer);
            // write data
            Self::write_command(
                component_kinds,
                world,
                entity_converter,
                global_world_manager,
                local_world_manager,
                packet_index,
                writer,
                &mut last_written_id,
                true,
                host_manager,
                next_send_commands,
            );

            // pop command we've written
            next_send_commands.pop_front();
        }

        // Finish commands by writing false CommandContinue bit
        writer.release_bits(1);
        false.ser(writer);
    }

    #[allow(clippy::too_many_arguments)]
    fn write_command<E: Copy + Eq + Hash + Send + Sync, W: WorldRefType<E>>(
        component_kinds: &ComponentKinds,
        world: &W,
        entity_converter: &dyn EntityAndGlobalEntityConverter<E>,
        global_world_manager: &dyn GlobalWorldManagerType,
        local_world_manager: &mut LocalWorldManager,
        packet_index: &PacketIndex,
        writer: &mut dyn BitWrite,
        last_written_id: &mut Option<CommandId>,
        is_writing: bool,
        host_manager: &mut HostWorldManager,
        next_send_commands: &mut VecDeque<(CommandId, EntityCommand)>,
    ) {
        let (command_id, command) = next_send_commands.front().unwrap();

        // write command id
        Self::write_command_id(writer, last_written_id, command_id);

        match command {
            EntityCommand::SpawnEntity(global_entity) => {
                EntityMessageType::SpawnEntity.ser(writer);

                // write net entity
                local_world_manager
                    .entity_converter()
                    .global_entity_to_host_entity(global_entity)
                    .unwrap()
                    .ser(writer);

                // if we are writing to this packet, add it to record
                if is_writing {
                    host_manager.record_command_written(
                        packet_index,
                        command_id,
                        EntityMessage::SpawnEntity(*global_entity),
                    );
                }
            }
            EntityCommand::DespawnEntity(global_entity) => {
                EntityMessageType::DespawnEntity.ser(writer);

                // write net entity
                local_world_manager
                    .entity_converter()
                    .global_entity_to_host_entity(global_entity)
                    .unwrap()
                    .ser(writer);

                // if we are writing to this packet, add it to record
                if is_writing {
                    host_manager.record_command_written(
                        packet_index,
                        command_id,
                        EntityMessage::DespawnEntity(*global_entity),
                    );
                }
            }
            EntityCommand::InsertComponent(global_entity, component_kind) => {
                // get world entity
                let world_entity = entity_converter
                    .global_entity_to_entity(global_entity)
                    .unwrap();

                if !host_manager.host_has_entity(global_entity) || !world.has_component_of_kind(&world_entity, component_kind)
                {
                    EntityMessageType::Noop.ser(writer);

                    // if we are actually writing this packet
                    if is_writing {
                        // add it to command record
                        host_manager.record_command_written(
                            packet_index,
                            command_id,
                            EntityMessage::Noop,
                        );
                    }
                } else {
                    EntityMessageType::InsertComponent.ser(writer);

                    // write net entity
                    local_world_manager
                        .entity_converter()
                        .global_entity_to_host_entity(global_entity)
                        .unwrap()
                        .ser(writer);

                    let mut converter =
                        EntityConverterMut::new(global_world_manager, local_world_manager);

                    // write component payload
                    world
                        .component_of_kind(&world_entity, component_kind)
                        .expect("Component does not exist in World")
                        .write(component_kinds, writer, &mut converter);

                    // if we are actually writing this packet
                    if is_writing {
                        // add it to command record
                        host_manager.record_command_written(
                            packet_index,
                            command_id,
                            EntityMessage::InsertComponent(*global_entity, *component_kind),
                        );
                    }
                }
            }
            EntityCommand::RemoveComponent(global_entity, component_kind) => {
                if !host_manager.host_has_entity(global_entity) {
                    EntityMessageType::Noop.ser(writer);

                    // if we are actually writing this packet
                    if is_writing {
                        // add it to command record
                        host_manager.record_command_written(
                            packet_index,
                            command_id,
                            EntityMessage::Noop,
                        );
                    }
                } else {
                    EntityMessageType::RemoveComponent.ser(writer);

                    // write net entity
                    local_world_manager
                        .entity_converter()
                        .global_entity_to_host_entity(global_entity)
                        .unwrap()
                        .ser(writer);

                    // write component kind
                    component_kind.ser(component_kinds, writer);

                    // if we are writing to this packet, add it to record
                    if is_writing {
                        host_manager.record_command_written(
                            packet_index,
                            command_id,
                            EntityMessage::RemoveComponent(*global_entity, *component_kind),
                        );
                    }
                }
            }
            // Former SystemChannel messages - now serialized as EntityCommandEvents
            EntityCommand::PublishEntity(global_entity) => {
                EntityMessageType::PublishEntity.ser(writer);

                // write net entity
                local_world_manager
                    .entity_converter()
                    .global_entity_to_host_entity(global_entity)
                    .unwrap()
                    .ser(writer);

                // if we are writing to this packet, add it to record
                if is_writing {
                    host_manager.record_command_written(
                        packet_index,
                        command_id,
                        EntityMessage::PublishEntity(*global_entity),
                    );
                }
            }
            EntityCommand::UnpublishEntity(global_entity) => {
                EntityMessageType::UnpublishEntity.ser(writer);

                // write net entity
                local_world_manager
                    .entity_converter()
                    .global_entity_to_host_entity(global_entity)
                    .unwrap()
                    .ser(writer);

                // if we are writing to this packet, add it to record
                if is_writing {
                    host_manager.record_command_written(
                        packet_index,
                        command_id,
                        EntityMessage::UnpublishEntity(*global_entity),
                    );
                }
            }
            EntityCommand::EnableDelegationEntity(global_entity) => {
                EntityMessageType::EnableDelegationEntity.ser(writer);

                // write net entity
                local_world_manager
                    .entity_converter()
                    .global_entity_to_host_entity(global_entity)
                    .unwrap()
                    .ser(writer);

                // if we are writing to this packet, add it to record
                if is_writing {
                    host_manager.record_command_written(
                        packet_index,
                        command_id,
                        EntityMessage::EnableDelegationEntity(*global_entity),
                    );
                }
            }
            EntityCommand::EnableDelegationEntityResponse(global_entity) => {
                EntityMessageType::EnableDelegationEntityResponse.ser(writer);

                // write net entity
                local_world_manager
                    .entity_converter()
                    .global_entity_to_remote_entity(global_entity)
                    .unwrap()
                    .ser(writer);

                // if we are writing to this packet, add it to record
                if is_writing {
                    host_manager.record_command_written(
                        packet_index,
                        command_id,
                        EntityMessage::EnableDelegationEntityResponse(*global_entity),
                    );
                }
            }
            EntityCommand::DisableDelegationEntity(global_entity) => {
                EntityMessageType::DisableDelegationEntity.ser(writer);

                // write net entity
                local_world_manager
                    .entity_converter()
                    .global_entity_to_host_entity(global_entity)
                    .unwrap()
                    .ser(writer);

                // if we are writing to this packet, add it to record
                if is_writing {
                    host_manager.record_command_written(
                        packet_index,
                        command_id,
                        EntityMessage::DisableDelegationEntity(*global_entity),
                    );
                }
            }
            EntityCommand::RequestAuthority(global_entity, host_entity) => {
                EntityMessageType::RequestAuthority.ser(writer);

                // write net entity
                local_world_manager
                    .entity_converter()
                    .global_entity_to_remote_entity(global_entity)
                    .unwrap()
                    .ser(writer);

                // write host entity value
                host_entity.value().ser(writer);

                // if we are writing to this packet, add it to record
                if is_writing {
                    host_manager.record_command_written(
                        packet_index,
                        command_id,
                        EntityMessage::EntityRequestAuthority(*global_entity, *host_entity),
                    );
                }
            }
            EntityCommand::ReleaseAuthority(global_entity) => {
                EntityMessageType::ReleaseAuthority.ser(writer);

                // write net entity
                local_world_manager
                    .entity_converter()
                    .global_entity_to_remote_entity(global_entity)
                    .unwrap()
                    .ser(writer);

                // if we are writing to this packet, add it to record
                if is_writing {
                    host_manager.record_command_written(
                        packet_index,
                        command_id,
                        EntityMessage::EntityReleaseAuthority(*global_entity),
                    );
                }
            }
            EntityCommand::UpdateAuthority(global_entity, auth_status) => {
                EntityMessageType::UpdateAuthority.ser(writer);

                // write net entity
                local_world_manager
                    .entity_converter()
                    .global_entity_to_host_entity(global_entity)
                    .unwrap()
                    .ser(writer);

                // write auth status
                auth_status.ser(writer);

                // if we are writing to this packet, add it to record
                if is_writing {
                    host_manager.record_command_written(
                        packet_index,
                        command_id,
                        EntityMessage::EntityUpdateAuthority(*global_entity, *auth_status),
                    );
                }
            }
            EntityCommand::EntityMigrateResponse(global_entity, new_host_entity_value) => {
                EntityMessageType::EntityMigrateResponse.ser(writer);

                // write net entity
                local_world_manager
                    .entity_converter()
                    .global_entity_to_host_entity(global_entity)
                    .unwrap()
                    .ser(writer);

                // write new host entity value
                new_host_entity_value.ser(writer);

                // if we are writing to this packet, add it to record
                if is_writing {
                    host_manager.record_command_written(
                        packet_index,
                        command_id,
                        EntityMessage::EntityMigrateResponse(*global_entity, *new_host_entity_value),
                    );
                }
            }
        }
    }

    fn warn_overflow_command(
        component_kinds: &ComponentKinds,
        bits_needed: u32,
        bits_free: u32,
        next_send_commands: &VecDeque<(CommandId, EntityCommand)>,
    ) {
        let (_command_id, command) = next_send_commands.front().unwrap();

        match command {
            EntityCommand::SpawnEntity(_entity) => {
                panic!(
                    "Packet Write Error: Blocking overflow detected! Entity Spawn message requires {bits_needed} bits, but packet only has {bits_free} bits available! Recommend slimming down these Components."
                )
            }
            EntityCommand::InsertComponent(_entity, component_kind) => {
                let component_name = component_kinds.kind_to_name(component_kind);
                panic!(
                    "Packet Write Error: Blocking overflow detected! Component Insertion message of type `{component_name}` requires {bits_needed} bits, but packet only has {bits_free} bits available! This condition should never be reached, as large Messages should be Fragmented in the Reliable channel"
                )
            }
            EntityCommand::PublishEntity(_)
            | EntityCommand::UnpublishEntity(_)
            | EntityCommand::EnableDelegationEntity(_)
            | EntityCommand::EnableDelegationEntityResponse(_)
            | EntityCommand::DisableDelegationEntity(_)
            | EntityCommand::RequestAuthority(_, _)
            | EntityCommand::ReleaseAuthority(_)
            | EntityCommand::UpdateAuthority(_, _)
            | EntityCommand::EntityMigrateResponse(_, _) => {
                panic!(
                    "Packet Write Error: Blocking overflow detected! Authority/delegation command requires {bits_needed} bits, but packet only has {bits_free} bits available! These messages should be small and not cause overflow."
                )
            }
            _ => {
                panic!(
                    "Packet Write Error: Blocking overflow detected! Command requires {bits_needed} bits, but packet only has {bits_free} bits available! This message should never display..."
                )
            }
        }
    }

    fn write_updates<E: Copy + Eq + Hash + Send + Sync, W: WorldRefType<E>>(
        component_kinds: &ComponentKinds,
        now: &Instant,
        writer: &mut BitWriter,
        packet_index: &PacketIndex,
        world: &W,
        converter: &dyn EntityAndGlobalEntityConverter<E>,
        global_world_manager: &dyn GlobalWorldManagerType,
        local_world_manager: &mut LocalWorldManager,
        has_written: &mut bool,
        host_manager: &mut HostWorldManager,
        next_send_updates: &mut HashMap<GlobalEntity, HashSet<ComponentKind>>,
    ) {
        let all_update_entities: Vec<GlobalEntity> = next_send_updates.keys().copied().collect();

        for global_entity in all_update_entities {
            // get LocalEntity
            let host_entity = local_world_manager
                .entity_converter()
                .global_entity_to_host_entity(&global_entity)
                .unwrap();

            // get World Entity
            let world_entity = converter.global_entity_to_entity(&global_entity).unwrap();

            // check that we can at least write a LocalEntity and a ComponentContinue bit
            let mut counter = writer.counter();
            // reserve ComponentContinue bit
            counter.write_bit(true);
            // write UpdateContinue bit
            counter.write_bit(true);
            // write LocalEntity
            host_entity.ser(&mut counter);
            if counter.overflowed() {
                break;
            }

            // reserve ComponentContinue bit
            writer.reserve_bits(1);
            // write UpdateContinue bit
            true.ser(writer);
            // write HostEntity
            host_entity.ser(writer);
            // write Components
            Self::write_update(
                component_kinds,
                now,
                world,
                global_world_manager,
                local_world_manager,
                packet_index,
                writer,
                &global_entity,
                &world_entity,
                has_written,
                host_manager,
                next_send_updates,
            );

            // write ComponentContinue finish bit, release
            writer.release_bits(1);
            false.ser(writer);
        }

        // write EntityContinue finish bit, release
        writer.release_bits(1);
        false.ser(writer);
    }

    /// For a given entity, write component value updates into a packet
    /// Only component values that changed in the internal (naia's) host world will be written
    fn write_update<E: Copy + Eq + Hash + Send + Sync, W: WorldRefType<E>>(
        component_kinds: &ComponentKinds,
        now: &Instant,
        world: &W,
        global_world_manager: &dyn GlobalWorldManagerType,
        local_world_manager: &mut LocalWorldManager,
        packet_index: &PacketIndex,
        writer: &mut BitWriter,
        global_entity: &GlobalEntity,
        world_entity: &E,
        has_written: &mut bool,
        host_manager: &mut HostWorldManager,
        next_send_updates: &mut HashMap<GlobalEntity, HashSet<ComponentKind>>,
    ) {
        let mut written_component_kinds = Vec::new();
        let component_kind_set = next_send_updates.get(global_entity).unwrap();
        for component_kind in component_kind_set {
            // get diff mask
            let diff_mask = host_manager
                .get_diff_mask(global_entity, component_kind)
                .clone();

            let mut converter = EntityConverterMut::new(global_world_manager, local_world_manager);

            // check that we can write the next component update
            let mut counter = writer.counter();
            // write ComponentContinue bit
            true.ser(&mut counter);
            // write component kind
            counter.count_bits(<ComponentKind as ConstBitLength>::const_bit_length());
            // write data
            world
                .component_of_kind(&world_entity, component_kind)
                .expect("Component does not exist in World")
                .write_update(&diff_mask, &mut counter, &mut converter);
            if counter.overflowed() {
                // if nothing useful has been written in this packet yet,
                // send warning about size of component being too big
                if !*has_written {
                    let component_name = component_kinds.kind_to_name(component_kind);
                    Self::warn_overflow_update(
                        component_name,
                        counter.bits_needed(),
                        writer.bits_free(),
                    );
                }

                break;
            }

            *has_written = true;

            // write ComponentContinue bit
            true.ser(writer);
            // write component kind
            component_kind.ser(component_kinds, writer);
            // write data
            world
                .component_of_kind(world_entity, component_kind)
                .expect("Component does not exist in World")
                .write_update(&diff_mask, writer, &mut converter);

            written_component_kinds.push(*component_kind);


            host_manager.record_update(
                now,
                packet_index,
                global_entity,
                component_kind,
                diff_mask,
            );
        }

        let update_kinds = next_send_updates.get_mut(global_entity).unwrap();
        for component_kind in &written_component_kinds {
            update_kinds.remove(component_kind);
        }
        if update_kinds.is_empty() {
            next_send_updates.remove(global_entity);
        }
    }

    fn warn_overflow_update(component_name: String, bits_needed: u32, bits_free: u32) {
        panic!(
            "Packet Write Error: Blocking overflow detected! Data update of Component `{component_name}` requires {bits_needed} bits, but packet only has {bits_free} bits available! Recommended to slim down this Component"
        )
    }
}
