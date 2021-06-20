use std::{
    ops::Deref,
    str::FromStr,
    sync::Mutex,
    time::{Duration, Instant},
};

use rand::Rng;

use actix::prelude::*;
use actix_web::{
    cookie::{Cookie, SameSite},
    middleware,
    web::{self, Data},
    App, Error, HttpMessage, HttpRequest, HttpResponse, HttpServer,
};
use actix_web_actors::ws::{self, WebsocketContext};
use planpoker_common::{
    deserialize_binary, deserialize_json, serialize_json, Card, RoomId, RoomInfo, RoomMessage,
    RoomRequest, SessionId, UserId, Vote,
};
use uuid::Uuid;

/// How often heartbeat pings are sent
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
/// How long before lack of client response causes a timeout
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

const MAX_ROOM_ID: RoomId = 999999;

#[derive(Clone)]
pub struct User {
    pub id: UserId,
    pub session_id: SessionId,
}
struct RoomNotification(RoomMessage);

impl Message for RoomNotification {
    type Result = ();
}

impl Deref for RoomNotification {
    type Target = RoomMessage;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

struct AppState {
    clients: Vec<(UserId, Addr<WebSocket>)>,
    rooms: Vec<RoomInfo>,
    users: Vec<User>,
}

impl AppState {
    fn add_client(&mut self, user_id: UserId, client: Addr<WebSocket>) {
        self.clients.push((user_id, client));
    }

    fn remove_client(&mut self, client: Addr<WebSocket>) {
        let client = self
            .clients
            .iter()
            .enumerate()
            .find(|(_, c)| c.1.eq(&client))
            .map(|(i, c)| (i, c.clone()));

        if let Some((i, client)) = client {
            // There may be multiple clients with same user id (multiple tabs open in the browser)
            // Do not remove the user from rooms in that case.
            if self.clients.iter().filter(|c| c.0 == client.0).count() == 1 {
                let room_ids = self.rooms.iter().map(|r| r.id).collect::<Vec<_>>();

                for room_id in room_ids {
                    self.remove_user_from_room(client.0, room_id);
                }
            }

            self.clients.remove(i);
        }
    }

    fn user_with_session_id(&mut self, id: SessionId) -> Option<User> {
        self.users.iter().find(|u| u.session_id == id).cloned()
    }

    fn create_user(&mut self) -> User {
        let user_id = Uuid::new_v4().as_u128();
        let session_id = Uuid::new_v4().as_u128();

        let user = User {
            id: user_id,
            session_id,
        };

        self.users.push(user.clone());
        user
    }

    fn create_room(&mut self, owner_id: UserId) -> RoomId {
        let id = loop {
            let id = rand::thread_rng().gen_range(0, MAX_ROOM_ID);

            if !self.has_room_with_id(id) {
                break id;
            }
        };

        let mut cards = (1..10)
            .map(|i| Card::valued(i.to_string(), i))
            .collect::<Vec<_>>();
        cards.push(Card::valueless("?"));

        let mut room = RoomInfo::new(id, owner_id);
        room.cards = cards;

        self.rooms.push(room);

        id
    }

    fn has_room_with_id(&self, id: RoomId) -> bool {
        self.rooms.iter().any(|r| r.id == id)
    }

    fn add_user_to_room(&mut self, user_id: UserId, room_id: RoomId) {
        if let Some(room) = self.room_mut(room_id) {
            room.add_user(user_id);

            let room = room.clone();
            self.send_to_room_users(&room, RoomMessage::UserJoined(user_id));
            self.send_room_info(&room);
        }
    }

    fn remove_user_from_room(&mut self, user_id: UserId, room_id: RoomId) {
        if let Some(room) = self.room_mut(room_id) {
            room.users.retain(|u| u.user_id != user_id);

            let room = room.clone();
            self.send_to_room_users(&room, RoomMessage::UserLeft(user_id));
            self.send_room_info(&room);
        }
    }

