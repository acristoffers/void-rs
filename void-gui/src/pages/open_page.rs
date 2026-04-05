/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use libadwaita as adw;

use adw::gtk::{Align, Box, Button, Orientation};
use adw::prelude::*;
use adw::{Clamp, EntryRow, HeaderBar, NavigationPage, PasswordEntryRow, PreferencesGroup};

use void::Store;

use crate::dialogs::error_dialog;
use crate::i18n::gettext;
use crate::window::VoidWindow;

/// Builds the 'Open Vault' page with location and password fields.
///
/// On confirmation, validates the path, opens the store, runs garbage
/// collection, and navigates to the main vault browser.
pub fn open_page(window: &VoidWindow) -> NavigationPage {
    let header_bar = HeaderBar::new();

    let path_row = EntryRow::builder().title(&gettext("Location")).build();
    let browse_btn = super::browse_button(&path_row, window);
    path_row.add_suffix(&browse_btn);

    let password_row = PasswordEntryRow::builder()
        .title(&gettext("Password"))
        .build();

    let group = PreferencesGroup::new();
    group.add(&path_row);
    group.add(&password_row);

    let confirm_button = Button::builder()
        .label(&gettext("Open Vault"))
        .halign(Align::Center)
        .build();
    confirm_button.add_css_class("suggested-action");
    confirm_button.add_css_class("pill");

    {
        let path_row = path_row.clone();
        let password_row = password_row.clone();
        let weak_window = window.downgrade();
        confirm_button.connect_clicked(move |_| {
            let Some(window) = weak_window.upgrade() else {
                return;
            };
            let path_str = path_row.text().to_string();
            let password = password_row.text().to_string();

            if path_str.is_empty() {
                error_dialog(
                    &window,
                    &gettext("No Location Selected"),
                    &gettext("Please select a vault location first."),
                );
                return;
            }

            if !std::path::Path::new(&path_str).join("Store.void").exists() {
                error_dialog(
                    &window,
                    &gettext("Not a Void Store"),
                    &gettext("The selected folder does not contain a Store.void file."),
                );
                return;
            }

            match Store::open(path_str, password) {
                Ok(store) => {
                    let _ = store.gc();
                    window.set_store(store);
                    window.nav().pop_to_tag("login");
                    let main = crate::main_view::main_page(&window);
                    window.nav().push(&main);
                }
                Err(void::Error::CannotDecryptFileError) => {
                    error_dialog(
                        &window,
                        &gettext("Wrong Password"),
                        &gettext("The password is incorrect."),
                    );
                }
                Err(e) => {
                    error_dialog(
                        &window,
                        &gettext("Could Not Open Store"),
                        &format!("{}: {e}", gettext("Failed to open the store")),
                    );
                }
            }
        });
    }

    {
        let confirm_button = confirm_button.clone();
        password_row.connect_entry_activated(move |_| {
            confirm_button.emit_clicked();
        });
    }

    let form = Box::new(Orientation::Vertical, 24);
    form.set_vexpand(true);
    form.set_valign(Align::Center);
    form.set_margin_top(24);
    form.set_margin_bottom(24);
    form.append(&group);
    form.append(&confirm_button);

    let clamp = Clamp::builder().maximum_size(400).child(&form).build();
    clamp.set_hexpand(true);
    clamp.set_vexpand(true);
    clamp.set_margin_start(12);
    clamp.set_margin_end(12);

    let content = Box::new(Orientation::Vertical, 0);
    content.append(&header_bar);
    content.append(&clamp);

    let page = NavigationPage::builder()
        .tag("open")
        .title(&gettext("Open Vault"))
        .child(&content)
        .build();

    let weak_window = window.downgrade();
    page.connect_shown(move |_| {
        if let Some(window) = weak_window.upgrade() {
            super::show_folder_picker(&window, &path_row);
        }
    });

    page
}
