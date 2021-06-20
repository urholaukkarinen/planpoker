use std::collections::HashSet;

use anyhow::Error;
use serde::{Deserialize, Serialize};
use yew::worker::Context;
use yew::worker::HandlerId;
use yew::{
    prelude::*,
    worker::{Agent, AgentLink},
};
use yew_services::websocket::{WebSocketService, WebSocketStatus, WebSocketTask};

pub enum Msg {
    Connect,
    MessageReceived(Result<String, Error>),
    Connected,
    Disconnected,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum WsResponse {
    Ready,
    Disconnected,
    Message(String),
}

pub struct WebSocketAgent {
    link: AgentLink<Self>,
    ws: Option<WebSocketTask>,
    subscribers: HashSet<HandlerId>,
    connected: bool,
}

impl Agent for WebSocketAgent {
    type Reach = Context<Self>;
    type Message = Msg;
    type Input = String;
    type Output = WsResponse;

    fn create(link: AgentLink<Self>) -> Self {
        link.send_message(Msg::Connect);

        Self {
            link,
            ws: None,
            subscribers: HashSet::new(),
            connected: false,
        }
    }

    fn update(&mut self, msg: Self::Message) {
        match msg {
            Msg::Connect => {
                self.connect();
            }
            Msg::Connected => {
                log::info!("ws connected");

                self.connected = true;
                self.respond_to_all(WsResponse::Ready);
            }
            Msg::Disconnected => {
                log::info!("ws disconnected");

                self.connected = false;
                self.ws = None;

                self.respond_to_all(WsResponse::Disconnected);
            }
            Msg::MessageReceived(res) => match res {
                Ok(msg) => {
                    log::info!("ws msg: {:?}", msg);
                    self.respond_to_all(WsResponse::Message(msg.clone()));
                }
                Err(err) => {
                    log::error!("ws msg error: {:?}", err);
                }
            },
        }
    }

    fn handle_input(&mut self, msg: Self::Input, id: worker::HandlerId) {
        if let Some(ws) = self.ws.as_mut() {
            ws.send(Ok(msg));
        }
    }

    fn connected(&mut self, id: worker::HandlerId) {
        self.subscribers.insert(id);

        if self.connected {
            self.respond(id, WsResponse::Ready);
        }
    }

    fn disconnected(&mut self, id: worker::HandlerId) {
        self.subscribers.remove(&id);
    }

    fn destroy(&mut self) {
        log::info!("ws agent destroyed")
    }
}

impl WebSocketAgent {
    fn respond(&self, sub: HandlerId, response: WsResponse) {
        self.link.respond(sub, response);
    }

    fn respond_to_all(&self, response: WsResponse) {
        for sub in self.subscribers.iter() {
            self.respond(*sub, response.clone())
        }
    }

    fn connect(&mut self) {
        let ws_msg_callback = self.link.callback(|data| Msg::MessageReceived(data));

        let ws_notification_callback = self.link.callback(|status| match status {
            WebSocketStatus::Opened => Msg::Connected,
            WebSocketStatus::Closed | WebSocketStatus::Error => Msg::Disconnected,
        });

        let ws = WebSocketService::connect_text(
            "ws://localhost:8082/ws/",
            ws_msg_callback,
            ws_notification_callback,
        )
        .unwrap();

        self.ws = Some(ws);
    }
}
