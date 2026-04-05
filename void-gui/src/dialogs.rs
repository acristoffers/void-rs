/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use libadwaita as adw;

use adw::prelude::*;

use crate::i18n::gettext;

/// Presents a modal error dialog with the given `heading` and `body` text and a single OK button.
pub fn error_dialog(window: &impl IsA<adw::gtk::Widget>, heading: &str, body: &str) {
    let dialog = adw::AlertDialog::builder()
        .heading(heading)
        .body(body)
        .build();
    dialog.add_response("ok", &gettext("OK"));
    dialog.present(Some(window));
}
