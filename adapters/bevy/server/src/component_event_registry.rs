use std::{any::Any, marker::PhantomData, collections::HashMap};

use bevy_app::App;
use bevy_ecs::{entity::Entity, world::World, system::Resource};

use log::warn;

use naia_bevy_shared::{ComponentKind, Replicate};

use naia_server::UserKey;

use crate::events::{InsertComponentEvent, RemoveComponentEvent, UpdateComponentEvent};

// App Extension Methods
pub trait AppRegisterComponentEvents {
    fn add_component_events<C: Replicate>(&mut self) -> &mut Self;
}

impl AppRegisterComponentEvents for App {
    fn add_component_events<C: Replicate>(&mut self) -> &mut Self {

        // add component type to registry
        let mut component_event_registry = self.world_mut().resource_mut::<ComponentEventRegistry>();
        component_event_registry.register_handler::<C>();

        // add events
        self.add_event::<InsertComponentEvent<C>>()
            .add_event::<UpdateComponentEvent<C>>()
            .add_event::<RemoveComponentEvent<C>>();

        self
    }
}

#[derive(Resource)]
pub(crate) struct ComponentEventRegistry {
    handlers: HashMap<ComponentKind, Box<dyn ComponentEventHandler>>,
}

unsafe impl Send for ComponentEventRegistry {}
unsafe impl Sync for ComponentEventRegistry {}

impl Default for ComponentEventRegistry {
    fn default() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }
}

impl ComponentEventRegistry {
    pub fn register_handler<R: Replicate>(
        &mut self,
    ) {
        self.handlers.insert(ComponentKind::of::<R>(), ComponentEventHandlerImpl::<R>::new_boxed());
    }

    pub fn handle_events(&mut self, world: &mut World, events: &mut naia_server::Events<Entity>) {
        // Insert Component Event
        if events.has_inserts() {
            let inserts = events.take_inserts().unwrap();
            for (kind, entities) in inserts {
                let Some(handler) = self.handlers.get_mut(&kind) else {
                    warn!("No insert event handler for ComponentKind: {:?}", kind);
                    continue;
                };
                handler.handle_inserts(world, entities);
            }
        }

        // Update Component Event
        if events.has_updates() {
            let updates = events.take_updates().unwrap();
            for (kind, entities) in updates {
                let Some(handler) = self.handlers.get_mut(&kind) else {
                    warn!("No update event handler for ComponentKind: {:?}", kind);
                    continue;
                };
                handler.handle_updates(world, entities);
            }
        }

        // Remove Component Event
        if events.has_removes() {
            let removes = events.take_removes().unwrap();
            for (kind, entities) in removes {
                let Some(handler) = self.handlers.get_mut(&kind) else {
                    warn!("No remove event handler for ComponentKind: {:?}", kind);
                    continue;
                };
                handler.handle_removes(world, entities);
            }
        }
    }
}

trait ComponentEventHandler: Send + Sync {
    fn handle_inserts(&mut self, world: &mut World, entities: Vec<(UserKey, Entity)>);
    fn handle_updates(&mut self, world: &mut World, entities: Vec<(UserKey, Entity)>);
    fn handle_removes(&mut self, world: &mut World, entities: Vec<(UserKey, Entity, Box<dyn Replicate>)>);
}

struct ComponentEventHandlerImpl<R: Replicate> {
    phantom_r: PhantomData<R>,
}

impl<R: Replicate> ComponentEventHandlerImpl<R> {
    fn new() -> Self {
        Self {
            phantom_r: PhantomData::<R>,
        }
    }

    fn new_boxed() -> Box<dyn ComponentEventHandler> {
        Box::new(Self::new())
    }
}

impl<R: Replicate> ComponentEventHandler for ComponentEventHandlerImpl<R> {
    fn handle_inserts(&mut self, world: &mut World, entities: Vec<(UserKey, Entity)>) {
        for (user_key, entity) in entities {
            world.send_event(InsertComponentEvent::<R>::new(user_key, entity));
        }
    }

    fn handle_updates(&mut self, world: &mut World, entities: Vec<(UserKey, Entity)>) {
        for (user_key, entity) in entities {
            world.send_event(UpdateComponentEvent::<R>::new(user_key, entity));
        }
    }

    fn handle_removes(&mut self, world: &mut World, entities: Vec<(UserKey, Entity, Box<dyn Replicate>)>) {
        for (user_key, entity, boxed_component) in entities {
            let boxed_any = boxed_component.copy_to_box().to_boxed_any();
            let component: R = Box::<dyn Any + 'static>::downcast::<R>(boxed_any)
                .ok()
                .map(|boxed_r| *boxed_r)
                .unwrap();
            world.send_event(RemoveComponentEvent::<R>::new(user_key, entity, component));
        }
    }
}