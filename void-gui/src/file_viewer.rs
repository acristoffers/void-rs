/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::io::Write;

use libadwaita as adw;

use adw::gio;
use adw::glib;
use adw::gtk::{self, gdk, Button, ScrolledWindow, TextBuffer, TextView, WrapMode};
use adw::prelude::*;

use crate::i18n::gettext;
use crate::window::VoidWindow;

/// Dispatches the given vault file to the appropriate viewer based on its MIME type.
///
/// Images open in a standalone picture window, text files in an editable window
/// with save support, and everything else prompts to decrypt and open with the
/// OS default application.
pub fn open_file(window: &VoidWindow, path: &str) {
    let name = path.rsplit('/').next().unwrap_or(path).to_string();

    let mimetype = {
        let store_ref = window.store();
        store_ref
            .as_ref()
            .and_then(|s| s.metadata_get(path, "mimetype").ok())
            .unwrap_or_default()
    };

    if mimetype.starts_with("image/") {
        let bytes = {
            let store_ref = window.store();
            match store_ref.as_ref().and_then(|s| s.get_bytes(path).ok()) {
                Some(b) => b,
                None => return,
            }
        };
        open_image_viewer(&name, bytes);
    } else if mimetype.starts_with("text/") {
        let bytes = {
            let store_ref = window.store();
            match store_ref.as_ref().and_then(|s| s.get_bytes(path).ok()) {
                Some(b) => b,
                None => return,
            }
        };
        open_text_editor(window, path, &name, bytes);
    } else {
        open_with_os_dialog(window, path, &name);
    }
}

// ── Image viewer (standalone window) ─────────────────────────────────────────

/// Opens a standalone window displaying the image decoded from `bytes`.
fn open_image_viewer(name: &str, bytes: Vec<u8>) {
    let glib_bytes = glib::Bytes::from(&bytes);
    let texture = match gdk::Texture::from_bytes(&glib_bytes) {
        Ok(t) => t,
        Err(_) => return,
    };

    let picture = gtk::Picture::for_paintable(&texture);
    picture.set_content_fit(gtk::ContentFit::Contain);
    picture.set_can_shrink(true);
    picture.set_hexpand(true);
    picture.set_vexpand(true);

    let scroll = ScrolledWindow::builder()
        .child(&picture)
        .hexpand(true)
        .vexpand(true)
        .build();

    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&adw::HeaderBar::new());
    toolbar.set_content(Some(&scroll));

    // Use the running application so the window is registered with it
    let app = gtk::gio::Application::default()
        .and_downcast::<adw::Application>()
        .expect("default application is adw::Application");

    let win = adw::ApplicationWindow::builder()
        .application(&app)
        .title(name)
        .default_width(900)
        .default_height(700)
        .content(&toolbar)
        .build();

    win.present();
}

// ── Text editor ───────────────────────────────────────────────────────────────

/// Opens a text editor window for a vault file, with save-back-to-store support
/// and an unsaved-changes guard on close.
fn open_text_editor(window: &VoidWindow, store_path: &str, name: &str, bytes: Vec<u8>) {
    let text = String::from_utf8_lossy(&bytes).into_owned();

    let buffer = TextBuffer::new(None);
    buffer.set_text(&text);

    let text_view = TextView::new();
    text_view.set_buffer(Some(&buffer));
    text_view.set_wrap_mode(WrapMode::WordChar);
    text_view.set_monospace(true);
    text_view.set_top_margin(8);
    text_view.set_bottom_margin(8);
    text_view.set_left_margin(12);
    text_view.set_right_margin(12);
    text_view.set_hexpand(true);
    text_view.set_vexpand(true);

    let scroll = ScrolledWindow::builder()
        .child(&text_view)
        .hexpand(true)
        .vexpand(true)
        .build();

    let save_btn = Button::with_label(&gettext("Save"));
    save_btn.add_css_class("suggested-action");
    save_btn.set_sensitive(false); // clean until the user edits something

    let header = adw::HeaderBar::new();
    header.pack_end(&save_btn);

    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&header);
    toolbar.set_content(Some(&scroll));

    let app = gtk::gio::Application::default()
        .and_downcast::<adw::Application>()
        .expect("default application is adw::Application");

    let win = adw::ApplicationWindow::builder()
        .application(&app)
        .title(name)
        .default_width(900)
        .default_height(700)
        .content(&toolbar)
        .build();

    // Shared save logic — returns true on success.
    let do_save: std::rc::Rc<dyn Fn() -> bool> = {
        let store_path = store_path.to_string();
        let weak_win_main = window.downgrade();
        let buffer = buffer.clone();
        let save_btn = save_btn.clone();
        std::rc::Rc::new(move || {
            let Some(w) = weak_win_main.upgrade() else {
                return false;
            };
            let start = buffer.start_iter();
            let end = buffer.end_iter();
            let new_text = buffer.text(&start, &end, false);
            let new_bytes = new_text.as_bytes().to_vec();

            // Write the buffer text to a temporary file for re-import.
            let tmp_path = std::env::temp_dir().join("void-text-save.tmp");
            {
                let mut f = match std::fs::File::create(&tmp_path) {
                    Ok(f) => f,
                    Err(_) => {
                        show_error(&w, &gettext("Cannot write temporary file."));
                        return false;
                    }
                };
                if f.write_all(&new_bytes).is_err() {
                    show_error(&w, &gettext("Cannot write temporary file."));
                    return false;
                }
            }

            let tmp_str = tmp_path.to_string_lossy().into_owned();

            let mut store_ref = w.store_mut();
            let ok = if let Some(store) = store_ref.as_mut() {
                // Snapshot existing metadata before removal.
                let metadata = store.metadata_list(&store_path).unwrap_or_default();
                // Remove the old entry and re-add from the temp file.
                let _ = store.remove(&store_path);
                if store.add(&tmp_str, &store_path).is_err() {
                    drop(store_ref);
                    let _ = std::fs::remove_file(&tmp_path);
                    show_error(&w, &gettext("Cannot save file."));
                    return false;
                }
                // Restore the original metadata (e.g. MIME type) on the new entry.
                for (k, v) in &metadata {
                    let _ = store.metadata_set_nosave(&store_path, k, v);
                }
                let _ = store.save();
                true
            } else {
                false
            };
            drop(store_ref);
            // Clean up the temporary file.
            let _ = std::fs::remove_file(&tmp_path);

            if ok {
                save_btn.set_sensitive(false);
                save_btn.set_label(&gettext("Saved"));
            }
            ok
        })
    };

    // Save button
    {
        let do_save = do_save.clone();
        save_btn.connect_clicked(move |_| {
            do_save();
        });
    }

    // Mark dirty on any edit
    {
        let save_btn = save_btn.clone();
        buffer.connect_changed(move |_| {
            save_btn.set_sensitive(true);
            save_btn.set_label(&gettext("Save"));
        });
    }

    // Unsaved-changes guard on close
    {
        let save_btn = save_btn.clone();
        win.connect_close_request(move |win| {
            if !save_btn.is_sensitive() {
                return glib::Propagation::Proceed;
            }

            let dialog = adw::AlertDialog::builder()
                .heading(gettext("Unsaved Changes"))
                .body(gettext("Do you want to save your changes before closing?"))
                .build();
            dialog.add_response("discard", &gettext("Discard"));
            dialog.add_response("cancel", &gettext("Cancel"));
            dialog.add_response("save", &gettext("Save"));
            dialog.set_default_response(Some("save"));
            dialog.set_response_appearance("discard", adw::ResponseAppearance::Destructive);
            dialog.set_response_appearance("save", adw::ResponseAppearance::Suggested);
            dialog.set_close_response("cancel");

            let win_weak = win.downgrade();
            let do_save = do_save.clone();
            dialog.choose(Some(win), None::<&gio::Cancellable>, move |resp| {
                match resp.as_str() {
                    "save" => {
                        if do_save() {
                            if let Some(w) = win_weak.upgrade() {
                                w.destroy();
                            }
                        }
                        // If save failed the error dialog is already shown; leave window open.
                    }
                    "discard" => {
                        if let Some(w) = win_weak.upgrade() {
                            w.destroy();
                        }
                    }
                    _ => {} // "cancel" — do nothing, window stays open
                }
            });

            glib::Propagation::Stop // prevent immediate close; dialog decides
        });
    }

    win.present();
}

