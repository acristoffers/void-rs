/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

mod application;
mod dialogs;
mod file_viewer;
mod i18n;
mod main_view;
mod pages;
mod settings;
mod thumbnails;
mod window;

use libadwaita as adw;

use adw::gio::resources_register_include;
use adw::prelude::*;

/// Application entry point: initializes i18n, registers GResource assets, and starts the GTK event loop.
fn main() {
    i18n::init();
    resources_register_include!("void.gresource").expect("Failed to register resources.");
    application::VoidApplication::new().run();
}
