/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

mod create_page;
mod login_page;
mod open_page;

use libadwaita as adw;

use adw::gio::Cancellable;
use adw::gtk::{Align, Button};
use adw::prelude::*;
use adw::EntryRow;

use crate::i18n::gettext;
use crate::window::VoidWindow;

pub use create_page::create_page;
pub use login_page::login_page;
pub use open_page::open_page;

/// Creates a folder-picker button that, when clicked, opens a file dialog
/// and writes the selected path into `path_row`.
fn browse_button(path_row: &EntryRow, window: &VoidWindow) -> Button {
    let button = Button::builder()
        .icon_name("folder-open-symbolic")
        .valign(Align::Center)
        .build();

    let path_row = path_row.clone();
    let weak_window = window.downgrade();
    button.connect_clicked(move |_| {
        let Some(window) = weak_window.upgrade() else {
            return;
        };
        let dialog = adw::gtk::FileDialog::new();
        dialog.set_title(&gettext("Select Vault Location"));
        let path_row = path_row.clone();
        dialog.select_folder(Some(&window), None::<&Cancellable>, move |result| {
            if let Ok(file) = result {
                if let Some(path) = file.path() {
                    path_row.set_text(&path.to_string_lossy());
                }
            }
        });
    });

    button
}

/// Shows a folder picker; on success writes the path into `path_row`, on cancel pops the nav.
fn show_folder_picker(window: &VoidWindow, path_row: &EntryRow) {
    let dialog = adw::gtk::FileDialog::new();
    dialog.set_title(&gettext("Select Vault Location"));
    let path_row = path_row.clone();
    let weak_window = window.downgrade();
    dialog.select_folder(
        Some(window),
        None::<&Cancellable>,
        move |result| match result {
            Ok(file) => {
                if let Some(path) = file.path() {
                    path_row.set_text(&path.to_string_lossy());
                }
            }
            Err(_) => {
                if let Some(window) = weak_window.upgrade() {
                    window.nav().pop();
                }
            }
        },
    );
}