    fn vote(&mut self, user_id: UserId, room_id: RoomId, vote: u32) {
        if let Some(room) = self.room_mut(room_id) {
            if room.revealed {
                // Room must be reset before voting is allowed.
                return;
            }

            if let Some(user) = room.users.iter_mut().find(|u| u.user_id == user_id) {
                let new_vote = match user.vote {
                    Vote::Hidden(v) if v == vote => Vote::None,
                    _ => Vote::Hidden(vote),
                };

                user.vote = new_vote;

                let room = room.clone();
                self.send_to_room_users(&room, RoomMessage::UserVoted(user_id));
                self.send_room_info(&room);
            }
        }
    }

    fn reveal_votes(&mut self, user_id: UserId, room_id: RoomId) {
        if let Some(room) = self.room_mut(room_id) {
            if room.admin == user_id {
                for user in room.users.iter_mut() {
                    if let Vote::Hidden(vote) = user.vote {
                        user.vote = Vote::Revealed(vote);
                    }
                }
            }

            room.revealed = true;

            let room = room.clone();
            self.send_to_room_users(&room, RoomMessage::CardsRevealed);
            self.send_room_info(&room);
        }
    }

    fn reset_votes(&mut self, user_id: UserId, room_id: RoomId) {
        if let Some(room) = self.room_mut(room_id) {
            if room.admin == user_id {
                for user in room.users.iter_mut() {
                    user.vote = Vote::None;
                }
            }

            room.revealed = false;

            let room = room.clone();
            self.send_to_room_users(&room, RoomMessage::Reset);
            self.send_room_info(&room);
        }
    }

    fn room(&mut self, room_id: RoomId) -> Option<&RoomInfo> {
        self.rooms.iter().find(|r| r.id == room_id)
    }

    fn room_mut(&mut self, room_id: RoomId) -> Option<&mut RoomInfo> {
        self.rooms.iter_mut().find(|r| r.id == room_id)
    }

    fn send_room_info(&self, room: &RoomInfo) {
        self.send_to_room_users(&room, RoomMessage::RoomInfo(room.clone()));
    }

    fn send_to_room_users(&self, room: &RoomInfo, msg: RoomMessage) {
        for (client_user_id, client) in self.clients.iter() {
            if room.users.iter().any(|u| u.user_id == *client_user_id) {
                self.send_to_client(client, msg.clone());
            }
        }
    }

    fn send_to_client(&self, client: &Addr<WebSocket>, msg: RoomMessage) {
        client.do_send(RoomNotification(msg));
    }
}

struct WebSocket {
    heartbeat: Instant,

    data: Data<Mutex<AppState>>,

    user: User,
    room: Option<RoomId>,
}

impl Actor for WebSocket {
    type Context = ws::WebsocketContext<Self>;

    /// Method is called on actor start. We start the heartbeat process here.
    fn started(&mut self, ctx: &mut Self::Context) {
        self.data
            .lock()
            .unwrap()
            .add_client(self.user.id, ctx.address());

        self.heartbeat(ctx);
    }

    fn stopped(&mut self, ctx: &mut Self::Context) {
        self.data().remove_client(ctx.address());
    }
}

impl Handler<RoomNotification> for WebSocket {
    type Result = ();

    fn handle(&mut self, msg: RoomNotification, ctx: &mut Self::Context) -> Self::Result {
        self.respond(ctx, msg.0)
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WebSocket {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => {
                self.heartbeat = Instant::now();
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {
                self.heartbeat = Instant::now();
            }
            Ok(ws::Message::Text(msg)) => {
                println!("Text received: {:?}", msg);

                if let Ok(msg) = deserialize_json::<RoomRequest>(&msg) {
                    self.handle_request(ctx, msg)
                }
            }
            Ok(ws::Message::Binary(bin)) => {
                println!("Binary received: {:?}", &bin);

                if let Ok(msg) = deserialize_binary::<RoomRequest>(&bin) {
                    self.handle_request(ctx, msg)
                }
            }
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            _ => ctx.stop(),
        }
    }
}

impl WebSocket {
    fn new(user: User, data: Data<Mutex<AppState>>) -> Self {
        Self {
            heartbeat: Instant::now(),
            user,
            data,
            room: None,
        }
    }

    fn data(&self) -> std::sync::MutexGuard<AppState> {
        self.data.lock().unwrap()
    }

