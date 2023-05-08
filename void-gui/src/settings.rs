/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use adw::gio::SettingsBindFlags;
use adw::gtk::{Box, Label, Orientation, Switch};
use libadwaita as adw;

use adw::gdk::gio::Settings;
use adw::{prelude::*, StyleManager};
use adw::{PreferencesGroup, PreferencesPage, PreferencesRow, PreferencesWindow};

pub fn settings_window() -> PreferencesWindow {
    let settings = Settings::new("me.acristoffers.void");

    let system_theme_switch = Switch::new();
    let system_theme_label = Label::new(Some("Use System Theme"));
    system_theme_label.set_hexpand(true);
    system_theme_label.set_halign(adw::gtk::Align::Start);

    let system_theme_box = Box::new(Orientation::Horizontal, 12);
    system_theme_box.set_margin_start(12);
    system_theme_box.set_margin_end(12);
    system_theme_box.set_margin_top(12);
    system_theme_box.set_margin_bottom(12);
    system_theme_box.append(&system_theme_label);
    system_theme_box.append(&system_theme_switch);

    let system_theme = PreferencesRow::builder()
        .title("System Theme")
        .activatable(true)
        .name("System Theme")
        .child(&system_theme_box)
        .build();

    let dark_theme_switch = Switch::new();
    let dark_theme_label = Label::new(Some("Use Dark Theme"));
    dark_theme_label.set_hexpand(true);
    dark_theme_label.set_halign(adw::gtk::Align::Start);

    let dark_theme_box = Box::new(Orientation::Horizontal, 12);
    dark_theme_box.set_margin_start(12);
    dark_theme_box.set_margin_end(12);
    dark_theme_box.set_margin_top(12);
    dark_theme_box.set_margin_bottom(12);
    dark_theme_box.append(&dark_theme_label);
    dark_theme_box.append(&dark_theme_switch);

    let dark_theme = PreferencesRow::builder()
        .title("Dark Theme")
        .activatable(true)
        .name("Dark Theme")
        .child(&dark_theme_box)
        .build();

    settings
        .bind("use-system", &system_theme_switch, "active")
        .flags(SettingsBindFlags::DEFAULT)
        .build();

    settings
        .bind("dark-theme", &dark_theme_switch, "active")
        .flags(SettingsBindFlags::DEFAULT)
        .build();

    settings.connect_changed(Some("use-system"), |s, _| {
        let use_system = s.boolean("use-system");
        let dark_theme = s.boolean("dark-theme");
        let style_manager = StyleManager::default();
        if use_system {
            style_manager.set_color_scheme(adw::ColorScheme::Default);
        } else {
            if dark_theme {
                style_manager.set_color_scheme(adw::ColorScheme::ForceDark);
            } else {
                style_manager.set_color_scheme(adw::ColorScheme::ForceLight);
            }
        }
    });

    settings.connect_changed(Some("dark-theme"), |s, _| {
        let use_system = s.boolean("use-system");
        let dark_theme = s.boolean("dark-theme");
        let style_manager = StyleManager::default();
        if use_system {
            style_manager.set_color_scheme(adw::ColorScheme::Default);
        } else {
            if dark_theme {
                style_manager.set_color_scheme(adw::ColorScheme::ForceDark);
            } else {
                style_manager.set_color_scheme(adw::ColorScheme::ForceLight);
            }
        }
    });

    let group = PreferencesGroup::new();
    group.set_title("Application Theme");
    group.add(&system_theme);
    group.add(&dark_theme);

    let page = PreferencesPage::new();
    page.set_title("Settings");
    page.set_icon_name(Some("preferences-system-symbolic"));
    page.add(&group);

    let window = PreferencesWindow::new();
    window.set_title(Some("Settings"));
    window.set_icon_name(Some("settings-symbolic"));
    window.add(&page);
    window.set_visible_page(&page);

    return window;
}
