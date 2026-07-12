//! Counter-Strike 2's official Game State Integration support.

mod config;
mod parser;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Manager};

pub use config::ensure_gsi_config;
use parser::GsiState;

/// A single localhost listener receives official CS2 GSI POSTs. It is started
/// once because CS2 opens the connection only after it reads the CFG at launch.
pub struct Cs2GsiListener {
    started: AtomicBool,
    state: Arc<Mutex<GsiState>>,
}

impl Cs2GsiListener {
    pub fn new() -> Self {
        Self {
            started: AtomicBool::new(false),
            state: Arc::new(Mutex::new(GsiState::default())),
        }
    }

    pub fn start(&self, app: AppHandle) {
        if self.started.swap(true, Ordering::SeqCst) {
            return;
        }

        let port = app
            .state::<crate::settings::SettingsManager>()
            .get()
            .general
            .cs2_gsi_port;
        let listener = app.state::<Cs2GsiListener>();
        let state = listener.state.clone();

        if let Err(error) = std::thread::Builder::new()
            .name("prism-cs2-gsi".into())
            .spawn(move || {
                let address = format!("127.0.0.1:{port}");
                let server = match tiny_http::Server::http(&address) {
                    Ok(server) => server,
                    Err(error) => {
                        eprintln!("[cs2-gsi] could not bind {address}: {error}");
                        return;
                    }
                };
                eprintln!("[cs2-gsi] listening on {address}");

                for mut request in server.incoming_requests() {
                    let response = if request.method() != &tiny_http::Method::Post {
                        tiny_http::Response::from_string("Method not allowed").with_status_code(405)
                    } else {
                        let mut body = String::new();
                        let result = std::io::Read::read_to_string(request.as_reader(), &mut body)
                            .ok()
                            .and_then(|_| serde_json::from_str::<serde_json::Value>(&body).ok());

                        if let Some(payload) = result {
                            if let Ok(mut tracker) = state.lock() {
                                for moment in tracker.consume(&payload) {
                                    crate::games::trigger::trigger_auto_clip(&app, moment);
                                }
                            }
                            tiny_http::Response::from_string("").with_status_code(204)
                        } else {
                            tiny_http::Response::from_string("Invalid GSI payload")
                                .with_status_code(400)
                        }
                    };
                    let _ = request.respond(response);
                }
            })
        {
            eprintln!("[cs2-gsi] failed to spawn listener: {error}");
        }
    }
}
