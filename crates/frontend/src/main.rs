mod agents;
mod components;
mod route;

use agents::WebSocketAgent;
use yew::prelude::*;
use yew_router::prelude::*;

use route::{switch, Route};

struct App {
    // Keeps WebSocket connection alive
    _ws_agent: Box<dyn Bridge<WebSocketAgent>>,
}

impl Component for App {
    type Message = ();
    type Properties = ();

    fn create(_props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let ws_agent = WebSocketAgent::bridge(link.callback(|_| {}));
        Self {
            _ws_agent: ws_agent,
        }
    }

    fn update(&mut self, _msg: Self::Message) -> ShouldRender {
        false
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        html! {
            <div class="container">
                <Router<Route> render=Router::render(switch) />
            </div>
        }
    }
}

fn main() {
    wasm_logger::init(wasm_logger::Config::default());

    yew::start_app::<App>();
}
