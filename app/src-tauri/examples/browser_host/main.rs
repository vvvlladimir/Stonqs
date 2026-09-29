//! The real host's commands over HTTP, for the frontend running in a browser (`pnpm dev:browser`,
//! `pnpm e2e`). `src/lib/browserHost.ts` forwards every IPC call to `POST /__host/<command>`,
//! and this answers it by calling the same command function the desktop app registers — so a
//! screen in the browser is fed exactly what the app would feed it, over a fresh demo portfolio.
//!
//! One request at a time on one thread: the commands share one `AppState`, as in the app.

mod routes;
mod setup;

use sq_app_lib::state::AppState;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use tauri::{Manager, State};

const PREFIX: &str = "/__host/";

fn main() {
    let port = std::env::var("BROWSER_HOST_PORT").unwrap_or_else(|_| "1430".into());
    let dir = setup::data_dir(&port);
    let app = setup::demo_app(&dir);
    let state = app.state::<AppState>();
    let listener = TcpListener::bind(format!("127.0.0.1:{port}")).expect("the browser host's port");
    println!(
        "browser host: demo profile in {}, listening on 127.0.0.1:{port}",
        dir.display()
    );
    // Every page asks the shell's questions again, and e2e opens many pages at once: a repeated
    // read is answered from here until anything that is not a read runs (`routes::READS`).
    let mut answered = HashMap::new();
    for stream in listener.incoming().flatten() {
        if let Err(e) = serve(stream, &state, &mut answered) {
            eprintln!("browser host: {e}");
        }
    }
}

fn serve(
    mut stream: TcpStream,
    state: &State<'_, AppState>,
    answered: &mut HashMap<String, serde_json::Value>,
) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let path = line.split_whitespace().nth(1).unwrap_or_default().to_string();
    let mut length = 0;
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header)? == 0 || header.trim().is_empty() {
            break;
        }
        if let Some((name, value)) = header.split_once(':')
            && name.eq_ignore_ascii_case("content-length")
        {
            length = value.trim().parse().unwrap_or(0);
        }
    }
    let mut body = vec![0; length];
    reader.read_exact(&mut body)?;

    let Some(command) = path.strip_prefix(PREFIX) else {
        return respond(&mut stream, 404, &serde_json::json!({ "err": "not a host path" }));
    };
    // The readiness probe `playwright.config.ts` waits on.
    if command == "ping" {
        return respond(&mut stream, 200, &serde_json::json!({ "ok": true }));
    }
    let args: serde_json::Value = serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null);
    if let Some(reason) = routes::DESKTOP_ONLY.iter().find(|(name, _)| *name == command) {
        // A known command the browser cannot run, which a screen may well ask for: answered as an
        // error the screen handles, and marked so e2e does not count it as a bug. Not a 5xx
        // status, which every browser also logs as a console error of its own.
        let err = serde_json::json!({ "code": "internal", "message": format!("{command} is desktop-only ({})", reason.1) });
        return respond(
            &mut stream,
            200,
            &serde_json::json!({ "err": err, "desktop_only": true }),
        );
    }
    let read = routes::READS.contains(&command);
    let key = format!("{command} {args}");
    if read && let Some(body) = answered.get(&key) {
        return respond(&mut stream, 200, body);
    }
    if !read {
        answered.clear();
    }
    match routes::answer(state, command, &args) {
        Some(Ok(value)) => {
            let body = serde_json::json!({ "ok": value });
            if read {
                answered.insert(key, body.clone());
            }
            respond(&mut stream, 200, &body)
        }
        Some(Err(err)) => respond(&mut stream, 200, &serde_json::json!({ "err": err })),
        None => {
            let err = serde_json::json!({ "code": "internal", "message": format!("the browser host has no route for {command}") });
            respond(&mut stream, 404, &serde_json::json!({ "err": err }))
        }
    }
}

fn respond(stream: &mut TcpStream, status: u16, body: &serde_json::Value) -> std::io::Result<()> {
    let body = body.to_string();
    // One write: `write!` on a bare socket is a syscall per fragment, and Nagle's algorithm then
    // holds the tail back for the peer's delayed ACK — tens of milliseconds on every answer.
    let response = format!(
        "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.set_nodelay(true)?;
    stream.write_all(response.as_bytes())
}
