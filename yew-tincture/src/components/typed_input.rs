use yew::prelude::*;

use std::str::FromStr;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[derive(Clone)]
pub struct RawAndParsed<T>
    where
        T: FromStr + Clone,
        <T as FromStr>::Err: Clone,
{
    raw_value: String,
    parsed: Result<T, <T as FromStr>::Err>,
}

impl<T> RawAndParsed<T>
    where
        T: FromStr + Clone,
        <T as FromStr>::Err: Clone,
{
    /// Create a RawAndParsed with empty value.
    fn empty() -> Self
        where
            T: std::fmt::Display,
    {
        let raw_value = "".to_string();
        let parsed = raw_value.parse();
        Self {
            raw_value,
            parsed,
        }
    }

    /// Create a RawAndParsed with a good value.
    fn from_initial(value: T) -> Self
        where
            T: std::fmt::Display,
    {
        let raw_value = format!("{}", value);
        let parsed = Ok(value);
        Self {
            raw_value,
            parsed,
        }
    }

}

fn new_focus_state() -> Rc<Cell<FocusState>> {
    Rc::new(Cell::new(FocusState::IsBlurred))
}

#[derive(PartialEq,Clone)]
pub struct TypedInputStorage<T>
    where
        T: FromStr + Clone,
        <T as FromStr>::Err: Clone,
{
    rc: Rc<RefCell<TypedInputStorageInner<T>>>,
}

impl<T> TypedInputStorage<T>
    where
        T: FromStr + Clone,
        <T as FromStr>::Err: Clone,
        Result<T, <T as FromStr>::Err>: Clone,
{
    /// Create a TypedInputStorage with an empty value.
    pub fn empty() -> Self
        where
            T: std::fmt::Display,
    {
        Self::create(RawAndParsed::empty())
    }

    /// Create a TypedInputStorage with an empty value.
    pub fn from_initial(value: T) -> Self
        where
            T: std::fmt::Display,
    {
        Self::create(RawAndParsed::from_initial(value))
    }

    /// Create a TypedInputStorage with an empty value.
    fn create(raw_and_parsed: RawAndParsed<T>) -> Self
        where
            T: std::fmt::Display,
    {
        let inner = TypedInputStorageInner {
            raw_and_parsed,
            focus_state: new_focus_state(),
        };
        let me = Rc::new(RefCell::new(inner));
        Self { rc: me }
    }

    pub fn parsed(&self) -> Result<T, <T as FromStr>::Err> {
        self.rc.borrow().raw_and_parsed.parsed.clone()
    }

    /// Update the value if the user is not editing it.
    ///
    /// See also the `set()` method.
    pub fn set_if_not_focused(&mut self, value: T)
        where
            T: std::fmt::Display,
    {
        use std::ops::Deref;
        {
            let mut inner = self.rc.borrow_mut();

            match (*(inner.focus_state).deref()).get() {
                FocusState::IsFocused => {}
                FocusState::IsBlurred => {
                    inner.raw_and_parsed.raw_value = format!("{}", value);
                    inner.raw_and_parsed.parsed = Ok(value);
                }
            }
        }
    }

}

struct TypedInputStorageInner<T>
    where
        T: FromStr + Clone,
        <T as FromStr>::Err: Clone,
{
    raw_and_parsed: RawAndParsed<T>,
    // TODO: does this need to be Rc<Cell<_>> or can I make it &'a _?
    focus_state: Rc<Cell<FocusState>>,
}

impl<T> PartialEq for TypedInputStorageInner<T>
    where
        T: FromStr + Clone,
        <T as FromStr>::Err: Clone,
{
    fn eq(&self, rhs: &Self) -> bool {
        // I am not sure when yew uses this. Here is my
        // best effort implementation.
        Rc::ptr_eq(&self.focus_state, &rhs.focus_state) &&
            self.raw_and_parsed.raw_value == rhs.raw_and_parsed.raw_value
    }
}

#[derive(PartialEq, Clone, Copy, Debug)]
enum FocusState {
    IsFocused,
    IsBlurred,
}

impl Default for FocusState {
    fn default() -> FocusState {
        FocusState::IsBlurred
    }
}

