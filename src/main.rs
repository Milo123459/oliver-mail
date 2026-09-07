mod app;
mod models;
mod ui;

use app::MailApp;
use gpui_platform::application;

#[tokio::main]
async fn main() {
    application().run(|cx| {
        MailApp::open(cx);
    });
}
