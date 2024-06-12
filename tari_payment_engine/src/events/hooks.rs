use std::{future::Future, pin::Pin, sync::Arc};

use crate::events::{EventHandler, EventProducer, Handler, OrderAnnulledEvent, OrderModifiedEvent, OrderPaidEvent};

/// A container struct for holding event producers for the different event types.
///
/// An EventProducer is a struct that can be used to send events to a handler (it's a thin wrapper around an mpsc
/// sender). You don't create this struct directly, but rather use the [`EventHandlers::producers`] method to generate
/// it.
#[derive(Default, Clone)]
pub struct EventProducers {
    pub order_paid_producer: Vec<EventProducer<OrderPaidEvent>>,
    pub order_annulled_producer: Vec<EventProducer<OrderAnnulledEvent>>,
    pub order_modified_producer: Vec<EventProducer<OrderModifiedEvent>>,
}

/// A container struct for holding event handlers for the different event types. These handlers are typically hooks
/// that allow other modules, plugins and integrations to respond to events on the payment engine.
pub struct EventHandlers {
    pub on_order_paid: Option<EventHandler<OrderPaidEvent>>,
    pub on_order_annulled: Option<EventHandler<OrderAnnulledEvent>>,
    pub on_order_modified: Option<EventHandler<OrderModifiedEvent>>,
}

impl EventHandlers {
    pub fn new(buffer_size: usize, hooks: EventHooks) -> Self {
        let on_order_paid = hooks.on_order_paid.map(|f| EventHandler::new(buffer_size, f));
        let on_order_annulled = hooks.on_order_annulled.map(|f| EventHandler::new(buffer_size, f));
        let on_order_modified = hooks.on_order_modified.map(|f| EventHandler::new(buffer_size, f));
        Self { on_order_paid, on_order_annulled, on_order_modified }
    }

    pub fn producers(&self) -> EventProducers {
        let mut producers = EventProducers::default();
        if let Some(handler) = &self.on_order_paid {
            producers.order_paid_producer.push(handler.subscribe());
        }
        if let Some(handler) = &self.on_order_annulled {
            producers.order_annulled_producer.push(handler.subscribe());
        }
        if let Some(handler) = &self.on_order_modified {
            producers.order_modified_producer.push(handler.subscribe());
        }
        producers
    }

    pub async fn start_handlers(self) {
        if let Some(handler) = self.on_order_paid {
            tokio::spawn(async move {
                handler.start_handler().await;
            });
        }
        if let Some(handler) = self.on_order_annulled {
            tokio::spawn(async move {
                handler.start_handler().await;
            });
        }
        if let Some(handler) = self.on_order_modified {
            tokio::spawn(async move {
                handler.start_handler().await;
            });
        }
    }
}

/// EventHooks is a container struct for holding the callback functions that are called when an event is triggered.
/// The management of co-ordinating and calling the hooks is handled by the [`EventHandlers`] struct.
///
/// The typical usage flow is to create an EventHooks struct, populate it with the hooks you want to use, and then
/// pass it to the [`EventHandlers::new`] method to create the handlers.
///
/// The server will call `start_handlers` on the handlers to start the event callback process using mpsc channels.
#[derive(Default, Clone)]
pub struct EventHooks {
    pub on_order_paid: Option<Handler<OrderPaidEvent>>,
    pub on_order_annulled: Option<Handler<OrderAnnulledEvent>>,
    pub on_order_modified: Option<Handler<OrderModifiedEvent>>,
}

impl EventHooks {
    pub fn on_order_paid<F>(&mut self, f: F) -> &mut Self
    where F: (Fn(OrderPaidEvent) -> Pin<Box<dyn Future<Output = ()> + Send>>) + Send + Sync + 'static {
        self.on_order_paid = Some(Arc::new(f));
        self
    }

    pub fn on_order_annulled<F>(&mut self, f: F) -> &mut Self
    where F: (Fn(OrderAnnulledEvent) -> Pin<Box<dyn Future<Output = ()> + Send>>) + Send + Sync + 'static {
        self.on_order_annulled = Some(Arc::new(f));
        self
    }

    pub fn on_order_modified<F>(&mut self, f: F) -> &mut Self
    where F: (Fn(OrderModifiedEvent) -> Pin<Box<dyn Future<Output = ()> + Send>>) + Send + Sync + 'static {
        self.on_order_modified = Some(Arc::new(f));
        self
    }
}
