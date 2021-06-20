use yew::prelude::*;

#[derive(Properties, Clone)]
pub struct CardProps {
    pub value: String,
    pub onclick: Callback<MouseEvent>,
    pub selected: bool,
}

pub struct Card {
    props: CardProps,
}

impl Component for Card {
    type Message = ();
    type Properties = CardProps;

    fn create(props: Self::Properties, _link: yew::ComponentLink<Self>) -> Self {
        Self { props }
    }

    fn update(&mut self, _msg: Self::Message) -> yew::ShouldRender {
        false
    }

    fn change(&mut self, props: Self::Properties) -> yew::ShouldRender {
        self.props = props;

        true
    }

    fn view(&self) -> yew::Html {
        log::info!("{}", self.props.selected);

        let class = if self.props.selected {
            "card selected"
        } else {
            "card"
        };

        html! {
            <div onclick=self.props.onclick.clone() class=class>{ &self.props.value }</div>
        }
    }
}
