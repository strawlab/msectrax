use yew::prelude::*;

pub struct CheckboxLabel {
    css_id: String,
    label: String,
    oncheck: Option<Callback<bool>>,
    checked: bool,
}

pub enum Msg {
    Toggle,
}

#[derive(PartialEq, Clone)]
pub struct Props {
    pub label: String,
    pub oncheck: Option<Callback<bool>>,
    pub initially_checked: bool,
}

impl Default for Props {
    fn default() -> Self {
        Props {
            label: "label".into(),
            oncheck: None,
            initially_checked: false,
        }
    }
}

impl Component for CheckboxLabel {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, _link: ComponentLink<Self>) -> Self {
        Self {
            css_id: uuid::Uuid::new_v4().to_string(),
            label: props.label,
            oncheck: props.oncheck,
            checked: props.initially_checked,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::Toggle => {
                let checked = !self.checked;
                self.checked = checked;
                if let Some(ref mut callback) = self.oncheck {
                    callback.emit(checked);
                }
            }
        }
        true
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        self.label = props.label;
        self.oncheck = props.oncheck;
        // ignore initially_checked
        true
    }
}

impl Renderable<CheckboxLabel> for CheckboxLabel {
    fn view(&self) -> Html<Self> {
        // Hmm, putting this in one html!{} macro fails, so make a list
        // manually.

        // Ideally, the structure would be <label><input /></label> but
        // I could not get this to work, so we are using `css_id` here.
        let el1 = html! {
            <input
                id=&self.css_id,
                type="checkbox",
                checked=self.checked,
                onclick=|_| Msg::Toggle,
                />
        };
        let el2 = html! {
            <label for=&self.css_id,>
                {&self.label}
            </label>
        };
        let mut vlist = yew::virtual_dom::vlist::VList::new();
        vlist.add_child(el1);
        vlist.add_child(el2);
        vlist.into()
    }
}
