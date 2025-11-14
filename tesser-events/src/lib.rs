use serde::{Deserialize, Serialize};
use tesser_core::{Candle, Fill, Order, OrderBook, Signal, Tick};
use tokio::sync::broadcast;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TickEvent {
    pub tick: Tick,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CandleEvent {
    pub candle: Candle,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OrderBookEvent {
    pub order_book: OrderBook,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SignalEvent {
    pub signal: Signal,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FillEvent {
    pub fill: Fill,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OrderUpdateEvent {
    pub order: Order,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Event {
    Tick(TickEvent),
    Candle(CandleEvent),
    OrderBook(OrderBookEvent),
    Signal(SignalEvent),
    Fill(FillEvent),
    OrderUpdate(OrderUpdateEvent),
}

pub struct EventBus {
    sender: broadcast::Sender<Event>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    pub fn subscribe(&self) -> EventStream {
        EventStream {
            receiver: self.sender.subscribe(),
        }
    }

    pub fn publish(&self, event: Event) {
        let _ = self.sender.send(event);
    }
}

pub struct EventStream {
    receiver: broadcast::Receiver<Event>,
}

impl EventStream {
    pub async fn recv(&mut self) -> Result<Event, broadcast::error::RecvError> {
        self.receiver.recv().await
    }
}
