//! Spike (ADR 0011): in the hybrid dock nothing of WebView2 runs at rest, so a
//! panel or the Configurações app has to start it. How long does a WebView2
//! window take to show a real page of the DockMax?
//!
//! - **cold**: no WebView2 of this process is running (its browser process has
//!   exited), as after the dock has been at rest;
//! - **warm**: another WebView2 window of the process is open, so the browser
//!   process is already up.
//!
//! Each run makes a small window, loads `settings.html` from the built `dist`
//! folder (the real bundle, through a custom protocol as Tauri serves it) and
//! measures until the page is loaded and painted (`load`, then two animation
//! frames). Usage: `webview-startup-spike <dist folder>`.

use std::borrow::Cow;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use tao::dpi::{LogicalPosition, LogicalSize};
use tao::event::Event;
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::window::{Window, WindowBuilder};
use wry::http::Response;
use wry::{WebContext, WebView, WebViewBuilder, WebViewBuilderExtWindows};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Cold,
    /// Opens a window that stays, so the next ones are warm.
    KeepOpen,
    Warm,
}

#[derive(Debug)]
enum Msg {
    Go(Kind),
    Painted(f64),
    Done,
}

const ARGS: &str = "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --process-per-site --in-process-gpu";

const PROBE: &str = r#"
(() => {
  window.addEventListener('load', () =>
    requestAnimationFrame(() => requestAnimationFrame(() =>
      window.ipc.postMessage('painted:' + performance.now()))));
})();
"#;

fn content_type(path: &str) -> &'static str {
    match path.rsplit('.').next() {
        Some("html") => "text/html",
        Some("js") => "text/javascript",
        Some("css") => "text/css",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("woff2") => "font/woff2",
        _ => "application/octet-stream",
    }
}

fn webview_processes() -> usize {
    let me = std::process::id();
    let out = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            &format!(
                "@(Get-CimInstance Win32_Process -Filter \"Name='msedgewebview2.exe'\" | Where-Object {{ $_.ParentProcessId -eq {me} }}).Count"
            ),
        ])
        .output();
    out.ok()
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .and_then(|text| text.trim().parse().ok())
        .unwrap_or(usize::MAX)
}

fn main() {
    let dist = PathBuf::from(std::env::args().nth(1).expect("the dist folder"));
    let event_loop = EventLoopBuilder::<Msg>::with_user_event().build();
    let proxy = event_loop.create_proxy();
    let mut context = WebContext::new(Some(std::env::temp_dir().join("webview-startup-spike")));

    // The plan: three cold starts, each after the browser process exited;
    // then one window that stays, and three warm starts beside it.
    let plan = proxy.clone();
    std::thread::spawn(move || {
        let wait = |seconds| std::thread::sleep(Duration::from_secs(seconds));
        wait(2);
        for _ in 0..3 {
            let _ = plan.send_event(Msg::Go(Kind::Cold));
            wait(15);
        }
        let _ = plan.send_event(Msg::Go(Kind::KeepOpen));
        wait(6);
        for _ in 0..3 {
            let _ = plan.send_event(Msg::Go(Kind::Warm));
            wait(5);
        }
        let _ = plan.send_event(Msg::Done);
    });

    let mut kept: Vec<(Window, WebView)> = Vec::new();
    let mut current: Option<(Kind, Instant, Window, WebView)> = None;

    event_loop.run(move |event, target, flow| {
        *flow = ControlFlow::Wait;
        let Event::UserEvent(message) = event else {
            return;
        };
        match message {
            Msg::Go(kind) => {
                if kind == Kind::Cold {
                    println!("before a cold start: {} WebView2 processes of this one", webview_processes());
                }
                let started = Instant::now();
                let window = WindowBuilder::new()
                    .with_title("WebView2 startup spike")
                    .with_inner_size(LogicalSize::new(420.0, 300.0))
                    .with_position(LogicalPosition::new(40.0, 40.0))
                    .build(target)
                    .expect("window");
                let root = dist.clone();
                let painted = proxy.clone();
                let webview = WebViewBuilder::new_with_web_context(&mut context)
                    .with_additional_browser_args(ARGS)
                    .with_initialization_script(PROBE)
                    .with_custom_protocol("app".into(), move |_, request| {
                        let path = request.uri().path().trim_start_matches('/');
                        let file = root.join(if path.is_empty() { "settings.html" } else { path });
                        match std::fs::read(&file) {
                            Ok(bytes) => Response::builder()
                                .header("Content-Type", content_type(path))
                                .body(Cow::Owned(bytes))
                                .unwrap(),
                            Err(_) => Response::builder().status(404).body(Cow::Borrowed(&[][..])).unwrap(),
                        }
                    })
                    .with_ipc_handler(move |request| {
                        if let Some(at) = request.body().strip_prefix("painted:") {
                            let _ = painted.send_event(Msg::Painted(at.parse().unwrap_or(0.0)));
                        }
                    })
                    .with_url("http://app.localhost/settings.html")
                    .build(&window)
                    .expect("webview");
                let built = started.elapsed();
                println!("{kind:?}: webview made in {} ms", built.as_millis());
                current = Some((kind, started, window, webview));
            }
            Msg::Painted(page_ms) => {
                if let Some((kind, started, window, webview)) = current.take() {
                    println!(
                        "{kind:?}: page painted {} ms after asking ({} ms of them in the page)",
                        started.elapsed().as_millis(),
                        page_ms as u64
                    );
                    if kind == Kind::KeepOpen {
                        kept.push((window, webview));
                    }
                    // Otherwise both are dropped here: the window closes.
                }
            }
            Msg::Done => {
                kept.clear();
                *flow = ControlFlow::Exit;
            }
        }
    });
}
