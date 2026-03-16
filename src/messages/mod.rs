pub mod order;
pub mod order_book;
pub mod tick;

pub use order::Order;
pub use order_book::{OrderBook, PriceLevel};
pub use tick::Tick;

use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::{Deserialize, Serialize};

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[repr(u8)]
#[derive(Default)]
pub enum Side {
    #[default]
    Unspecified = 0,
    Buy = 1,
    Sell = 2,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[repr(u8)]
#[derive(Default)]
pub enum OrderType {
    #[default]
    Unspecified = 0,
    Limit = 1,
    Market = 2,
}
