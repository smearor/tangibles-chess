use gtk4::prelude::*;

use tangibles_chess_icons::icon_names;

use crate::ui::dual_facing_row::DualFacingRow;

/// Modal game-over notification rendered once per table edge: outcome
/// icon, result line and a confirmation button that leaves the game for
/// the setup dialog (`win.new-game`).
///
/// The backdrop covers the whole surface and swallows pointer input so
/// the covered board cannot be tapped while the result is up. Both
/// facing copies carry the same confirm action, so either player may
/// acknowledge the result.
pub struct GameResult {
    /// Overlay widget for `GameScreen::push_overlay`.
    pub widget: gtk4::Widget,
}

impl GameResult {
    /// Builds the overlay; `icon_name` distinguishes decisive games
    /// (trophy) from draws (handshake). `replay` adds a second button
    /// (`win.replay`) that steps through the recorded game; `quit` is
    /// the tooltip of the application-exit button.
    pub fn new(text: &str, icon_name: &str, confirm: &str, replay: Option<&str>, quit: &str) -> Self {
        let text = text.to_string();
        let icon_name = icon_name.to_string();
        let confirm = confirm.to_string();
        let replay = replay.map(str::to_string);
        let quit = quit.to_string();
        let row = DualFacingRow::new(move |_| Self::build_card(&text, &icon_name, &confirm, replay.as_deref(), &quit));
        let widget = row.widget;
        widget.set_hexpand(true);
        widget.set_vexpand(true);
        widget.add_css_class("result-backdrop");
        widget.add_controller(gtk4::GestureClick::new());
        Self { widget: widget.upcast() }
    }

    /// One facing copy of the result card.
    fn build_card(text: &str, icon_name: &str, confirm: &str, replay: Option<&str>, quit: &str) -> gtk4::Widget {
        let card = gtk4::Box::new(gtk4::Orientation::Vertical, 20);
        card.set_halign(gtk4::Align::Center);
        card.set_valign(gtk4::Align::Center);
        card.set_vexpand(true);
        card.add_css_class("result-card");

        let icon = gtk4::Image::from_icon_name(icon_name);
        icon.set_pixel_size(72);
        card.append(&icon);

        let label = gtk4::Label::new(Some(text));
        label.add_css_class("result-text");
        card.append(&label);

        let buttons = gtk4::Box::new(gtk4::Orientation::Horizontal, 16);
        buttons.set_halign(gtk4::Align::Center);
        if let Some(replay) = replay {
            let button = gtk4::Button::builder().label(replay).build();
            button.add_css_class("pill");
            button.set_action_name(Some("win.replay"));
            button.set_size_request(280, 72);
            buttons.append(&button);
        }
        let button = gtk4::Button::builder().label(confirm).build();
        button.add_css_class("pill");
        button.add_css_class("suggested-action");
        button.set_action_name(Some("win.new-game"));
        button.set_size_request(280, 72);
        buttons.append(&button);
        // Exits the application entirely (same affordance as the setup
        // screen's quit button).
        let quit_icon = gtk4::Image::from_icon_name(icon_names::TCH_X_SYMBOLIC);
        quit_icon.set_pixel_size(32);
        let quit_button = gtk4::Button::new();
        quit_button.set_child(Some(&quit_icon));
        quit_button.add_css_class("pill");
        quit_button.set_tooltip_text(Some(quit));
        quit_button.set_size_request(84, 84);
        quit_button.connect_clicked(|button| {
            if let Some(window) = button.root().and_then(|root| root.downcast::<gtk4::Window>().ok()) {
                window.close();
            }
        });
        buttons.append(&quit_button);
        card.append(&buttons);

        card.upcast()
    }
}
