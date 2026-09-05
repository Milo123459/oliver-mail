mod app;
mod ui;
mod models;

use app::MailApp;
use gpui_platform::application;

#[tokio::main]
async fn main() {
    application().run(|cx| {
        MailApp::open(cx);
    });
}