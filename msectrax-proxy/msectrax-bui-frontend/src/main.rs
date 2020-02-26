#![recursion_limit="256"]

extern crate yew;
#[macro_use]
extern crate stdweb;

use yew::prelude::*;

use yew_tincture::components::{Button, TypedInput, TypedInputStorage,
    CheckboxLabel};
use yew::services::fetch::{FetchService, FetchTask, Request, Response};
use yew::services::{Task, IntervalService};
use yew::format::Json;

#[derive(Clone,PartialEq)]
struct DacValue(i16);

impl std::str::FromStr for DacValue {
    type Err = std::num::ParseIntError;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let v = i16::from_str(s)?;
        Ok(DacValue(v))
    }
}

impl std::fmt::Display for DacValue {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

struct Model {
    web: FetchService,
    _interval_task: Box<dyn Task>,
    ft: Option<FetchTask>,
    link: ComponentLink<Self>,
    dac1_initial: TypedInputStorage<DacValue>,
    dac2_initial: TypedInputStorage<DacValue>,
    last_state: Option<msectrax_comms::DeviceState>,
    query_state: bool,
}

enum Msg {
    Holdoff,
    Lockon,
    CheckState,
    Ignore,
    GotState(msectrax_comms::DeviceState),
    ToggleQueryState(bool),
}

impl Component for Model {
    type Message = Msg;
    type Properties = ();

    fn create(_: Self::Properties, mut link: ComponentLink<Self>) -> Self {
        let callback = link.send_back(|_| Msg::CheckState);
        let mut interval = IntervalService::new();
        let handle = interval.spawn(std::time::Duration::from_millis(1000),
            callback.into());
        Self {
            web: FetchService::new(),
            _interval_task: Box::new(handle),
            ft: None,
            link,
            dac1_initial: TypedInputStorage::from_initial(DacValue(-3365)),
            dac2_initial: TypedInputStorage::from_initial(DacValue(-3709)),
            last_state: None,
            query_state: true,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::Holdoff => {
                if let Ok(dac1) = self.dac1_initial.parsed() {
                    if let Ok(dac2) = self.dac2_initial.parsed() {
                        let msg = msectrax_comms::ToDevice::SetGalvos((dac1.0, dac2.0));
                        self.ft = Some(send_message(&msg, self));
                    }
                }
                return false; // don't update DOM, do that on return
            },
            Msg::Lockon => {
                if let Ok(dac1) = self.dac1_initial.parsed() {
                    if let Ok(dac2) = self.dac2_initial.parsed() {
                        let mut initial = match &self.last_state {
                            None => msectrax_comms::SetDeviceState::default(),
                            Some(s) => s.inner.clone(),
                        };
                        initial.mode = msectrax_comms::DeviceMode::ClosedLoop( msectrax_comms::ClosedLoopMode::Proportional );
                        initial.dac1_initial = dac1.0;
                        initial.dac2_initial = dac2.0;
                        let msg = msectrax_comms::ToDevice::SetState(initial);
                        self.ft = Some(send_message(&msg, self));
                    }
                }
                return false; // don't update DOM, do that on return
            },
            Msg::CheckState => {
                if self.query_state {
                    let msg = msectrax_comms::ToDevice::QueryState;
                    self.ft = Some(send_message(&msg, self));
                }
                return false; // don't update DOM, do that on return
            },
            Msg::GotState(state) => {
                self.last_state = Some(state);
            }
            Msg::ToggleQueryState(v) => {
                self.query_state = v;
            }
            Msg::Ignore => {}
        }
        true
    }
}

fn send_message(msg: &msectrax_comms::ToDevice, model: &mut Model) -> FetchTask {
    let post_request = Request::post("callback")
            .header("Content-Type", "application/json;charset=UTF-8")
            .body(Json(&msg))
            .expect("Failed to build request.");
    let callback = model.link.send_back(move |response: Response<Json<Result<msectrax_comms::FromDevice, _>>>| {
        let (meta, Json(data)) = response.into_parts();
        if !meta.status.is_success() {
            let rs = format!("Error sending message: {:?}", meta);
            js!{console.error(@{rs})};
        }
        match data {
            Ok(msectrax_comms::FromDevice::EchoState(state)) => {
                Msg::GotState(state)
            },
            Ok(msectrax_comms::FromDevice::Empty) => {
                Msg::Ignore
            },
            Ok(val) => {
                let rs = format!("Unexpected valid json result: {:?}", val);
                js!{console.error(@{rs})};
                Msg::Ignore
            },
            Err(err) => {
                let rs = format!("Error parsing json: {:?}", err);
                js!{console.error(@{rs})};
                Msg::Ignore
            },
        }
    });
    model.web.fetch(post_request, callback)
}

fn main() {
    yew::initialize();
    let app = App::<Model>::new();
    app.mount_to_body();
    yew::run_loop();
}

impl Model {
    fn view_state(&self) -> Html<Model> {
        if let Some(ref state) = self.last_state {
            let state_string = serde_yaml::to_string(state).unwrap();
            html! {
                <div>
                    {"Mode: "}{format!("{:?}",state.inner.mode)}
                    <div>
                        <p>{"DAC1: "}{format!("{}",state.dac1)}</p>
                        <p>{"DAC2: "}{format!("{}",state.dac2)}</p>
                        <p>{"ADC1: "}{format!("{}",state.adc1)}</p>
                        <p>{"ADC2: "}{format!("{}",state.adc2)}</p>
                    </div>
                    <div class="preformatted",>
                        {state_string}
                    </div>
                </div>
            }
        } else {
            html!{<div></div>}
        }
    }
}

impl Renderable<Model> for Model {
    fn view(&self) -> Html<Self> {
        html! {
            <div>
                <h1>{"msectrax"}</h1>
                <div class="border-1px",>
                    <h2>{"Center Position"}</h2>
                    <div class="my-padding",>
                        <label>{"DAC1"}
                            <TypedInput<DacValue>:
                                storage=&self.dac1_initial,
                                />
                        </label>
                    </div>
                    <div class="my-padding",>
                        <label>{"DAC2"}
                            <TypedInput<DacValue>:
                                storage=&self.dac2_initial,
                                />
                        </label>
                    </div>
                </div>
                <div class="border-1px",>
                    <h2>{"Actions"}</h2>
                    <div class="button-holder",>
                        <Button: title="Center", onsignal=|_| Msg::Holdoff,/>
                    </div>
                    <div class="button-holder",>
                        <Button: title="Lock", onsignal=|_| Msg::Lockon,/>
                    </div>
                </div>

                <div class="border-1px",>
                    <h2>{"Device State"}</h2>

                    <CheckboxLabel:
                        label="Query for updates",
                        initially_checked=self.query_state,
                        oncheck=|checked| Msg::ToggleQueryState(checked),
                        />

                    { self.view_state() }
                </div>
            </div>
        }
    }
}
