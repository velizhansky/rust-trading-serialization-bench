pub mod tick;
pub mod order;
pub mod order_book;

pub use tick::Tick;
pub use order::Order;
pub use order_book::{OrderBook, PriceLevel};

use serde::{Deserialize, Serialize};
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize)]
#[repr(u8)]
pub enum Side {
    Unspecified = 0,
    Buy = 1,
    Sell = 2,
}

impl Default for Side {
    fn default() -> Self {
        Self::Unspecified
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize)]
#[repr(u8)]
pub enum OrderType {
    Unspecified = 0,
    Limit = 1,
    Market = 2,
}

impl Default for OrderType {
    fn default() -> Self {
        Self::Unspecified
    }
}