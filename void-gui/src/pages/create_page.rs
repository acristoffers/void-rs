/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use libadwaita as adw;

use adw::gtk::{Align, Box, Button, Label, Orientation};
use adw::prelude::*;
use adw::{Clamp, EntryRow, HeaderBar, NavigationPage, PasswordEntryRow, PreferencesGroup};

use void::Store;

use crate::dialogs::error_dialog;
use crate::i18n::gettext;
use crate::window::VoidWindow;

/// Builds the 'Create Vault' page with location, password, and confirm-password fields.
///
/// Validates that the target folder is empty and that passwords match before
/// creating the store and navigating to the main vault browser.
pub fn create_page(window: &VoidWindow) -> NavigationPage {
    let header_bar = HeaderBar::new();

    let path_row = EntryRow::builder().title(&gettext("Location")).build();
    let browse_btn = super::browse_button(&path_row, window);
    path_row.add_suffix(&browse_btn);

    let password_row = PasswordEntryRow::builder()
        .title(&gettext("Password"))
        .build();
    let confirm_row = PasswordEntryRow::builder()
        .title(&gettext("Confirm Password"))
        .build();

    let group = PreferencesGroup::new();
    group.add(&path_row);
    group.add(&password_row);
    group.add(&confirm_row);

    let mismatch_label = Label::new(Some(&gettext("Passwords do not match")));
    mismatch_label.add_css_class("error");
    mismatch_label.set_visible(false);

    let create_button = Button::builder()
        .label(&gettext("Create Vault"))
        .halign(Align::Center)
        .sensitive(false)
        .build();
    create_button.add_css_class("suggested-action");
    create_button.add_css_class("pill");

    {
        let path_row = path_row.clone();
        let password_row = password_row.clone();
        let weak_window = window.downgrade();
        create_button.connect_clicked(move |_| {
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

            {
                let path = std::path::Path::new(&path_str);
                if path.exists() {
                    match path.read_dir() {
                        Ok(mut entries) => {
                            if entries.next().is_some() {
                                error_dialog(
                                    &window,
                                    &gettext("Folder Not Empty"),
                                    &gettext("The selected folder is not empty. Please choose an empty folder or a new location."),
                                );
                                return;
                            }
                        }
                        Err(e) => {
                            error_dialog(
                                &window,
                                &gettext("Cannot Read Folder"),
                                &format!("{}: {e}", gettext("Could not read the selected folder")),
                            );
                            return;
                        }
                    }
                    let _ = std::fs::remove_dir(path);
                }
            }

            match Store::create(path_str, password) {
                Ok(store) => {
                    window.set_store(store);
                    window.nav().pop_to_tag("login");
                    let main = crate::main_view::main_page(&window);
                    window.nav().push(&main);
                }
                Err(e) => {
                    error_dialog(
                        &window,
                        &gettext("Could Not Create Store"),
                        &format!("{}: {e}", gettext("Failed to create the store")),
                    );
                }
            }
        });
    }

    {
        let confirm_row = confirm_row.clone();
        password_row.connect_entry_activated(move |_| {
            confirm_row.grab_focus();
        });
    }

    {
        let create_button = create_button.clone();
        confirm_row.connect_entry_activated(move |_| {
            create_button.emit_clicked();
        });
    }

    let validate = {
        let password_row = password_row.clone();
        let confirm_row = confirm_row.clone();
        let create_button = create_button.clone();
        let mismatch_label = mismatch_label.clone();
        move || {
            let pw = password_row.text();
            let cf = confirm_row.text();
            let matches = !pw.is_empty() && pw == cf;
            let show_mismatch = !pw.is_empty() && !cf.is_empty() && pw != cf;
            create_button.set_sensitive(matches);
            mismatch_label.set_visible(show_mismatch);
        }
    };
    password_row.connect_changed({
        let validate = validate.clone();
        move |_| validate()
    });
    confirm_row.connect_changed({
        let validate = validate.clone();
        move |_| validate()
    });

    let form = Box::new(Orientation::Vertical, 24);
    form.set_vexpand(true);
    form.set_valign(Align::Center);
    form.set_margin_top(24);
    form.set_margin_bottom(24);
    form.append(&group);
    form.append(&mismatch_label);
    form.append(&create_button);

    let clamp = Clamp::builder().maximum_size(400).child(&form).build();
    clamp.set_hexpand(true);
    clamp.set_vexpand(true);
    clamp.set_margin_start(12);
    clamp.set_margin_end(12);

    let content = Box::new(Orientation::Vertical, 0);
    content.append(&header_bar);
    content.append(&clamp);

    let page = NavigationPage::builder()
        .tag("create")
        .title(&gettext("Create Vault"))
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
