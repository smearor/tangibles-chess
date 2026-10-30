mod controller;
mod l10n;
mod playable;
mod replayable;
mod rotatable;
mod settings;
mod sound;
mod sound_effect;
mod tangible_host;
mod theme;
mod ui;
mod undoable;

use std::rc::Rc;

use gtk4::prelude::*;
use libadwaita as adw;
use tracing::warn;
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
    /// Expose the game-domain MCP endpoint (Streamable HTTP, e.g.
    /// `127.0.0.1:8091` or `0.0.0.0:8091` for LAN agents).
    #[arg(long)]
    mcp_listen: Option<std::net::SocketAddr>,
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
            warn!("icon set install failed: {err}");
        }
        if let Err(err) = fonts_rs_cuernavaca::Cuernavaca::install() {
            warn!("cuernavaca install failed: {err}");
        }
        if let Some(display) = gdk4::Display::default() {
            if let Err(err) = tangibles_chess_icons::TangiblesChessIcons::install_icon_theme(&display) {
                warn!("icon theme registration failed: {err}");
            }
            if let Err(err) = fonts_rs_cuernavaca::Cuernavaca::install_icon_theme(&display) {
                warn!("cuernavaca icon theme failed: {err}");
            }
        }
        install_css();
        install_shutdown_handlers(app);
        let window = AppWindow::new(app, settings.clone(), l10n.clone(), args.mock, args.mcp_listen);
        window.window.present();
        if std::env::var_os("TANGIBLES_NO_FULLSCREEN").is_none() {
            // Fullscreen must be requested once the window is presented;
            // before that the content's natural size can overshoot the
            // monitor and the compositor clamps it visually.
            window.window.fullscreen();
        }
    });

    // `run()` would feed argv to GTK's own option parser, which rejects
    // our flags; clap already consumed them.
    application.run_with_args(&["tangibles-chess"])
}

/// Releases the daemon field-set registration on SIGINT/SIGTERM and
/// then quits the app, so a Ctrl+C on the desk does not leave a stale
/// `chess` set registered. `signal-hook` feeds the signals into a
/// channel; the actual cleanup runs on the glib main loop where the
/// controller lives.
fn install_shutdown_handlers(app: &adw::Application) {
    let (tx, rx) = async_channel::unbounded::<()>();
    std::thread::spawn(move || {
        let mut signals = match signal_hook::iterator::Signals::new([signal_hook::consts::SIGINT, signal_hook::consts::SIGTERM]) {
            Ok(signals) => signals,
            Err(err) => {
                warn!("signal handler install failed: {err}");
                return;
            }
        };
        if signals.forever().next().is_some() {
            let _ = tx.send_blocking(());
        }
    });
    let app = app.clone();
    glib::MainContext::default().spawn_local(async move {
        if rx.recv().await.is_ok() {
            controller::with_controller(|controller| {
                controller.borrow_mut().release_registrations();
            });
            app.quit();
        }
    });
}

/// Registers the application stylesheet.
fn install_css() {
    let provider = gtk4::CssProvider::new();
    provider.load_from_string(include_str!("../resources/style.css"));
    if let Some(display) = gdk4::Display::default() {
        gtk4::style_context_add_provider_for_display(&display, &provider, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION);
    }
}
