use std::collections::HashSet;

use planpoker_common::deserialize_json;
use planpoker_common::serialize_json;
use planpoker_common::RoomMessage;
use planpoker_common::RoomRequest;
use yew::worker::HandlerId;
use yew::worker::Job;
use yew::{
    prelude::*,
    worker::{Agent, AgentLink},
};

use super::ws_agent::WsResponse;
use super::WebSocketAgent;

pub enum Msg {
    WsConnected,
    WsDisconnected,
    WsMessage(String),
}

pub struct RoomAgent {
    link: AgentLink<Self>,
    ws_agent: Box<dyn Bridge<WebSocketAgent>>,
    subscribers: HashSet<HandlerId>,
}

impl Agent for RoomAgent {
    type Reach = Job<Self>;
    type Message = Msg;
    type Input = RoomRequest;
    type Output = RoomMessage;

    fn create(link: AgentLink<Self>) -> Self {
        let cb = link.callback(|ws_msg| match ws_msg {
            WsResponse::Ready => Msg::WsConnected,
            WsResponse::Disconnected => Msg::WsDisconnected,
            WsResponse::Message(msg) => Msg::WsMessage(msg),
        });
        let ws_agent = WebSocketAgent::bridge(cb);

        Self {
            link,
            ws_agent,
            subscribers: HashSet::new(),
        }
    }

    fn update(&mut self, msg: Self::Message) {
        match msg {
            Msg::WsConnected => {
                self.send(RoomRequest::UserInfo);
            }
            Msg::WsDisconnected => {
                self.respond_to_all(RoomMessage::Disconnected);
            }
            Msg::WsMessage(msg) => match deserialize_json(&msg) {
                Ok(msg) => self.respond_to_all(msg),
                Err(err) => log::warn!("{}", err),
            },
        }
    }

    fn handle_input(&mut self, msg: Self::Input, id: worker::HandlerId) {
        self.send(msg);
    }

    fn connected(&mut self, id: worker::HandlerId) {
        self.subscribers.insert(id);
    }

    fn disconnected(&mut self, id: worker::HandlerId) {
        self.subscribers.remove(&id);
    }

    fn destroy(&mut self) {
        log::info!("room agent destroyed")
    }
}

impl RoomAgent {
    fn send(&mut self, msg: RoomRequest) {
        self.ws_agent.send(serialize_json(&msg).unwrap());
    }

    fn respond(&self, sub: HandlerId, response: RoomMessage) {
        self.link.respond(sub, response);
    }

    fn respond_to_all(&self, response: RoomMessage) {
        for sub in self.subscribers.iter() {
            self.respond(*sub, response.clone())
        }
    }
}
