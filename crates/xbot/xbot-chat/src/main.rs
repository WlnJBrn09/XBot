//! XBot chat window: a system WebView showing the embedded UI, with a small
//! JSON API that talks to `xbotd` on the session D-Bus.

mod api;
mod assets;
#[cfg(test)]
mod design;

use std::borrow::Cow;
use std::sync::Arc;

use tao::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Icon, WindowBuilder},
};
use wry::http::{header, Request, Response};
use wry::WebViewBuilder;

const APP_URL: &str = "xbot://localhost/";

fn window_icon() -> Option<Icon> {
    let image = image::load_from_memory(assets::WINDOW_ICON_PNG)
        .ok()?
        .into_rgba8();
    let (width, height) = image.dimensions();
    Icon::from_rgba(image.into_raw(), width, height).ok()
}

fn response(status: u16, mime: &str, body: Cow<'static, [u8]>) -> Response<Cow<'static, [u8]>> {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, mime)
        .header(header::CACHE_CONTROL, "no-store")
        .body(body)
        .expect("static response headers are valid")
}

fn asset_response(path: &str) -> Response<Cow<'static, [u8]>> {
    match assets::lookup(path) {
        Some(asset) => response(200, asset.mime, Cow::Borrowed(asset.body)),
        None => response(404, "text/plain", Cow::Borrowed(b"not found")),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = Arc::new(
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()?,
    );
    let api = Arc::new(api::Api::new(xbot_core::Config::path()?));

    let event_loop = EventLoop::new();
    let mut builder = WindowBuilder::new()
        .with_title("XBot")
        .with_inner_size(tao::dpi::LogicalSize::new(1080.0, 740.0))
        .with_min_inner_size(tao::dpi::LogicalSize::new(640.0, 480.0));
    if let Some(icon) = window_icon() {
        builder = builder.with_window_icon(Some(icon));
    }
    let window = builder.build(&event_loop)?;

    let protocol_runtime = runtime.clone();
    let webview = WebViewBuilder::new()
        .with_url(APP_URL)
        .with_navigation_handler(|url| url.starts_with(APP_URL))
        .with_asynchronous_custom_protocol(
            "xbot".into(),
            move |_id, request: Request<Vec<u8>>, responder| {
                let path = request.uri().path().to_owned();
                if !path.starts_with("/api/") {
                    responder.respond(asset_response(&path));
                    return;
                }
                let method = request.method().as_str().to_owned();
                let body = request.into_body();
                let api = api.clone();
                protocol_runtime.spawn(async move {
                    let reply = api.handle(&method, &path, &body).await;
                    let json = serde_json::to_vec(&reply.body).unwrap_or_default();
                    responder.respond(response(reply.status, "application/json", Cow::Owned(json)));
                });
            },
        );

    #[cfg(target_os = "linux")]
    let _webview = {
        use tao::platform::unix::WindowExtUnix;
        use wry::WebViewBuilderExtUnix;
        let vbox = window.default_vbox().ok_or("window has no GTK container")?;
        webview.build_gtk(vbox)?
    };
    #[cfg(not(target_os = "linux"))]
    let _webview = webview.build(&window)?;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        if let Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } = event
        {
            *control_flow = ControlFlow::Exit;
        }
    });
}