pub struct TypedInput<T>
    where
        T: FromStr + Clone,
        <T as FromStr>::Err: Clone,
{
    raw_value_copy: String, // TODO: can we remove this and just use storage?
    storage: TypedInputStorage<T>,
    placeholder: String,
    on_send_valid: Option<Callback<T>>,
    on_input: Option<Callback<RawAndParsed<T>>>,
}

pub enum Msg {
    NewValue(String),
    OnFocus,
    OnBlur,
    SendValueIfValid,
    Ignore,
}

#[derive(PartialEq, Clone)]
pub struct Props<T>
    where
        T: FromStr + Clone,
        <T as FromStr>::Err: Clone,
{
    pub storage: TypedInputStorage<T>,
    pub placeholder: String,
    /// Called when the user wants to send a valid value
    pub on_send_valid: Option<Callback<T>>,
    /// Called whenever the user changes the value
    pub on_input: Option<Callback<RawAndParsed<T>>>,
}

impl<T> Default for Props<T>
    where
        T: FromStr + Clone + std::fmt::Display,
        <T as FromStr>::Err: Clone,
{
    fn default() -> Self {
        Props {
            storage: TypedInputStorage::empty(),
            placeholder: "".to_string(),
            on_send_valid: None,
            on_input: None,
        }
    }
}

impl<T> TypedInput<T>
    where
        T: FromStr + Clone,
        <T as FromStr>::Err: Clone,
{
    fn send_value_if_valid(&mut self) {
        if let Some(ref mut callback) = self.on_send_valid {
            if let Ok(value) = &self.storage.rc.borrow().raw_and_parsed.parsed {
                callback.emit(value.clone());
            }
        }
    }
}

impl<T> Component for TypedInput<T>
    where
        T: 'static + Clone + PartialEq + FromStr + std::fmt::Display,
        <T as FromStr>::Err: Clone,
{
    type Message = Msg;
    type Properties = Props<T>;

    fn create(props: Self::Properties, _link: ComponentLink<Self>) -> Self {
        let raw_value_copy = props.storage.rc.borrow().raw_and_parsed.raw_value.clone();
        Self {
            raw_value_copy,
            storage: props.storage,
            placeholder: props.placeholder,
            on_send_valid: props.on_send_valid,
            on_input: props.on_input,
        }
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        self.raw_value_copy = props.storage.rc.borrow().raw_and_parsed.raw_value.clone();
        self.storage = props.storage;
        self.placeholder = props.placeholder;
        self.on_send_valid = props.on_send_valid;
        true
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::NewValue(raw_value) => {
                self.raw_value_copy = raw_value.clone();
                let parsed = raw_value.parse();
                let stor2 = {
                    let mut stor = self.storage.rc.borrow_mut();
                    stor.raw_and_parsed.raw_value = raw_value;
                    stor.raw_and_parsed.parsed = parsed;
                    stor.raw_and_parsed.clone()
                };

                if let Some(ref mut callback) = self.on_input {
                    callback.emit(stor2);
                }

            }
            Msg::OnFocus => {
                let stor = self.storage.rc.borrow_mut();
                stor.focus_state.replace(FocusState::IsFocused);
            }
            Msg::OnBlur => {
                {
                    let stor = self.storage.rc.borrow_mut();
                    stor.focus_state.replace(FocusState::IsBlurred);
                }
                self.send_value_if_valid();
            }
            Msg::SendValueIfValid => {
                self.send_value_if_valid();
                return false;
            }
            Msg::Ignore => {
                return false; // no need to rerender DOM
            }
        }
        true
    }
}

impl<T> Renderable<TypedInput<T>> for TypedInput<T>
    where
        T: 'static + Clone + PartialEq + FromStr + std::fmt::Display,
        <T as FromStr>::Err: Clone,
{
    fn view(&self) -> Html<Self> {
        let input_class = match &self.storage.rc.borrow().raw_and_parsed.parsed {
            Ok(_) => "ranged-value-input",
            Err(_) => "ranged-value-input-error",
        };

        html! {
            <input type="text",
                class=input_class,
                placeholder=&self.placeholder,
                value=&self.raw_value_copy,
                oninput=|e| Msg::NewValue(e.value),
                onfocus=|_| Msg::OnFocus,
                onblur=|_| Msg::OnBlur,
                onkeypress=|e| {
                    if e.key() == "Enter" { Msg::SendValueIfValid } else { Msg::Ignore }
                },
                />
        }
    }
}
