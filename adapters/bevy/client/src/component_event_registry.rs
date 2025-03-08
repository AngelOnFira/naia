use std::{any::Any, marker::PhantomData, collections::HashMap};

use bevy_app::App;
use bevy_ecs::{entity::Entity, world::World, system::Resource};

use naia_bevy_shared::{ComponentKind, Replicate, Tick};

use crate::events::{InsertComponentEvent, RemoveComponentEvent, UpdateComponentEvent};

// App Extension Methods
pub trait AppRegisterComponentEvents {
    fn add_component_events<T: Send + Sync + 'static, C: Replicate>(&mut self) -> &mut Self;
}

impl AppRegisterComponentEvents for App {
    fn add_component_events<T: Send + Sync + 'static, C: Replicate>(&mut self) -> &mut Self {

        // add component type to registry
        let mut component_event_registry = self.world_mut().resource_mut::<ComponentEventRegistry<T>>();
        component_event_registry.register_handler::<C>();

        // add events
        self.add_event::<InsertComponentEvent<T, C>>()
            .add_event::<UpdateComponentEvent<T, C>>()
            .add_event::<RemoveComponentEvent<T, C>>();

        self
    }
}

#[derive(Resource)]
pub(crate) struct ComponentEventRegistry<T: Send + Sync + 'static> {
    handlers: HashMap<ComponentKind, Box<dyn ComponentEventHandler>>,
    phantom_t: PhantomData<T>,
}

unsafe impl<T: Send + Sync + 'static> Send for ComponentEventRegistry<T> {}
unsafe impl<T: Send + Sync + 'static> Sync for ComponentEventRegistry<T> {}

impl<T: Send + Sync + 'static> Default for ComponentEventRegistry<T> {
    fn default() -> Self {
        Self {
            handlers: HashMap::new(),
            phantom_t: PhantomData::<T>,
        }
    }
}

impl<T: Send + Sync + 'static> ComponentEventRegistry<T> {
    pub fn register_handler<R: Replicate>(
        &mut self,
    ) {
        self.handlers.insert(ComponentKind::of::<R>(), ComponentEventHandlerImpl::<T, R>::new_boxed());
    }

    pub fn handle_events(&mut self, world: &mut World, events: &mut naia_client::Events<Entity>) {
        // Insert Component Event
        if events.has_inserts() {
            let inserts = events.take_inserts().unwrap();
            for (kind, entities) in inserts {
                let Some(handler) = self.handlers.get_mut(&kind) else {
                    panic!("No insert event handler for ComponentKind: {:?}", kind);
                };
                handler.handle_inserts(world, entities);
            }
        }

        // Update Component Event
        if events.has_updates() {
            let updates = events.take_updates().unwrap();
            for (kind, entities) in updates {
                let Some(handler) = self.handlers.get_mut(&kind) else {
                    panic!("No update event handler for ComponentKind: {:?}", kind);
                };
                handler.handle_updates(world, entities);
            }
        }

        // Remove Component Event
        if events.has_removes() {
            let removes = events.take_removes().unwrap();
            for (kind, entities) in removes {
                let Some(handler) = self.handlers.get_mut(&kind) else {
                    panic!("No remove event handler for ComponentKind: {:?}", kind);
                };
                handler.handle_removes(world, entities);
            }
        }
    }
}

trait ComponentEventHandler: Send + Sync {
    fn handle_inserts(&mut self, world: &mut World, entities: Vec<Entity>);
    fn handle_updates(&mut self, world: &mut World, entities: Vec<(Tick, Entity)>);
    fn handle_removes(&mut self, world: &mut World, entities: Vec<(Entity, Box<dyn Replicate>)>);
}

struct ComponentEventHandlerImpl<T: Send + Sync + 'static, R: Replicate> {
    phantom_t: PhantomData<T>,
    phantom_r: PhantomData<R>,
}

impl<T: Send + Sync + 'static, R: Replicate> ComponentEventHandlerImpl<T, R> {
    fn new() -> Self {
        Self {
            phantom_t: PhantomData::<T>,
            phantom_r: PhantomData::<R>,
        }
    }

    fn new_boxed() -> Box<dyn ComponentEventHandler> {
        Box::new(Self::new())
    }
}

impl<T: Send + Sync + 'static, R: Replicate> ComponentEventHandler for ComponentEventHandlerImpl<T, R> {
    fn handle_inserts(&mut self, world: &mut World, entities: Vec<Entity>) {
        for entity in entities {
            world.send_event(InsertComponentEvent::<T, R>::new(entity));
        }
    }

    fn handle_updates(&mut self, world: &mut World, entities: Vec<(Tick, Entity)>) {
        for (tick, entity) in entities {
            world.send_event(UpdateComponentEvent::<T, R>::new(tick, entity));
        }
    }

    fn handle_removes(&mut self, world: &mut World, entities: Vec<(Entity, Box<dyn Replicate>)>) {
        for (entity, boxed_component) in entities {
            let boxed_any = boxed_component.copy_to_box().to_boxed_any();
            let component: R = Box::<dyn Any + 'static>::downcast::<R>(boxed_any)
                .ok()
                .map(|boxed_r| *boxed_r)
                .unwrap();
            world.send_event(RemoveComponentEvent::<T, R>::new(entity, component));
        }
    }
}