use crate::db::{Database, InsertResult};
use crate::order_matcher::messages::TransferReceived;
use actix::{Actor, Context, Handler, ResponseFuture, Running};
use log::*;

#[derive(Clone, Default)]
pub struct PaymentWatcher;

impl Actor for PaymentWatcher {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        info!("🤑  PaymentWatcher started");
    }

    fn stopping(&mut self, _ctx: &mut Self::Context) -> Running {
        info!("🤑  PaymentWatcher stopping");
        Running::Stop
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        info!("🤑  PaymentWatcher stopped");
    }
}

impl Handler<TransferReceived> for PaymentWatcher {
    type Result = ResponseFuture<()>;

    fn handle(&mut self, msg: TransferReceived, _ctx: &mut Self::Context) -> Self::Result {
        debug!(
            "🤑  PaymentWatcher has observed a transfer for {}",
            msg.amount
        );
        Box::pin(save_transfer_to_db(msg))
    }
}

async fn save_transfer_to_db(transfer: TransferReceived) {
    let db = Database::new().await;
    if let Err(e) = db {
        error!("🤑  Error creating connection to database: {e}");
        return;
    }
    let db = db.unwrap();
    let msg = format!(
        "🤑  Transfer from {} for {} stored.",
        transfer.sender, transfer.amount
    );
    match db.insert_transfer(transfer).await {
        Ok(InsertResult::AlreadyExists) => debug!("🤑  Transfer already exists in database"),
        Ok(InsertResult::Inserted) => info!("{msg}"),
        Err(e) => error!("🤑  Error saving transfer to database: {e}"),
    }
}
