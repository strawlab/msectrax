#[macro_use]
extern crate log;

mod error;

use futures::Future;
use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::Mutex;

use actix::prelude::*;
use actix_web::{
    http, middleware, server, App, HttpRequest, HttpResponse, State,
    AsyncResponder, FutureResponse, Json, Error};

use structopt::StructOpt;

use crossbeam_channel::Receiver;

use msectrax_comms::{DeviceMode, ClosedLoopMode};
use crate::error::Error as MyError;

type MyResult<T> = std::result::Result<T,MyError>;

#[allow(dead_code)]
#[cfg(target_os = "macos")]
const DEFAULT_DEVICE: &'static str = "/dev/tty.usbmodem1423";

#[allow(dead_code)]
#[cfg(target_os = "linux")]
const DEFAULT_DEVICE: &'static str = "/dev/ttyACM0";

#[derive(Debug, StructOpt)]
#[structopt()]
struct Arguments {
    /// Path to the msectrax device
    // Note when https://github.com/TeXitoi/structopt/issues/142 is implemented,
    // we will switch this to use DEFAULT_DEVICE var above and remove the
    // `#[allow(dead_code)]` lines.
    #[structopt(long="--device", parse(from_os_str), default_value = "/dev/ttyACM0")]
    device: PathBuf,

    /// The address to bind for the HTTP server
    #[structopt(long="--http-addr", default_value = "0.0.0.0:8080")]
    http_addr: String,

    /// Prints example HTTP requests and other information, then quit
    #[structopt(long="--print-example-requests", short="-p")]
    show_examples: bool,

    /// Perform request logging
    #[structopt(long="--logging", short="-l")]
    logging: bool,

}

fn show_examples(http_addr: &str) {
    use msectrax_comms::ToDevice::*;

    let mut closed_loop_state = msectrax_comms::SetDeviceState::default();
    closed_loop_state.mode = DeviceMode::ClosedLoop(ClosedLoopMode::Proportional);
    let example_msgs = [
        EchoRequest8((1,2,3,4,5,6,7,8)),
        SetState(msectrax_comms::SetDeviceState::default()),
        SetState(closed_loop_state),
        QueryState,
        QueryAnalog,
        SetGalvos((0,0)),
    ];
    let bufs: Vec<String> = example_msgs.iter().map(|msg| format!("    {}",serde_json::to_string(&msg).unwrap()) ).collect();
    println!("# Example messages understood as JSON HTTP requests: \n\n{}\n", bufs.join("\n\n"));

    let curl_cmd = format!(r#"curl --header "Content-Type: application/json" --request POST --data '{{"EchoRequest8":[1,2,3,4,5,6,7,8]}}' -v http://{}/"#,
        http_addr);

    println!("# Example usage with curl:

        {}", curl_cmd);
}

enum VersionCheck {
    Started(std::time::Instant),
    Success,
}

struct SerialThread {
    ser: Box<dyn serialport::SerialPort>,
    outq: Receiver<msectrax_comms::ToDevice>,
    from_device_tx: crossbeam_channel::Sender<msectrax_comms::FromDevice>,
}

impl SerialThread {
    fn new(
        device: &std::path::Path,
        outq: Receiver<msectrax_comms::ToDevice>,
        from_device_tx: crossbeam_channel::Sender<msectrax_comms::FromDevice>,
    ) -> MyResult<Self>
    {

        let ser = {
            use serialport::*;

            let settings = SerialPortSettings {
                baud_rate: msectrax_comms::BPS_HZ,
                data_bits: DataBits::Eight,
                flow_control: FlowControl::None,
                parity: Parity::None,
                stop_bits: StopBits::One,
                timeout: std::time::Duration::from_millis(10),
            };
            serialport::open_with_settings(&device, &settings)?
        };

        Ok(Self {
            ser,
            outq,
            from_device_tx,
        })
    }

    fn my_write(&mut self, buf: &[u8]) -> std::io::Result<()> {
        trace!("sending {} bytes", buf.len());
        for byte in buf.iter() {
            trace!("  sending byte: {}", byte);
        }
        self.ser.write(buf)?;
        Ok(())
    }

    fn run(&mut self, flag: thread_control::Flag) -> MyResult<()> {
        let mut send_buf = [0; 256];
        let mut read_buf = [0; 256];

        let mut decode_buf = [0; 256];
        let mut decoder = mini_rxtx::Decoder::new(&mut decode_buf);

        // initiate version check
        let mut version_check_state = {
            let serialized_msg = mini_rxtx::serialize_msg(&msectrax_comms::ToDevice::QueryDatatypesVersion, &mut send_buf).expect("serialize_msg");
            self.my_write( serialized_msg.framed_slice() )?;
            info!("Sent firmware version request.");
            VersionCheck::Started(std::time::Instant::now())
        };

        while flag.alive() {


            loop {
                // Handle all new messages, but do not block.
                match self.outq.recv_timeout(std::time::Duration::from_millis(0)) {
                    Ok(msg) => {
                        debug!("sending message {:?}", msg);
                        let serialized_msg = mini_rxtx::serialize_msg(&msg, &mut send_buf).expect("serialize_msg");
                        self.my_write( serialized_msg.framed_slice() )?;
                    },
                    Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                        break;
                    },
                    Err(e) => {
                        return Err(e.into());
                    },
                }
            }


            // TODO: this could be made (much) more efficient. Right
            // now, we wake up every timeout duration and run the whole
            // cycle when no byte arrives.
            match self.ser.read(&mut read_buf) {
                Ok(n_bytes_read) => {
                    for i in 0..n_bytes_read {
                        let byte = read_buf[i];
                        trace!("read byte {} (char {})",
                            byte, String::from_utf8_lossy(&read_buf[i..i+1]));
                        match decoder.consume::<msectrax_comms::FromDevice>(byte) {
                            mini_rxtx::Decoded::Msg(msg) => {
                                if let msectrax_comms::FromDevice::EchoDatatypesVersion(firmware_version) = msg {
                                    info!("Firmware version {}", firmware_version);
                                    if firmware_version != msectrax_comms::DATATYPES_VERSION {
                                        return Err(crate::error::Error::FirmwareVersionMismatch((firmware_version,msectrax_comms::DATATYPES_VERSION)));
                                    } else {
                                        info!("firmware version OK.");
                                        version_check_state = VersionCheck::Success;
                                    }
                                } else {
                                    self.from_device_tx.send(msg).unwrap();
                                }
                            }
                            mini_rxtx::Decoded::FrameNotYetComplete => {}
                            mini_rxtx::Decoded::Error(e) => {
                                return Err(e.into());
                            }
                        }
                    }
                },
                Err(e) => {
                    match e.kind() {
                        std::io::ErrorKind::TimedOut => {},
                        _ => {return Err(e.into());},
                    }
                }
            }

            version_check_state = match version_check_state {
                VersionCheck::Started(start) => {
                    if start.elapsed() > std::time::Duration::from_millis(500) {
                        return Err(crate::error::Error::FirmwareVersionCheckTimeout);
                    } else {
                        VersionCheck::Started(start)
                    }
                },
                VersionCheck::Success => VersionCheck::Success,
            };
        }

        Ok(())
    }
}


