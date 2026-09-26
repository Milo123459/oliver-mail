mod google_oauth;
mod temp_mail;
mod theme;

pub use google_oauth::{GoogleAccount, get_gmail_mail, get_gmail_message, login, set_gmail_starred};
pub use temp_mail::{Email, TempEmail, create_account, create_account_with_credentials, get_mail, refresh_token, get_domains};
pub use theme::Theme;
