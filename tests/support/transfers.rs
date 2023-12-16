use rand::Rng;

use shopify_payment_gateway::db::models::{MicroTari, PublicKey};
use shopify_payment_gateway::order_matcher::messages::{TransferBuilder, TransferReceived};

const USERS: [&str; 5] = [
    "abe0000100000000111111110000000011111111000000001111111100000000",
    "bea4000200000000222222220000000022222222000000002222222200000000",
    "ca1e800300000000333333330000000033333333000000003333333300000000",
    "de88113000000000444444440000000044444444000000004444444400000000",
    "e1f0000400000000555555550000000055555555000000005555555500000000",
];

const STORE: &str = "aaaaaaaabbbbbbbbaaaaaaaabbbbbbbbaaaaaaaabbbbbbbbaaaaaaaabbbbbbbb";


pub fn random_transfer() -> TransferReceived {
    let mut rng = rand::thread_rng();
    let sender = rng.gen_range(0..5);
    TransferBuilder::default()
        .block_height(rng.gen_range(1..1000))
        .sender(PublicKey(USERS[sender].into()))
        .receiver(PublicKey(STORE.into()))
        .amount(MicroTari::from(rng.gen_range(1000..1_000_000) * 1000))
        .memo("random transfer")
        .build()
}