/// This is state where we will store *SerialExecutor* address.
struct AppState {
    serial_executor: actix::Addr<SerialExecutor>,
}

pub struct WrappedToDevice {
    pub to_device: msectrax_comms::ToDevice,
}

impl Message for WrappedToDevice {
    type Result = Result<msectrax_comms::FromDevice, Error>;
}

struct SerialExecutor{
    tx: crossbeam_channel::Sender<msectrax_comms::ToDevice>,
    rx: Arc<Mutex<crossbeam_channel::Receiver<msectrax_comms::FromDevice>>>,
}

impl actix::Actor for SerialExecutor {
    type Context = actix::SyncContext<Self>;
}

impl Handler<WrappedToDevice> for SerialExecutor {
    type Result = Result<msectrax_comms::FromDevice, Error>;

    fn handle(&mut self, msg: WrappedToDevice, _: &mut Self::Context) -> Self::Result {

        let rx = self.rx.lock(); // grab lock early to have exclusive control over serial thread.
        // Keep lock for the the entire scope of this handle() call.
        match self.tx.send(msg.to_device) {
            Ok(()) => {},
            Err(e) => {
                // If we have a serial error, stop the entire proxy program.
                error!("failed to send to serial in handler");
                actix::System::current().stop_with_code(1);
                let e2 = actix_web::error::InternalError::new(
                    e,
                    actix_web::http::StatusCode::INTERNAL_SERVER_ERROR);
                return Err(e2.into());
            },
        }

        // wait for response
        let from_device = match rx.recv() {
            Ok(fd) => fd,
            Err(e) => {
                // If we have a serial error, stop the entire proxy program.
                error!("{}", e);
                actix::System::current().stop_with_code(1);
                let e2 = actix_web::error::InternalError::new(
                    e,
                    actix_web::http::StatusCode::INTERNAL_SERVER_ERROR);
                return Err(e2.into());
            }
        };
        Ok(from_device)
    }
}

