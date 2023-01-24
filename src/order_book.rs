use crate::datastructs::orders::Side;
use crate::datastructs::websocket::{
    Level2Change,
    Level2Snapshot,
    Quote,
};
use log::debug;
use std::cmp::Ordering;
use std::mem;
use std::num::FpCategory;
use std::ops::{
    DerefMut,
    Index,
    IndexMut,
};
use std::sync::Arc;
use tokio::sync::{
    Mutex,
    MutexGuard,
};
use tokio::time::{
    Duration,
    Instant,
};

#[derive(Debug, Clone)]
pub struct RealFloat(f64);

#[derive(Debug)]
pub struct InvalidFloat {}

impl Into<f64> for RealFloat {
    fn into(self) -> f64 {
        self.0
    }
}

impl TryFrom<f64> for RealFloat {
    type Error = InvalidFloat;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        match value.classify() {
            FpCategory::Nan => {
                return Err(InvalidFloat {});
            }
            FpCategory::Infinite => {
                return Err(InvalidFloat {});
            }
            _ => {}
        }

        Ok(RealFloat(value))
    }
}

impl PartialEq<Self> for RealFloat {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl PartialEq<f64> for RealFloat {
    fn eq(&self, other: &f64) -> bool {
        self.0 == other.clone()
    }
}

impl Eq for RealFloat {}

impl PartialOrd for RealFloat {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self == other {
            return Some(Ordering::Equal);
        }

        return if self.0 < other.0 {
            Some(Ordering::Less)
        } else {
            Some(Ordering::Greater)
        };
    }
}

impl Ord for RealFloat {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

#[derive(Debug, Clone)]
pub struct OrderBookEntry {
    pub(crate) price: RealFloat,
    pub(crate) size: RealFloat,
}

impl OrderBookEntry {
    pub fn price(&self) -> f64 {
        self.price.0.clone()
    }

    pub fn size(&self) -> f64 {
        self.size.0.clone()
    }

    pub fn price_ref(&self) -> &f64 {
        &self.price.0
    }

    pub fn size_ref(&self) -> &f64 {
        &self.size.0
    }
}

impl From<Level2Change> for OrderBookEntry {
    fn from(value: Level2Change) -> Self {
        Self {
            price: value.price.try_into().unwrap(),
            size: value.size.try_into().unwrap(),
        }
    }
}

impl PartialEq for OrderBookEntry {
    fn eq(&self, other: &Self) -> bool {
        self.price == other.price
    }
}

impl PartialOrd for OrderBookEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.price.partial_cmp(&other.price)
    }
}

impl Eq for OrderBookEntry {}

impl Ord for OrderBookEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.price.cmp(&other.price)
    }
}

impl TryFrom<Quote> for OrderBookEntry {
    type Error = InvalidFloat;

    fn try_from(value: Quote) -> Result<Self, Self::Error> {
        Ok(Self {
            price: value.price.try_into()?,
            size: value.size.try_into()?,
        })
    }
}

impl TryFrom<(f64, f64)> for OrderBookEntry {
    type Error = InvalidFloat;

