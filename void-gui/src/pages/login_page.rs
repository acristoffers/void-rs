/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use libadwaita as adw;

use adw::gio::Menu;
use adw::gtk::{Align, Box, Button, Image, Label, MenuButton, Orientation};
use adw::prelude::*;
use adw::{HeaderBar, NavigationPage};

use crate::i18n::gettext;
use crate::window::VoidWindow;

/// Builds the initial login page with the Void logo, title, and Open/Create buttons.
///
/// The page disables the `win.open` action when hidden so it cannot be
/// triggered while another page is active.
pub fn login_page(window: &VoidWindow) -> NavigationPage {
    // ── Header ────────────────────────────────────────────────────────────────
    let settings_section = Menu::new();
    settings_section.append(Some(&gettext("Settings")), Some("win.settings"));

    let help_section = Menu::new();
    help_section.append(Some(&gettext("Keyboard Shortcuts")), Some("win.shortcuts"));
    help_section.append(Some(&gettext("Help")), Some("win.help"));
    help_section.append(Some(&gettext("About Void")), Some("win.about"));

    let menu = Menu::new();
    menu.append_section(None, &settings_section);
    menu.append_section(None, &help_section);

    let menu_button = MenuButton::builder()
        .icon_name("open-menu-symbolic")
        .menu_model(&menu)
        .build();

    let header_bar = HeaderBar::new();
    header_bar.set_show_title(false);
    header_bar.pack_end(&menu_button);

    // ── Body ──────────────────────────────────────────────────────────────────
    let body_icon = Image::new();
    body_icon.set_resource(Some("/icon.png"));
    body_icon.set_pixel_size(128);

    let title = Label::new(Some("Void"));
    title.add_css_class("title-1");

    let subtitle = Label::new(Some(&gettext("Open or create a vault")));
    subtitle.add_css_class("dim-label");

    let open_button = Button::builder().label(&gettext("Open Vault")).build();
    open_button.add_css_class("suggested-action");
    open_button.add_css_class("pill");
    open_button.set_action_name(Some("win.open"));

    let create_button = Button::builder().label(&gettext("Create Vault")).build();
    create_button.add_css_class("suggested-action");
    create_button.add_css_class("pill");
    create_button.set_action_name(Some("win.create"));

    let button_box = Box::new(Orientation::Horizontal, 12);
    button_box.set_halign(Align::Center);
    button_box.append(&open_button);
    button_box.append(&create_button);

    let body = Box::new(Orientation::Vertical, 16);
    body.set_halign(Align::Center);
    body.set_valign(Align::Center);
    body.set_hexpand(true);
    body.set_vexpand(true);
    body.append(&body_icon);
    body.append(&title);
    body.append(&subtitle);
    body.append(&button_box);

    // ── Layout ────────────────────────────────────────────────────────────────
    let content = Box::new(Orientation::Vertical, 0);
    content.append(&header_bar);
    content.append(&body);

    // ── Page ──────────────────────────────────────────────────────────────────
    let page = NavigationPage::builder()
        .tag("login")
        .title("Void")
        .child(&content)
        .build();

    let weak_window = window.downgrade();
    page.connect_showing(move |_| {
        if let Some(window) = weak_window.upgrade() {
            window.set_action_enabled("open", true);
        }
    });
    let weak_window = window.downgrade();
    page.connect_hiding(move |_| {
        if let Some(window) = weak_window.upgrade() {
            window.set_action_enabled("open", false);
        }
    });

    page
}