fn handle_http_post((item, state): (Json<msectrax_comms::ToDevice>, State<AppState>)) -> FutureResponse<HttpResponse> {
    state.serial_executor
        .send(WrappedToDevice {
            to_device: item.into_inner(),
        })
        .from_err()
        .and_then(|res| match res {
            Ok(from_dev) => Ok(HttpResponse::Ok().json(from_dev)),
            Err(_) => Ok(HttpResponse::InternalServerError().into()),
        })
        .responder()
}

const INDEX_HTML: &'static [u8] = include_bytes!("../msectrax-bui-frontend/dist/index.html");
const STYLE_CSS: &'static [u8] = include_bytes!("../msectrax-bui-frontend/dist/style.css");
const FRONTEND_JS: &'static [u8] = include_bytes!("../msectrax-bui-frontend/dist/msectrax-bui-frontend.js");
const FRONTEND_WASM: &'static [u8] = include_bytes!("../msectrax-bui-frontend/dist/msectrax-bui-frontend.wasm");

fn index_html(_req: &HttpRequest<AppState>) -> Result<HttpResponse,Error> {
    Ok(HttpResponse::Ok()
        .content_type("text/html")
        .body(INDEX_HTML))
}

fn style_css(_req: &HttpRequest<AppState>) -> Result<HttpResponse,Error> {
    Ok(HttpResponse::Ok()
        .content_type("text/css")
        .body(STYLE_CSS))
}

fn msectrax_js(_req: &HttpRequest<AppState>) -> Result<HttpResponse,Error> {
    Ok(HttpResponse::Ok()
        .content_type("application/javascript")
        .body(FRONTEND_JS))
}

fn msectrax_wasm(_req: &HttpRequest<AppState>) -> Result<HttpResponse,Error> {
    Ok(HttpResponse::Ok()
        .content_type("application/wasm")
        .body(FRONTEND_WASM))
}

fn main() -> MyResult<()> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "msectrax_proxy=info,actix_web=info,error");
    }

    env_logger::init();
    let args = Arguments::from_args();
    if args.show_examples {
        show_examples(&args.http_addr);
        return Ok(());
    }

    info!("device: {}", args.device.display());
    let http_addr = args.http_addr.clone();
    let logging = args.logging;

    let (tx, rx) = crossbeam_channel::bounded(100);
    let (from_device_tx, from_device_rx) = crossbeam_channel::bounded(100);
    let rx_arc = Arc::new(Mutex::new(from_device_rx));

    let thread_builder = std::thread::Builder::new()
        .name("comms".to_string());
    let (flag, control) = thread_control::make_pair();
    let _thread_handle = thread_builder.spawn(move || {
        let mut x = SerialThread::new(&args.device, rx, from_device_tx).expect("new");
        x.run(flag).expect("run");
    })?;

    // give serial thread time to start
    std::thread::sleep(std::time::Duration::from_millis(100));

    if control.is_done() {
        error!("serial thread failed to start as expected");
        return Err(crate::error::Error::SerialStart);
    };

    let sys = actix::System::new("msectrax-proxy");

    // Start 1 serial executor
    let addr = SyncArbiter::start(1, move || {
        let tx = tx.clone();
        let rx = rx_arc.clone();
        SerialExecutor{ tx, rx }
    });

    // Start http server
    server::new(move || {
        let state = AppState{serial_executor: addr.clone()};

        let app = App::with_state(state);
        let app: App<AppState> = match logging {
            true => app.middleware(middleware::Logger::default()),
            false => app,
        };
        app
            .resource("/callback", |r| r.method(http::Method::POST).with(handle_http_post))
            .resource("/", |r| r.method(http::Method::GET).f(index_html))
            .resource("/index.html", |r| r.method(http::Method::GET).f(index_html))
            .resource("/style.css", |r| r.method(http::Method::GET).f(style_css))
            .resource("/msectrax-bui-frontend.js", |r| r.method(http::Method::GET).f(msectrax_js))
            .resource("/msectrax-bui-frontend.wasm", |r| r.method(http::Method::GET).f(msectrax_wasm))

    }).bind(&http_addr)
        .unwrap()
        .start();

    info!("Started http server: {}", http_addr);
    let _ = sys.run();

    Ok(())
}