// ── Open with OS ──────────────────────────────────────────────────────────────

/// Prompts the user to confirm decrypting `store_path` to a temporary file
/// and opening it with the OS default application.
fn open_with_os_dialog(window: &VoidWindow, store_path: &str, name: &str) {
    let dialog = adw::AlertDialog::builder()
        .heading(gettext("Open with default application?"))
        .body(gettext(
            "This file type cannot be previewed inside Void. \
             Decrypt to a temporary file and open with the default application?",
        ))
        .build();
    dialog.add_response("cancel", &gettext("Cancel"));
    dialog.add_response("open", &gettext("Open"));
    dialog.set_default_response(Some("open"));
    dialog.set_response_appearance("open", adw::ResponseAppearance::Suggested);
    dialog.set_close_response("cancel");

    let store_path = store_path.to_string();
    let name = name.to_string();
    let weak_win = window.downgrade();

    dialog.choose(Some(window), None::<&gio::Cancellable>, move |resp| {
        if resp != "open" {
            return;
        }
        let Some(w) = weak_win.upgrade() else { return };

        // Prepare a temporary directory and file path for decryption output.
        let tmp_dir = std::env::temp_dir().join("void-open");
        let _ = std::fs::create_dir_all(&tmp_dir);
        let tmp_path = tmp_dir.join(&name);
        // Remove existing file so store.get() doesn't fail with FileAlreadyExistsError
        let _ = std::fs::remove_file(&tmp_path);
        let tmp_str = tmp_path.to_string_lossy().into_owned();

        // Decrypt the vault entry to the temporary file.
        let store_ref = w.store();
        if let Some(store) = store_ref.as_ref() {
            if store.get(&store_path, &tmp_str).is_err() {
                drop(store_ref);
                show_error(&w, &gettext("Cannot decrypt file."));
                return;
            }
        }
        drop(store_ref);

        // Launch the decrypted file with the OS default application via the
        // portal, which passes a file descriptor so that sandboxed apps can
        // open files from our private /tmp.
        let file = gio::File::for_path(&tmp_str);
        let launcher = gtk::FileLauncher::new(Some(&file));
        launcher.launch(Some(&w), None::<&gio::Cancellable>, move |result| {
            if result.is_err() {
                // Cannot show_error here easily since we moved w, but the
                // portal itself shows an error dialog on failure.
            }
        });
    });
}

// ── Error helper ──────────────────────────────────────────────────────────────

/// Presents a simple error alert dialog on `window`.
fn show_error(window: &VoidWindow, msg: &str) {
    let d = adw::AlertDialog::builder()
        .heading(gettext("Error"))
        .body(msg)
        .build();
    d.add_response("ok", &gettext("OK"));
    d.set_close_response("ok");
    d.present(Some(window));
}

// ── Workaround: keep stream alive by copying bytes into a Box ─────────────────

// gtk::MediaFile::for_input_stream borrows the stream but does not keep a
// strong reference to it in all bindings versions, so we use the
// connect_prepared_notify trick above to close over the stream.
