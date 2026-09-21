mod app;
mod assets;
mod html_text;
mod models;
mod runtime;
mod storage;
mod ui;

use app::MailApp;
use assets::Assets;
use gpui_platform::application;

fn main() {
    // Start the tokio runtime up front. gpui runs on the main thread with its
    // own executors; network work is sent to tokio via `runtime::spawn`.
    runtime::runtime();

    application().with_assets(Assets).run(|cx| {
        MailApp::open(cx);
    });
}
