use std::{future::Future, pin::Pin, sync::Arc};

use crate::events::{EventHandler, EventProducer, Handler, OrderPaidEvent};

#[derive(Default, Clone)]
pub struct EventProducers {
    pub order_paid_producer: Vec<EventProducer<OrderPaidEvent>>,
}

pub struct EventHandlers {
    pub on_order_paid: Option<EventHandler<OrderPaidEvent>>,
}

impl EventHandlers {
    pub fn new(buffer_size: usize, hooks: EventHooks) -> Self {
        let on_order_paid = hooks.on_order_paid.map(|f| EventHandler::new(buffer_size, f));
        Self { on_order_paid }
    }

    pub fn producers(&self) -> EventProducers {
        let mut result = EventProducers::default();
        if let Some(handler) = &self.on_order_paid {
            result.order_paid_producer.push(handler.subscribe());
        }
        result
    }

    pub async fn start_handlers(self) {
        if let Some(handler) = self.on_order_paid {
            tokio::spawn(async move {
                handler.start_handler().await;
            });
        }
    }
}

#[derive(Default, Clone)]
pub struct EventHooks {
    pub on_order_paid: Option<Handler<OrderPaidEvent>>,
}

impl EventHooks {
    pub fn on_order_paid<F>(&mut self, f: F) -> &mut Self
    // Arc<dyn Fn(E) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>
    where F: (Fn(OrderPaidEvent) -> Pin<Box<dyn Future<Output = ()> + Send>>) + Send + Sync + 'static {
        self.on_order_paid = Some(Arc::new(f));
        self
    }
}
