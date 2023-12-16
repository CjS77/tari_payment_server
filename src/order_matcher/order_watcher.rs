use crate::db;
use crate::db::{Database, InsertResult};
use crate::spg_server::new_order::FreshOrder;
use crate::spg_server::new_order_service::{NewOrder, NewOrderService, SubscribeToNewOrders};
use actix::SystemService;
use actix::{Actor, AsyncContext, Context, Handler, ResponseFuture};
use log::*;

#[derive(Clone, Default)]
pub struct OrderWatcher {}

impl Actor for OrderWatcher {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let msg = SubscribeToNewOrders {
            client_name: "OrderWatcher".to_string(),
            subscriber: ctx.address().downgrade().recipient(),
        };
        NewOrderService::from_registry().do_send(msg);
        debug!("🛒 OrderWatcher started");
    }

    fn stopping(&mut self, _ctx: &mut Self::Context) -> actix::Running {
        debug!("🛒 OrderWatcher stopping");
        actix::Running::Stop
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        debug!("🛒 OrderWatcher stopped");
    }
}

impl Handler<NewOrder> for OrderWatcher {
    type Result = ResponseFuture<()>;

    fn handle(
        &mut self,
        NewOrder(fresh_order): NewOrder,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        debug!("🛒 OrderWatcher received {:?}", fresh_order.id);
        trace!("🛒 order details {:?}", fresh_order);
        Box::pin(save_order_to_db(fresh_order))
    }
}

async fn save_order_to_db(fresh_order: FreshOrder) {
    let id = fresh_order.id;
    let db = Database::new().await;
    if let Err(e) = db {
        error!("🛒 Error creating connection to database: {e}");
        return;
    }
    let db = db.unwrap();
    let new_order = match db::models::NewOrder::try_from(fresh_order) {
        Ok(new_order) => new_order,
        Err(e) => {
            error!("🛒 Error converting NewOrder to NewOrder: {:?}", e);
            handle_rejected_order(id);
            return;
        }
    };
    match db.insert_order(new_order).await {
        Ok(InsertResult::Inserted) => notify_order_inserted(id),
        Ok(InsertResult::AlreadyExists) => notify_order_already_exists(id),
        Err(e) => error!("🛒 Error inserting order: {:?}", e),
    }
}

fn handle_rejected_order(_id: i64) {
    // TODO
}

fn notify_order_inserted(id: i64) {
    debug!("🛒 Order {id} inserted");
}

fn notify_order_already_exists(id: i64) {
    debug!("🛒 Order {id} already exists")
}
