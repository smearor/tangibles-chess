mod controller;
mod l10n;
mod piece_icons;
mod settings;
mod sound;
mod tangible_host;
mod theme;
mod ui;

use std::rc::Rc;

use gtk4::prelude::*;
use libadwaita as adw;
use tracing_subscriber::EnvFilter;
use unic_langid::LanguageIdentifier;

use crate::l10n::L10n;
use crate::settings::AppSettings;
use crate::ui::app_window::AppWindow;

/// Command-line options.
#[derive(Debug, clap::Parser)]
#[command(name = "tangibles-chess", about = "Tabletop chess for the smart desk")]
struct Args {
    /// tangible-objects daemon GraphQL endpoint.
    #[arg(long, default_value = "http://127.0.0.1:8080")]
    daemon_url: String,
    /// Locale override (e.g. `de`, `en`).
    #[arg(long)]
    locale: Option<String>,
    /// Use the mock tangible source instead of the daemon (development).
    #[arg(long)]
    mock: bool,
}

fn main() -> glib::ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    let args = <Args as clap::Parser>::parse();
    let mut settings = AppSettings::load();
    settings.daemon_url = args.daemon_url;
    if let Some(locale) = args.locale.clone() {
        settings.locale = Some(locale);
    }

    let l10n = settings
        .locale
        .as_deref()
        .and_then(|code| code.parse::<LanguageIdentifier>().ok())
        .map_or_else(L10n::system, L10n::for_locale);
    let l10n = Rc::new(l10n);

    let application = adw::Application::builder().application_id("io.github.smearor.tangibles_chess").build();

    application.connect_activate(move |app| {
        if let Err(err) = tangibles_chess_icons::TangiblesChessIcons::install() {
            tracing::warn!("icon set install failed: {err}");
        }
        if let Err(err) = fonts_rs_cuernavaca::Cuernavaca::install() {
            tracing::warn!("cuernavaca install failed: {err}");
        }
        if let Some(display) = gdk4::Display::default() {
            if let Err(err) = tangibles_chess_icons::TangiblesChessIcons::install_icon_theme(&display) {
                tracing::warn!("icon theme registration failed: {err}");
            }
            if let Err(err) = fonts_rs_cuernavaca::Cuernavaca::install_icon_theme(&display) {
                tracing::warn!("cuernavaca icon theme failed: {err}");
            }
        }
        install_css();
        let window = AppWindow::new(app, settings.clone(), l10n.clone(), args.mock);
        window.window.present();
        if std::env::var_os("TANGIBLES_NO_FULLSCREEN").is_none() {
            // Fullscreen must be requested once the window is presented;
            // before that the content's natural size can overshoot the
            // monitor and the compositor clamps it visually.
            window.window.fullscreen();
        }
    });

    application.run()
}

/// Registers the application stylesheet.
fn install_css() {
    let provider = gtk4::CssProvider::new();
    provider.load_from_string(include_str!("../resources/style.css"));
    if let Some(display) = gdk4::Display::default() {
        gtk4::style_context_add_provider_for_display(&display, &provider, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION);
    }
}
