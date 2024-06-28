//! Simple stateless pub-sub event handler
//!
//! This module provides a simple hook system that allow components of the system subscribe to payment server events
//! and react to them. The event handler is stateless, i.e. the handlers have no access to the internal state of the
//! system. All that is received is the event itself.
//!
//! However, the handlers can be async.
use std::{
    future::Future,
    pin::Pin,
    sync::{atomic::AtomicI64, Arc},
};

use log::*;
use tokio::sync::mpsc;

pub type Handler<E> = Arc<dyn Fn(E) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

pub struct EventHandler<E: Send + Sync + 'static> {
    listener: mpsc::Receiver<E>,
    sender: mpsc::Sender<E>,
    handler: Handler<E>,
}

impl<E: Send + Sync + 'static> EventHandler<E> {
    pub fn new(buffer_size: usize, handler: Handler<E>) -> Self {
        let (sender, receiver) = mpsc::channel(buffer_size);
        Self { listener: receiver, sender, handler }
    }

    pub fn subscribe(&self) -> EventProducer<E> {
        EventProducer::new(self.sender.clone())
    }

    pub async fn start_handler(mut self) {
        debug!("📬️ Starting event handler");
        // drop the internal sender so that when the last subscriber is dropped, we can automatically shut down the
        // handler
        drop(self.sender);
        let jobs = Arc::new(AtomicI64::new(0));
        while let Some(ev) = self.listener.recv().await {
            trace!("📬️ Handling event");
            let handler = Arc::clone(&self.handler);
            jobs.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let job = jobs.clone();
            tokio::spawn(async move {
                (handler)(ev).await;
                job.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
                trace!("📬️ Event handled");
            });
        }
        match tokio::spawn(async move {
            while jobs.load(std::sync::atomic::Ordering::SeqCst) > 0 {
                debug!("📬️ Waiting for jobs to complete");
                tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
            }
        })
        .await
        {
            Ok(_) => {
                debug!("📬️ Event handler shutting down gracefully");
            },
            Err(e) => {
                warn!(
                    "📬️ Event handler shutdown process failed: {e}. It's probably not a chain smash (hehe), but \
                     logging this just in case."
                );
            },
        }
        debug!("📬️ Event handler has shut down");
    }
}

#[derive(Clone)]
pub struct EventProducer<E: Send + Sync> {
    sender: mpsc::Sender<E>,
}

impl<E: Send + Sync> EventProducer<E> {
    pub fn new(sender: mpsc::Sender<E>) -> Self {
        Self { sender }
    }

    pub async fn publish_event(&self, event: E) {
        if let Err(e) = self.sender.send(event).await {
            error!("📬️ Failed to send event: {e}");
        }
    }
}

#[cfg(test)]
mod test {
    use std::sync::atomic::AtomicU64;

    use super::*;

    #[tokio::test]
    async fn test_event_handler() {
        let _ = env_logger::try_init();
        let count = Arc::new(AtomicU64::new(0));
        let c2 = count.clone();
        let handler = Arc::new(move |v| {
            let count = count.clone();
            Box::pin(async move {
                debug!("Handler received {v}");
                let _ = count.fetch_add(v, std::sync::atomic::Ordering::SeqCst);
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }) as Pin<Box<dyn Future<Output = ()> + Send>>
        });
        let event_handler = EventHandler::new(1, handler);
        let producer_1 = event_handler.subscribe();
        let producer_2 = event_handler.subscribe();
        tokio::spawn(async move {
            for i in 0..5 {
                let v = i * 2 + 1;
                producer_1.publish_event(v).await;
                debug!("P1 publishing {v}");
            }
        });
        tokio::spawn(async move {
            for i in 0..5 {
                let v = i * 2;
                producer_2.publish_event(v).await;
                debug!("P2 publishing {v}");
            }
        });

        event_handler.start_handler().await;
        debug!("Handler done");
        assert_eq!(c2.load(std::sync::atomic::Ordering::SeqCst), 45);
    }
}
