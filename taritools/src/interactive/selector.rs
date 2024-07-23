use std::time::{Duration, Instant};

use anyhow::Result;

use crate::tari_payment_server::client::PaymentServerClient;

macro_rules! selector_for {
    ($name:ident, $update_fn:ident) => {
        paste::paste! { pub struct [<$name:camel Selector>] {
                items: Vec<String>,
                last_update: Option<Instant>,
                update_interval: Duration,
            }
        }

        impl Default for paste::paste! {[<$name:camel Selector>]} {
            fn default() -> Self {
                Self::new(Duration::from_secs(60 * 60))
            }
        }

        impl paste::paste! {[<$name:camel Selector>]} {
            pub fn new(update_interval: Duration) -> Self {
                Self { items: Vec::new(), last_update: None, update_interval }
            }

            pub fn items(&self) -> &[String] {
                &self.items
            }

            pub async fn update(&mut self, client: &PaymentServerClient) -> Result<()> {
                if let Some(last_update) = self.last_update {
                    if last_update.elapsed() < self.update_interval {
                        return Ok(());
                    }
                }
                let items = client.$update_fn().await?;
                self.items = items;
                self.last_update = Some(Instant::now());
                Ok(())
            }
        }
    };
}

selector_for!(Customer, customer_ids);
selector_for!(Address, addresses);
