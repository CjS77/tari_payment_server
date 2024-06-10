mod address_extractor;
mod memo_signature;
mod wallet_signature;

// All other helpers get thrown in here
mod gumbo;

pub use address_extractor::extract_order_number_from_memo;
pub use gumbo::create_dummy_address_for_cust_id;
pub use memo_signature::{extract_and_verify_memo_signature, MemoSignature, MemoSignatureError};
pub use wallet_signature::{WalletSignature, WalletSignatureError};

#[macro_export]
macro_rules! op {
    (binary $for_struct:ident, $impl_trait:ident, $impl_fn:ident) => {
        impl $impl_trait for $for_struct {
            type Output = Self;

            fn $impl_fn(self, rhs: Self) -> Self::Output {
                Self(self.0.$impl_fn(rhs.0))
            }
        }
    };

    (inplace $for_struct:ident, $impl_trait:ident, $impl_fn:ident) => {
        impl $impl_trait for $for_struct {
            fn $impl_fn(&mut self, rhs: Self) {
                self.0.$impl_fn(rhs.0)
            }
        }
    };

    (unary $for_struct:ident, $impl_trait:ident, $impl_fn:ident) => {
        impl $impl_trait for $for_struct {
            type Output = Self;

            fn $impl_fn(self) -> Self::Output {
                Self(self.0.$impl_fn())
            }
        }
    };
}
