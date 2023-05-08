/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

mod login;
mod settings;

use libadwaita as adw;

use adw::gio::resources_register_include;
use adw::prelude::*;
use adw::Application;

fn main() {
    resources_register_include!("void.gresource").expect("Failed to register resources.");

    let application = Application::builder()
        .application_id("me.acristoffers.void")
        .build();

    application.connect_activate(|app| {
        let login_window = login::login_window(app);
        login_window.show();
    });

    application.run();
}
