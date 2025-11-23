mod app;
mod events;
mod ui;

use app::CommandOverlay;
pub use app::{LogCategory, LogEntry, MonitorApp, MonitorConfig};

use anyhow::{anyhow, Context, Result};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{stdout, Stdout};
use tesser_rpc::proto::control_service_client::ControlServiceClient;
use tesser_rpc::proto::CancelAllRequest;
use tokio::sync::mpsc;
use tokio::time::{interval, sleep, Duration, MissedTickBehavior};
use tonic::transport::Channel;

use crate::tui::events::MonitorEvent;

pub async fn run_monitor(config: MonitorConfig) -> Result<()> {
    let endpoint = normalize_endpoint(&config.control_addr);
    let client = connect_with_retry(&endpoint).await?;
    let mut terminal = setup_terminal().context("failed to setup terminal")?;
    let result = run_loop(&mut terminal, client, config.clone()).await;
    teardown_terminal(&mut terminal)?;
    result
}

async fn connect_with_retry(target: &str) -> Result<ControlServiceClient<Channel>> {
    const MAX_ATTEMPTS: usize = 30;
    const BACKOFF: Duration = Duration::from_millis(250);
    let mut last_err = None;
    for _ in 0..MAX_ATTEMPTS {
        match ControlServiceClient::connect(target.to_string()).await {
            Ok(client) => return Ok(client),
            Err(err) => {
                last_err = Some(err);
                sleep(BACKOFF).await;
            }
        }
    }
    Err(last_err
        .map(|err| anyhow!("failed to connect to control plane: {err}"))
        .unwrap_or_else(|| anyhow!("failed to connect to control plane")))
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend).context("failed to initialize terminal backend")
}

fn teardown_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

async fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    base_client: ControlServiceClient<Channel>,
    config: MonitorConfig,
) -> Result<()> {
    let mut app = MonitorApp::new(config.clone());
    let poll_client = base_client.clone();
    let stream_client = base_client.clone();
    let mut cancel_client = base_client;

    let (tx, mut rx) = mpsc::channel(512);
    events::spawn_input_listener(tx.clone());
    events::spawn_snapshot_poller(poll_client, tx.clone());
    events::spawn_monitor_stream(stream_client, tx.clone());

    let mut ticker = interval(config.tick_rate);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
    let ctrl_c = tokio::signal::ctrl_c();
    tokio::pin!(ctrl_c);

    loop {
        terminal
            .draw(|frame| ui::draw(frame, &app))
            .context("failed to draw TUI")?;

        tokio::select! {
            _ = ticker.tick() => {}
            Some(event) = rx.recv() => {
                handle_event(event, &mut app, &mut cancel_client).await?;
            }
            _ = &mut ctrl_c => {
                app.request_quit();
            }
        }

        if app.should_quit() {
            break;
        }
    }

    Ok(())
}

async fn handle_event(
    event: MonitorEvent,
    app: &mut MonitorApp,
    cancel_client: &mut ControlServiceClient<Channel>,
) -> Result<()> {
    match event {
        MonitorEvent::Input(key) => handle_key_event(key, app, cancel_client).await?,
        MonitorEvent::Status(status) => app.on_status(status),
        MonitorEvent::Portfolio(snapshot) => app.on_portfolio(snapshot),
        MonitorEvent::Orders(orders) => app.on_orders(orders),
        MonitorEvent::Stream(event) => app.on_stream_event(event),
        MonitorEvent::StreamConnected => app.set_stream_connected(true),
        MonitorEvent::StreamDisconnected => app.set_stream_connected(false),
        MonitorEvent::Error(msg) => app.set_error(msg),
    }
    Ok(())
}

async fn handle_key_event(
    key: KeyEvent,
    app: &mut MonitorApp,
    cancel_client: &mut ControlServiceClient<Channel>,
) -> Result<()> {
    if handle_overlay_key(key, app, cancel_client).await? {
        return Ok(());
    }
    match key.code {
        crossterm::event::KeyCode::Char('q') | crossterm::event::KeyCode::Esc => {
            app.request_quit();
        }
        crossterm::event::KeyCode::Char('c') => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                app.request_quit();
            }
        }
        crossterm::event::KeyCode::Char('m') | crossterm::event::KeyCode::Char('M') => {
            app.toggle_command_palette();
            if matches!(app.overlay(), crate::tui::app::CommandOverlay::Palette) {
                app.record_info("Command palette opened");
            }
        }
        crossterm::event::KeyCode::Char('C') => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                app.request_quit();
            }
        }
        _ => {}
    }
    Ok(())
}

async fn handle_overlay_key(
    key: KeyEvent,
    app: &mut MonitorApp,
    cancel_client: &mut ControlServiceClient<Channel>,
) -> Result<bool> {
    use crossterm::event::KeyCode;
    match app.overlay() {
        CommandOverlay::Hidden => Ok(false),
        CommandOverlay::Palette => {
            match key.code {
                KeyCode::Char('c') | KeyCode::Char('C') => {
                    app.begin_cancel_confirmation();
                    app.record_info("Confirm cancel-all by typing 'cancel all'");
                }
                KeyCode::Esc | KeyCode::Char('m') | KeyCode::Char('M') => {
                    app.close_overlay();
                }
                _ => {}
            }
            Ok(true)
        }
        CommandOverlay::Confirm { .. } => {
            match key.code {
                KeyCode::Esc | KeyCode::Char('m') | KeyCode::Char('M') => {
                    app.close_overlay();
                }
                KeyCode::Backspace => {
                    app.backspace_confirmation();
                }
                KeyCode::Enter => {
                    if app.confirmation_matches() {
                        app.close_overlay();
                        trigger_cancel_all(app, cancel_client).await?;
                    } else {
                        app.set_overlay_error("Type 'cancel all' exactly to proceed.");
                    }
                }
                KeyCode::Char(ch) => {
                    if !key.modifiers.contains(KeyModifiers::CONTROL)
                        && !key.modifiers.contains(KeyModifiers::ALT)
                    {
                        app.append_confirmation_char(ch);
                    }
                }
                _ => {}
            }
            Ok(true)
        }
    }
}

async fn trigger_cancel_all(
    app: &mut MonitorApp,
    cancel_client: &mut ControlServiceClient<Channel>,
) -> Result<()> {
    if app.cancel_in_progress() {
        return Ok(());
    }
    app.set_cancel_in_progress(true);
    app.record_info("Issuing CancelAll request");
    match cancel_client.cancel_all(CancelAllRequest {}).await {
        Ok(response) => app.record_cancel_result(response.into_inner()),
        Err(err) => {
            app.set_cancel_in_progress(false);
            app.set_error(format!("cancel-all failed: {err}"));
        }
    }
    Ok(())
}

fn normalize_endpoint(addr: &str) -> String {
    if addr.starts_with("http://") || addr.starts_with("https://") {
        addr.to_string()
    } else {
        format!("http://{addr}")
    }
}