    fn handle_request(&mut self, ctx: &mut ws::WebsocketContext<Self>, msg: RoomRequest) {
        match msg {
            RoomRequest::UserInfo => self.respond(ctx, RoomMessage::UserInfo(self.user.id)),
            RoomRequest::RoomInfo => {
                if let Some(room_id) = self.room {
                    if let Some(room) = self.data().room(room_id).cloned() {
                        self.respond(ctx, RoomMessage::RoomInfo(room));
                    }
                }
            }
            RoomRequest::CreateRoom => self.create_room(ctx),
            RoomRequest::JoinRoom(room_id) => self.join_room(room_id, ctx),
            RoomRequest::LeaveRoom => self.leave_room(),
            RoomRequest::Vote(card_index) => self.vote(card_index),
            RoomRequest::Reveal => self.reveal_votes(),
            RoomRequest::Reset => self.reset_votes(),
            msg => {
                println!("unhandled req: {:?}", msg);
            }
        }
    }

    fn create_room(&mut self, ctx: &mut <Self as Actor>::Context) {
        let room_id = self.data().create_room(self.user.id);

        self.respond(ctx, RoomMessage::RoomCreated(room_id));
    }

    fn join_room(&mut self, room_id: RoomId, ctx: &mut <Self as Actor>::Context) {
        let room_exists = self.data().has_room_with_id(room_id);

        if room_exists {
            self.room = Some(room_id);

            self.data().add_user_to_room(self.user.id.clone(), room_id);
        } else {
            self.respond(ctx, RoomMessage::NoSuchRoom(room_id));
        }
    }

    fn leave_room(&mut self) {
        if let Some(room_id) = self.room.take() {
            self.data()
                .remove_user_from_room(self.user.id.clone(), room_id);
        }
    }

    fn vote(&mut self, vote: u32) {
        if let Some(room_id) = self.room {
            self.data().vote(self.user.id, room_id, vote);
        }
    }

    fn reveal_votes(&mut self) {
        if let Some(room_id) = self.room {
            self.data().reveal_votes(self.user.id, room_id);
        }
    }

    fn reset_votes(&mut self) {
        if let Some(room_id) = self.room {
            self.data().reset_votes(self.user.id, room_id);
        }
    }

    fn heartbeat(&self, ctx: &mut <Self as Actor>::Context) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.heartbeat) > CLIENT_TIMEOUT {
                println!("Websocket Client heartbeat failed, disconnecting!");
                ctx.stop();
            } else {
                ctx.ping(b"");
            }
        });
    }

    fn respond(&self, ctx: &mut ws::WebsocketContext<Self>, msg: RoomMessage) {
        ctx.text(serialize_json(&msg).unwrap());
    }
}

async fn ws_index(
    req: HttpRequest,
    stream: web::Payload,
    data: Data<Mutex<AppState>>,
) -> Result<HttpResponse, Error> {
    let mut res = ws::handshake(&req)?;

    let (user, is_new) = req
        .cookie("session_id")
        .and_then(|cookie| Uuid::from_str(cookie.value()).ok())
        .and_then(|uuid| data.lock().unwrap().user_with_session_id(uuid.as_u128()))
        .map_or_else(
            /* no such user. create new */
            || (data.lock().unwrap().create_user(), true),
            /* existing user */
            |u| (u, false),
        );

    if is_new {
        println!(
            "User created with user id {} and session id {}",
            &user.id, &user.session_id
        );

        res.cookie(
            Cookie::build("session_id", &Uuid::from_u128(user.session_id).to_string())
                .http_only(true)
                .secure(false)
                .same_site(SameSite::Strict)
                .finish(),
        );
    }

    Ok(res.streaming(WebsocketContext::create(WebSocket::new(user, data), stream)))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "actix_server=info,actix_web=info");
    env_logger::init();

    let state = Data::new(Mutex::new(AppState {
        clients: Vec::new(),
        rooms: Vec::new(),
        users: Vec::new(),
    }));

    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .wrap(middleware::Logger::default())
            .route("/ws/", web::get().to(ws_index))
    })
    .bind("127.0.0.1:8082")?
    .run()
    .await
}
