mod app;
mod ui;

use app::MailApp;
use gpui_platform::application;

fn main() {
    application().run(|cx| {
        MailApp::open(cx);
    });
}