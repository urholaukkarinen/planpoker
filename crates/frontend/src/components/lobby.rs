use planpoker_common::{RoomMessage, RoomRequest};
use yew::prelude::*;
use yew_router::push_route;

use crate::{agents::RoomAgent, route::Route};

#[derive(Debug, Clone)]
pub enum Msg {
    CreateRoom,
    RoomMessage(RoomMessage),
}

#[derive(PartialEq, Eq)]
enum LobbyState {
    Loading,
    Idle,
    CreatingRoom,
}

pub struct Lobby {
    link: ComponentLink<Self>,
    room_agent: Box<dyn Bridge<RoomAgent>>,
    state: LobbyState,
}

impl Component for Lobby {
    type Message = Msg;
    type Properties = ();

    fn create(_props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let room_agent = RoomAgent::bridge(link.callback(|msg| Msg::RoomMessage(msg)));

        Self {
            link,
            room_agent,
            state: LobbyState::Loading,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::CreateRoom => {
                if self.state == LobbyState::Idle {
                    self.state = LobbyState::CreatingRoom;
                    self.room_agent.send(RoomRequest::CreateRoom);
                }
            }
            Msg::RoomMessage(msg) => match msg {
                RoomMessage::UserInfo(_) => {
                    if self.state == LobbyState::Loading {
                        self.state = LobbyState::Idle;
                    }
                }
                RoomMessage::RoomCreated(id) => push_route(Route::Room { id }),
                msg => {
                    log::info!("Unhandled message: {:?}", msg);
                }
            },
        };

        true
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        if self.state != LobbyState::Loading {
            html! {
                <button
                    disabled=self.state == LobbyState::CreatingRoom
                    onclick=self.link.callback(move |_| Msg::CreateRoom)>
                {
                    if self.state == LobbyState::CreatingRoom {
                        "Creating room"
                    } else {
                        "Create room"
                    }
                }
                </button>
            }
        } else {
            html! {}
        }
    }
}
