//! WebSocket client for communicating with a local Runner process.
//!
//! Uses `ewebsock` which works on both native and WASM targets.

use ewebsock::{Options, WsEvent, WsMessage, WsReceiver, WsSender};
use serde::{Deserialize, Serialize};

const RUNNER_URL: &str = "ws://127.0.0.1:9734";

/// Commands sent to the Runner.
#[derive(Serialize)]
#[serde(tag = "type")]
pub enum RunnerCommand {
    #[serde(rename = "run_test")]
    RunTest { feature: String, scenario: String },
    #[serde(rename = "stop_test")]
    StopTest,
    #[serde(rename = "ping")]
    Ping,
}

/// Events received from the Runner.
#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum RunnerEvent {
    #[serde(rename = "test_started")]
    TestStarted { scenario: String },
    #[serde(rename = "test_step")]
    TestStep { step: String, status: String },
    #[serde(rename = "test_output")]
    TestOutput { line: String },
    #[serde(rename = "test_finished")]
    TestFinished {
        scenario: String,
        status: String,
        duration_ms: u64,
    },
    #[serde(rename = "pong")]
    Pong,
}

/// Connection state to a Runner.
pub struct RunnerConnection {
    sender: Option<WsSender>,
    receiver: Option<WsReceiver>,
    pub connected: bool,
    pending_events: Vec<RunnerEvent>,
}

impl RunnerConnection {
    /// Attempt to connect to the Runner at the default URL.
    pub fn connect() -> Self {
        let options = Options::default();
        match ewebsock::connect(RUNNER_URL, options) {
            Ok((sender, receiver)) => Self {
                sender: Some(sender),
                receiver: Some(receiver),
                connected: true,
                pending_events: Vec::new(),
            },
            Err(e) => {
                log::warn!("Failed to connect to Runner: {}", e);
                Self {
                    sender: None,
                    receiver: None,
                    connected: false,
                    pending_events: Vec::new(),
                }
            }
        }
    }

    /// Poll the WebSocket for new events. Should be called each frame.
    pub fn poll(&mut self) -> &[RunnerEvent] {
        self.pending_events.clear();
        if let Some(receiver) = &mut self.receiver {
            while let Some(event) = receiver.try_recv() {
                match event {
                    WsEvent::Opened => self.connected = true,
                    WsEvent::Closed => {
                        self.connected = false;
                    }
                    WsEvent::Error(e) => log::warn!("Runner WebSocket: {}", e),
                    WsEvent::Message(WsMessage::Text(text)) => {
                        if let Ok(evt) = serde_json::from_str::<RunnerEvent>(&text) {
                            self.pending_events.push(evt);
                        }
                    }
                    _ => {}
                }
            }
        }
        &self.pending_events
    }

    /// Send a command to the Runner.
    pub fn send(&mut self, cmd: &RunnerCommand) {
        if let Some(sender) = &mut self.sender {
            if self.connected {
                let json = serde_json::to_string(cmd).unwrap();
                sender.send(WsMessage::Text(json));
            }
        }
    }
}
