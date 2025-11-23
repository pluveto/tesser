use std::time::Duration;

use crossterm::event::{Event as CrosstermEvent, EventStream, KeyEvent, KeyEventKind};
use futures::StreamExt;
use tesser_rpc::proto::control_service_client::ControlServiceClient;
use tesser_rpc::proto::{
    Event, GetOpenOrdersRequest, GetPortfolioRequest, GetStatusRequest, GetStatusResponse,
    MonitorRequest, OrderSnapshot, PortfolioSnapshot,
};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::{interval, sleep, MissedTickBehavior};
use tonic::transport::Channel;

#[derive(Debug)]
pub enum MonitorEvent {
    Input(KeyEvent),
    Status(GetStatusResponse),
    Portfolio(PortfolioSnapshot),
    Orders(Vec<OrderSnapshot>),
    Stream(Event),
    StreamConnected,
    StreamDisconnected,
    Error(String),
}

pub fn spawn_input_listener(tx: mpsc::Sender<MonitorEvent>) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut reader = EventStream::new();
        while let Some(event) = reader.next().await {
            match event {
                Ok(CrosstermEvent::Key(key)) if key.kind == KeyEventKind::Press => {
                    if tx.send(MonitorEvent::Input(key)).await.is_err() {
                        break;
                    }
                }
                Ok(_) => {}
                Err(err) => {
                    if tx
                        .send(MonitorEvent::Error(format!("input error: {err}")))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
            }
        }
    })
}

pub fn spawn_snapshot_poller(
    client: ControlServiceClient<Channel>,
    tx: mpsc::Sender<MonitorEvent>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut client = client;
        let mut ticker = interval(Duration::from_secs(1));
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            ticker.tick().await;
            if tx.is_closed() {
                break;
            }
            match client.get_status(GetStatusRequest {}).await {
                Ok(resp) => {
                    if tx
                        .send(MonitorEvent::Status(resp.into_inner()))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
                Err(err) => {
                    if tx
                        .send(MonitorEvent::Error(format!("status error: {err}")))
                        .await
                        .is_err()
                    {
                        break;
                    }
                    continue;
                }
            }
            match client.get_portfolio(GetPortfolioRequest {}).await {
                Ok(resp) => {
                    if let Some(portfolio) = resp.into_inner().portfolio {
                        if tx.send(MonitorEvent::Portfolio(portfolio)).await.is_err() {
                            break;
                        }
                    }
                }
                Err(err) => {
                    if tx
                        .send(MonitorEvent::Error(format!("portfolio error: {err}")))
                        .await
                        .is_err()
                    {
                        break;
                    }
                    continue;
                }
            }
            match client.get_open_orders(GetOpenOrdersRequest {}).await {
                Ok(resp) => {
                    if tx
                        .send(MonitorEvent::Orders(resp.into_inner().orders))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
                Err(err) => {
                    if tx
                        .send(MonitorEvent::Error(format!("orders error: {err}")))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
            }
        }
    })
}

pub fn spawn_monitor_stream(
    client: ControlServiceClient<Channel>,
    tx: mpsc::Sender<MonitorEvent>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut client = client;
        loop {
            if tx.is_closed() {
                break;
            }
            match client.monitor(MonitorRequest {}).await {
                Ok(resp) => {
                    if tx.send(MonitorEvent::StreamConnected).await.is_err() {
                        break;
                    }
                    let mut stream = resp.into_inner();
                    loop {
                        match stream.message().await {
                            Ok(Some(event)) => {
                                if tx.send(MonitorEvent::Stream(event)).await.is_err() {
                                    return;
                                }
                            }
                            Ok(None) => break,
                            Err(status) => {
                                let _ = tx
                                    .send(MonitorEvent::Error(format!(
                                        "monitor stream error: {status}"
                                    )))
                                    .await;
                                break;
                            }
                        }
                    }
                    let _ = tx.send(MonitorEvent::StreamDisconnected).await;
                }
                Err(err) => {
                    if tx
                        .send(MonitorEvent::Error(format!("monitor connect error: {err}")))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
            }
            sleep(Duration::from_secs(1)).await;
        }
    })
}
