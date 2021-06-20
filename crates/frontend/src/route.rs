use yew::prelude::*;
use yew_router::prelude::*;

use crate::components::lobby::Lobby;
use crate::components::room::Room;

#[derive(Routable, PartialEq, Clone, Debug)]
pub(crate) enum Route {
    #[at("/")]
    Lobby,
    #[at("/room/:id")]
    Room { id: u32 },
}

pub(crate) fn switch(route: &Route) -> Html {
    match route {
        Route::Lobby => {
            html! { <Lobby /> }
        }
        Route::Room { id } => {
            html! { <Room id=*id /> }
        }
    }
}
