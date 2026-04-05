/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

pub use gettextrs::gettext;

/// Initializes gettext localisation: sets the locale, binds the `void` text domain
/// to the locale directory (compile-time `LOCALEDIR`, dev fallback, or `/usr/share/locale`).
pub fn init() {
    gettextrs::setlocale(gettextrs::LocaleCategory::LcAll, "");

    let localedir = option_env!("LOCALEDIR")
        .map(String::from)
        .unwrap_or_else(|| {
            // During development, look for locale/ next to po/ in the source tree
            let dev_path = concat!(env!("CARGO_MANIFEST_DIR"), "/po/locale");
            if std::path::Path::new(dev_path).exists() {
                dev_path.to_string()
            } else {
                "/usr/share/locale".to_string()
            }
        });

    gettextrs::bindtextdomain("void", &localedir).ok();
    gettextrs::textdomain("void").ok();
}
