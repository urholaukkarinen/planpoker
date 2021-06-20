use planpoker_common::RoomInfo;
use planpoker_common::RoomMessage;
use planpoker_common::RoomRequest;
use planpoker_common::UserId;
use planpoker_common::Vote;
use yew::prelude::*;
use yew_router::push_route;

use crate::agents::RoomAgent;
use crate::components::card::Card;
use crate::components::loading::Loading;
use crate::route::Route;

#[derive(Properties, Clone, Copy)]
pub struct RoomProps {
    pub id: u32,
}

pub struct Room {
    props: RoomProps,
    link: ComponentLink<Room>,
    room_agent: Box<dyn Bridge<RoomAgent>>,

    room_info: Option<RoomInfo>,
    user_info: Option<UserId>,

    vote: Option<u32>,
}

pub enum Msg {
    Request(RoomRequest),
    Response(RoomMessage),
}

impl Component for Room {
    type Message = Msg;
    type Properties = RoomProps;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Self {
            props,
            link: link.clone(),
            room_agent: RoomAgent::bridge(link.callback(|msg| Msg::Response(msg))),
            room_info: None,
            user_info: None,
            vote: None,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::Request(req) => self.send_request(req),
            Msg::Response(res) => self.handle_response(res),
        };

        true
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        if let Some(room_info) = self.room_info.as_ref() {
            let vote_results = if room_info.revealed {
                let votes = room_info
                    .users
                    .iter()
                    .filter_map(|user| user.vote.value())
                    .filter_map(|card_index| room_info.cards.get(card_index as usize))
                    .filter_map(|card| card.value())
                    .collect::<Vec<_>>();
                log::info!("votes: {:?}", &votes);
                let sum = votes.iter().sum::<u32>() as f32;
                log::info!("sum: {}", sum);

                let avg = sum / votes.len() as f32;

                log::info!("avg: {:?}", avg);

                html! {
                    <p>{ "Avg: " }{avg}</p>
                }
            } else {
                html! {}
            };

            let admin_actions = match self.user_info {
                Some(user_id) if user_id == room_info.admin => {
                    html! {
                        <div class="admin-actions">
                            <button onclick=self.link.callback(move |_| Msg::Request(RoomRequest::Reveal))>{ "Reveal" }</button>
                            <button onclick=self.link.callback(move |_| Msg::Request(RoomRequest::Reset))>{ "Reset" }</button>
                        </div>
                    }
                }
                _ => html! {},
            };

            html! {
                <>
                { self.cards_view(room_info) }
                { self.users_view(room_info) }
                { vote_results }
                { admin_actions }
                </>
            }
        } else {
            html! {
                <Loading/>
            }
        }
    }
}

impl Room {
    fn cards_view(&self, room_info: &RoomInfo) -> Html {
        let card_views = room_info.cards.iter().enumerate().map(|(i, card)| {
            let selected = self.vote == Some(i as u32);
            log::info!("{:?} == Some({}) : {}", self.vote, i, selected);
            html! {
                <Card
                    onclick=self.link.callback(move |_| Msg::Request(RoomRequest::Vote(i as u32)))
                    selected=selected
                    value=card.as_str().to_owned() />
            }
        });

        html! {
                { for card_views }
        }
    }

    fn users_view(&self, room_info: &RoomInfo) -> Html {
        let user_views = room_info.users.iter().enumerate().map(|(i, u)| {
            let a = match u.vote {
                Vote::None => format!("User {} (not voted)", i),
                Vote::Unknown => format!("User {} (voted)", i),
                Vote::Revealed(vote) => format!(
                    "User {} (vote: {})",
                    i,
                    room_info.cards[vote as usize].as_str()
                ),
                _ => "".to_string(),
            };

            html! {
                { a }
            }
        });

        html! {
            <div class="users">
                { for user_views }
            </div>
        }
    }

    fn send_request(&mut self, req: RoomRequest) {
        if let RoomRequest::Vote(vote) = req {
            if self.vote == Some(vote) {
                self.vote = None;
            } else {
                self.vote = Some(vote)
            }
        }

        self.room_agent.send(req);
    }

    fn handle_response(&mut self, msg: RoomMessage) {
        match msg {
            RoomMessage::UserInfo(user_info) => {
                self.user_info = Some(user_info);
                self.join_room();
            }
            RoomMessage::NoSuchRoom(id) => {
                log::info!("No such room: {}", id);
                self.go_to_lobby();
            }
            RoomMessage::Disconnected => {
                log::info!("Disconnected");
                self.go_to_lobby();
            }
            RoomMessage::UserJoined(user_id) => {
                log::info!("User joined the room: {}", user_id);
            }
            RoomMessage::UserLeft(user_id) => {
                log::info!("User left the room: {}", user_id);

                if Some(user_id) == self.user_info {
                    self.go_to_lobby();
                }
            }
            RoomMessage::RoomInfo(room_info) => {
                log::info!("Room info: {:?}", &room_info);
                self.room_info = Some(room_info);
            }
            RoomMessage::Reset => {
                self.vote = None;
            }
            msg => println!("Unhandled msg: {:?}", msg),
        }
    }

    fn join_room(&mut self) {
        log::info!("joining room {}", self.props.id);

        self.send_request(RoomRequest::JoinRoom(self.props.id));
    }

    fn go_to_lobby(&self) {
        push_route(Route::Lobby);
    }
}
