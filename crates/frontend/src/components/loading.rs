use yew::prelude::*;

pub struct Loading;

impl Component for Loading {
    type Message = ();
    type Properties = ();

    fn create(props: Self::Properties, link: yew::ComponentLink<Self>) -> Self {
        Self {}
    }

    fn update(&mut self, msg: Self::Message) -> yew::ShouldRender {
        false
    }

    fn change(&mut self, _props: Self::Properties) -> yew::ShouldRender {
        false
    }

    fn view(&self) -> yew::Html {
        html! {
            <div class="loading">{ "Loading" }</div>
        }
    }
}