    fn try_from(value: (f64, f64)) -> Result<Self, Self::Error> {
        Ok(Self {
            price: value.0.try_into()?,
            size: value.1.try_into()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct OrderBook {
    // Bids ordered least to greatest
    pub(crate) bids: Arc<Mutex<Vec<OrderBookEntry>>>,
    // Asks ordered greatest to least
    pub(crate) asks: Arc<Mutex<Vec<OrderBookEntry>>>,

    pub(crate) updated: Arc<Mutex<Instant>>,
}

impl From<Level2Snapshot> for OrderBook {
    fn from(snap: Level2Snapshot) -> Self {
        let mut bids = snap
            .bids
            .into_iter()
            .map(|q| q.try_into())
            .flatten()
            .collect::<Vec<OrderBookEntry>>();
        let mut asks = snap
            .asks
            .into_iter()
            .map(|q| q.try_into())
            .flatten()
            .collect::<Vec<OrderBookEntry>>();

        bids.sort();
        asks.sort();

        asks.reverse();

        Self {
            bids: Arc::new(Mutex::new(bids)),
            asks: Arc::new(Mutex::new(asks)),
            updated: Arc::new(Mutex::new(Instant::now())),
        }
    }
}

impl OrderBook {
    pub async fn bid_ask_locks(
        &self,
    ) -> (
        MutexGuard<'_, Vec<OrderBookEntry>>,
        MutexGuard<'_, Vec<OrderBookEntry>>,
    ) {
        let ask_lock = self.asks.lock().await;
        let bid_lock = self.bids.lock().await;

        (bid_lock, ask_lock)
    }

    pub async fn mid_price(&self) -> f64 {
        let (bid_lock, ask_lock) = self.bid_ask_locks().await;
        self.mid_price_with_locks(&ask_lock, &bid_lock).await
    }

    pub async fn mid_price_with_locks(
        &self,
        bid_lock: &MutexGuard<'_, Vec<OrderBookEntry>>,
        ask_lock: &MutexGuard<'_, Vec<OrderBookEntry>>,
    ) -> f64 {
        let best_ask = ask_lock
            .last()
            .unwrap_or(&(0f64, 0f64).try_into().unwrap())
            .price
            .0;

        let best_bid = bid_lock
            .last()
            .unwrap_or(&(0f64, 0f64).clone().try_into().unwrap())
            .price
            .0;

        (best_ask + best_bid) / 2.0
    }

    pub async fn apply_change_l2_changes(&mut self, changes: Vec<Level2Change>) {
        let ask_lock = self.asks.clone();
        let bid_lock = self.bids.clone();

        let b = ask_lock.lock().await;
        let a = bid_lock.lock().await;

        let mut locks = (a, b);

        for change in changes {
            match change.clone().side.as_str() {
                "buy" => {
                    self.apply_change_with_lock(Side::BUY, change.into(), &mut locks.0)
                        .await;
                }
                "sell" => {
                    self.apply_change_with_lock(Side::SELL, change.into(), &mut locks.1)
                        .await;
                }
                _ => {
                    return;
                }
            }
        }
    }

    pub async fn apply_change_l2_change(&mut self, change: Level2Change) {
        let side = match change.side.as_str() {
            "buy" => Side::BUY,
            "sell" => Side::SELL,
            _ => {
                return;
            }
        };

        self.apply_change(side, change.into()).await
    }

    pub async fn apply_change_with_lock(
        &mut self,
        side: Side,
        entry: OrderBookEntry,
        lock: &mut MutexGuard<'_, Vec<OrderBookEntry>>,
    ) {
        let idx = match side {
            Side::BUY => Self::find_bid_index(&lock, &entry),
            Side::SELL => Self::find_ask_index(&lock, &entry),
        };



        if idx == lock.len() {
            lock.insert(idx, entry);
            mem::swap(self.updated.lock().await.deref_mut(), &mut Instant::now());
            return;
        }

        if entry.size == 0.0{
            lock.remove(idx);
            return;
        }

        if lock.index(idx).price == entry.price {
            if entry.size == 0f64 {
                lock.remove(idx);
            } else {
                lock.index_mut(idx).size = entry.size;
            }

            mem::swap(self.updated.lock().await.deref_mut(), &mut Instant::now());
            return;
        }

        if lock.index(idx).price != entry.price && entry.size != 0f64 {
            lock.insert(idx, entry);
            mem::swap(self.updated.lock().await.deref_mut(), &mut Instant::now());
            return;
        }
    }

    pub async fn last_updated(&mut self) -> Duration {
        self.updated.lock().await.elapsed()
    }

    pub async fn apply_change(&mut self, side: Side, entry: OrderBookEntry) {
        let vec = match side {
            Side::BUY => self.bids.clone(),
            Side::SELL => self.asks.clone(),
        };

        let mut lock = vec.lock().await;

        self.apply_change_with_lock(side, entry, &mut lock).await;
    }

    pub(crate) fn find_bid_index(
        vec: &MutexGuard<Vec<OrderBookEntry>>,
        entry: &OrderBookEntry,
    ) -> usize {
        for (index, bid) in vec.iter().enumerate().rev() {
            if bid.price == entry.price {
                return index;
            } else if bid.price < entry.price {
                return index + 1;
            }
        }

        return 0;
    }

    pub(crate) fn find_ask_index(
        vec: &MutexGuard<Vec<OrderBookEntry>>,
        entry: &OrderBookEntry,
    ) -> usize {
        for (index, ask) in vec.iter().enumerate().rev() {
            if ask.price == entry.price {
                return index;
            } else if ask.price > entry.price {
                return index + 1;
            }
        }

        return 0;
    }
}
