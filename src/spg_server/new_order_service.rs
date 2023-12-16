use crate::spg_server::new_order::FreshOrder;
use actix::{prelude::*, Actor, Context, WeakRecipient};
use log::*;
use std::collections::HashMap;
use std::sync::RwLock;

#[derive(Default)]
pub struct NewOrderService {
    subscribers: RwLock<HashMap<String, WeakRecipient<NewOrder>>>,
}

impl Supervised for NewOrderService {}
impl SystemService for NewOrderService {}

impl Actor for NewOrderService {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        debug!("🗒️  NewOrderService started");
    }

    fn stopping(&mut self, _ctx: &mut Self::Context) -> Running {
        debug!("🗒️  NewOrderService stopping");
        Running::Stop
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        debug!("🗒️  NewOrderService stopped");
    }
}

// -----------------------------------------    Messages      ----------------------------------------------------------

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct SubscribeToNewOrders {
    pub client_name: String,
    pub subscriber: WeakRecipient<NewOrder>,
}

#[derive(Message, Debug, Clone)]
#[rtype(result = "()")]
pub struct NewOrder(pub FreshOrder);

impl NewOrder {
    pub fn into_inner(self) -> FreshOrder {
        self.0
    }
}

impl Handler<SubscribeToNewOrders> for NewOrderService {
    type Result = ();

    fn handle(&mut self, msg: SubscribeToNewOrders, _ctx: &mut Self::Context) -> Self::Result {
        let mut subscribers = match self.subscribers.write() {
            Ok(subscribers) => subscribers,
            Err(e) => {
                error!("🗒️  Error getting lock on subscriber hashmap: {e}");
                return;
            }
        };
        if subscribers
            .insert(msg.client_name.clone(), msg.subscriber)
            .is_some()
        {
            warn!("🗒️  {} has replaced an existing subscriber", msg.client_name);
        } else {
            debug!("🗒️  '{}' has subscribed to new orders", msg.client_name);
            debug!(
                "🗒️  NewOrder service now has {} subscribers",
                subscribers.len()
            );
        }
    }
}

impl Handler<NewOrder> for NewOrderService {
    type Result = ();

    fn handle(&mut self, order: NewOrder, _ctx: &mut Self::Context) -> Self::Result {
        debug!("🗒️  New order received: {}", order.0.id);
        let subscribers = match self.subscribers.read() {
            Ok(subscribers) => subscribers,
            Err(e) => {
                error!("🗒️  Error getting lock on subscriber hashmap: {e}");
                return;
            }
        };
        let num_subscribers = subscribers.len();
        trace!("🗒️  Broadcasting to {num_subscribers} subscribers");
        for (name, subscriber) in &*subscribers {
            if let Some(subscriber) = subscriber.upgrade() {
                debug!("🗒️  Sending new order to {name}");
                match subscriber.try_send(order.clone()) {
                    Err(SendError::Full(_)) => {
                        warn!("🗒️  Subscriber {name} message queue is full");
                    }
                    Err(SendError::Closed(_)) => {
                        warn!("🗒️  Subscriber {name} message queue is closed");
                    }
                    Ok(()) => {
                        trace!("🗒️  New order message was dispatched ok.");
                    }
                }
            } else {
                debug!("🗒️  Subscriber {name} cannot be upgraded. It will be removed from the list of subscribers.");
            }
        }
        drop(subscribers);
        // Clean up dead subscribers
        let mut subscribers = match self.subscribers.write() {
            Ok(subscribers) => subscribers,
            Err(e) => {
                error!("🗒️  Error getting lock on subscriber hashmap: {e}");
                return;
            }
        };
        subscribers.retain(|_, v| v.upgrade().is_some());
        let new_n = subscribers.len();
        if num_subscribers > new_n {
            debug!(
                "🗒️  Removed {} dead subscribers. {new_n} subscribers left.",
                num_subscribers - new_n
            ); // underflow is not possible
        }
    }
}
