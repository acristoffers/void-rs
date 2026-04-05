/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

use libadwaita as adw;

use adw::gio::{self, SimpleAction};
use adw::glib;
use adw::gtk::{self, Box, Entry, Label, ProgressBar, Spinner};
use adw::prelude::*;

use super::utils::{format_size, ClipboardState};
use crate::i18n::gettext;
use crate::window::VoidWindow;

/// Registers window actions for file-level operations (change password, import, create folder, export folder, paste).
///
/// Import operations run on a background thread with progress reporting.
/// Paste handles both copy (decrypt → re-encrypt) and cut (in-store move)
/// semantics based on the clipboard state.
pub(crate) fn setup(
    window: &VoidWindow,
    current_path: &Rc<RefCell<String>>,
    right_clicked_path: &Rc<RefCell<Option<String>>>,
    refresh: &Rc<dyn Fn()>,
    busy_box: &Box,
    busy_spinner: &Spinner,
    import_progress: &ProgressBar,
    import_label: &Label,
    clipboard: &Rc<RefCell<ClipboardState>>,
) {
    // ── Change password action ───────────────────────────────────────────────
    let change_password_action = SimpleAction::new("change-password", None);
    window.add_action(&change_password_action);
    {
        let weak_window = window.downgrade();
        change_password_action.connect_activate(move |_, _| {
            let Some(w) = weak_window.upgrade() else {
                return;
            };

            let dialog = adw::AlertDialog::builder()
                .heading(gettext("Change Password"))
                .build();
            dialog.add_response("cancel", &gettext("Cancel"));
            dialog.add_response("change", &gettext("Change"));
            dialog.set_default_response(Some("change"));
            dialog.set_response_appearance("change", adw::ResponseAppearance::Destructive);
            dialog.set_close_response("cancel");

            let vbox = gtk::Box::new(gtk::Orientation::Vertical, 8);
            let new_pw = Entry::builder()
                .placeholder_text(gettext("New password"))
                .input_purpose(gtk::InputPurpose::Password)
                .visibility(false)
                .build();
            let confirm_pw = Entry::builder()
                .placeholder_text(gettext("Confirm new password"))
                .input_purpose(gtk::InputPurpose::Password)
                .visibility(false)
                .build();
            vbox.append(&new_pw);
            vbox.append(&confirm_pw);
            dialog.set_extra_child(Some(&vbox));

            let weak_win2 = w.downgrade();
            dialog.choose(Some(&w), None::<&gio::Cancellable>, move |resp| {
                if resp != "change" {
                    return;
                }
                let p1 = new_pw.text().to_string();
                let p2 = confirm_pw.text().to_string();
                if p1.is_empty() {
                    return;
                }
                if p1 != p2 {
                    let Some(w) = weak_win2.upgrade() else { return };
                    let err = adw::AlertDialog::builder()
                        .heading(gettext("Passwords do not match"))
                        .body(gettext("The two passwords you entered are different."))
                        .build();
                    err.add_response("ok", &gettext("OK"));
                    err.set_close_response("ok");
                    err.present(Some(&w));
                    return;
                }
                let Some(w) = weak_win2.upgrade() else { return };
                let mut store_ref = w.store_mut();
                let ok = store_ref
                    .as_mut()
                    .map_or(false, |s| s.change_password(&p1).is_ok());
                drop(store_ref);
                if !ok {
                    let err = adw::AlertDialog::builder()
                        .heading(gettext("Error"))
                        .body(gettext("Could not change the password."))
                        .build();
                    err.add_response("ok", &gettext("OK"));
                    err.set_close_response("ok");
                    err.present(Some(&w));
                }
            });
        });
    }

    // ── Import file action ──────────────────────────────────────────────────
    // Import pattern: take the Store out of the window (so it can be sent to
    // a background thread), spawn the heavy I/O work there, then poll every
    // 50 ms with `glib::timeout_add_local` on the UI thread. When the channel
    // delivers the finished Store, put it back and refresh the view.
    let import_file_action = SimpleAction::new("import-file", None);
    window.add_action(&import_file_action);
    {
        let weak_window = window.downgrade();
        let current_path = current_path.clone();
        let refresh = refresh.clone();
        let busy_box = busy_box.clone();
        let busy_spinner = busy_spinner.clone();
        let import_progress = import_progress.clone();
        let import_label = import_label.clone();
        import_file_action.connect_activate(move |_, _| {
            let Some(window) = weak_window.upgrade() else {
                return;
            };
            let dialog = gtk::FileDialog::new();
            dialog.set_title(&gettext("Import File"));
            let current_path = current_path.clone();
            let refresh = refresh.clone();
            let busy_box = busy_box.clone();
            let busy_spinner = busy_spinner.clone();
            let import_progress = import_progress.clone();
            let import_label = import_label.clone();
            let weak_window2 = window.downgrade();
            dialog.open(Some(&window), None::<&gio::Cancellable>, move |result| {
                if let Ok(file) = result {
                    if let Some(path) = file.path() {
                        let Some(window) = weak_window2.upgrade() else {
                            return;
                        };
                        let cwd = current_path.borrow().clone();
                        // Take ownership of the Store so it can cross thread boundaries.
                        let store = { window.store_mut().take() };
                        if let Some(mut store) = store {
                            let total_bytes =
                                std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                            let bytes_done = Arc::new(AtomicU64::new(0));
                            let bytes_done_th = bytes_done.clone();

                            busy_box.set_visible(true);
                            busy_spinner.set_spinning(true);
                            import_progress.set_visible(true);
                            import_progress.set_fraction(0.0);
                            import_label.set_visible(true);
                            import_label.set_text(&gettext("Importing…"));

                            let busy_box = busy_box.clone();
                            let busy_spinner = busy_spinner.clone();
                            let import_progress = import_progress.clone();
                            let import_label = import_label.clone();
                            let weak = window.downgrade();
                            let file_path = path.to_string_lossy().to_string();
                            let refresh = refresh.clone();
                            let (tx, rx) = std::sync::mpsc::channel();
                            std::thread::spawn(move || {
                                let _ = store.add_with_progress(&file_path, &cwd, bytes_done_th);
                                let _ = tx.send(store);
                            });
                            glib::timeout_add_local(
                                std::time::Duration::from_millis(50),
                                move || match rx.try_recv() {
                                    Ok(store) => {
                                        if let Some(w) = weak.upgrade() {
                                            *w.store_mut() = Some(store);
                                        }
                                        busy_box.set_visible(false);
                                        busy_spinner.set_spinning(false);
                                        import_progress.set_visible(false);
                                        import_label.set_visible(false);
                                        refresh();
                                        glib::ControlFlow::Break
                                    }
                                    Err(_) => {
                                        let done = bytes_done.load(Ordering::Relaxed);
                                        if total_bytes > 0 {
                                            import_progress
                                                .set_fraction(done as f64 / total_bytes as f64);
                                            import_label.set_text(&format!(
                                                "{} / {}",
                                                format_size(done),
                                                format_size(total_bytes)
                                            ));
                                        } else {
                                            import_progress.pulse();
                                        }
                                        glib::ControlFlow::Continue
                                    }
                                },
                            );
                        }
                    }
                }
            });
        });
    }

    // ── Import folder action ────────────────────────────────────────────────
    // Same take-Store / spawn / poll pattern as import-file above, but walks
    // the directory tree first to compute a total byte count for progress.
    let import_folder_action = SimpleAction::new("import-folder", None);
    window.add_action(&import_folder_action);
    {
        let weak_window = window.downgrade();
        let current_path = current_path.clone();
        let refresh = refresh.clone();
        let busy_box = busy_box.clone();
        let busy_spinner = busy_spinner.clone();
        let import_progress = import_progress.clone();
        let import_label = import_label.clone();
        import_folder_action.connect_activate(move |_, _| {
            let Some(window) = weak_window.upgrade() else {
                return;
            };
            let dialog = gtk::FileDialog::new();
            dialog.set_title(&gettext("Import Folder"));
            let current_path = current_path.clone();
            let refresh = refresh.clone();
            let busy_box = busy_box.clone();
            let busy_spinner = busy_spinner.clone();
            let import_progress = import_progress.clone();
            let import_label = import_label.clone();
            let weak_window2 = window.downgrade();
            dialog.select_folder(Some(&window), None::<&gio::Cancellable>, move |result| {
                if let Ok(file) = result {
                    if let Some(path) = file.path() {
                        let Some(window) = weak_window2.upgrade() else {
                            return;
                        };
                        let cwd = current_path.borrow().clone();
                        let store = { window.store_mut().take() };
                        if let Some(mut store) = store {
                            let bytes_done = Arc::new(AtomicU64::new(0));
                            let bytes_total = Arc::new(AtomicU64::new(0));
                            let bytes_done_th = bytes_done.clone();
                            let bytes_total_th = bytes_total.clone();

                            busy_box.set_visible(true);
                            busy_spinner.set_spinning(true);
                            import_progress.set_visible(true);
                            import_progress.set_fraction(0.0);
                            import_label.set_visible(true);
                            import_label.set_text(&gettext("Importing…"));

                            let busy_box = busy_box.clone();
                            let busy_spinner = busy_spinner.clone();
                            let import_progress = import_progress.clone();
                            let import_label = import_label.clone();
                            let weak = window.downgrade();
                            let folder_path = path.to_string_lossy().to_string();
                            let refresh = refresh.clone();
                            let (tx, rx) = std::sync::mpsc::channel();
                            std::thread::spawn(move || {
                                let total: u64 = walkdir::WalkDir::new(&folder_path)
                                    .follow_links(true)
                                    .into_iter()
                                    .filter_map(Result::ok)
                                    .filter_map(|e| e.metadata().ok())
                                    .filter(|m: &std::fs::Metadata| m.is_file())
                                    .map(|m: std::fs::Metadata| m.len())
                                    .sum();
                                bytes_total_th.store(total, Ordering::Relaxed);
                                let _ = store.add_with_progress(&folder_path, &cwd, bytes_done_th);
                                let _ = tx.send(store);
                            });
                            glib::timeout_add_local(
                                std::time::Duration::from_millis(50),
                                move || match rx.try_recv() {
                                    Ok(store) => {
                                        if let Some(w) = weak.upgrade() {
                                            *w.store_mut() = Some(store);
                                        }
                                        busy_box.set_visible(false);
                                        busy_spinner.set_spinning(false);
                                        import_progress.set_visible(false);
                                        import_label.set_visible(false);
                                        refresh();
                                        glib::ControlFlow::Break
                                    }
                                    Err(_) => {
                                        let total = bytes_total.load(Ordering::Relaxed);
                                        let done = bytes_done.load(Ordering::Relaxed);
                                        if total > 0 {
                                            import_progress
                                                .set_fraction(done as f64 / total as f64);
                                            import_label.set_text(&format!(
                                                "{} / {}",
                                                format_size(done),
                                                format_size(total)
                                            ));
                                        } else {
                                            import_progress.pulse();
                                        }
                                        glib::ControlFlow::Continue
                                    }
                                },
                            );
                        }
                    }
                }
            });
        });
    }

    // ── Create folder action ────────────────────────────────────────────────
    let create_folder_action = SimpleAction::new("create-folder", None);
    window.add_action(&create_folder_action);
    {
        let weak_window = window.downgrade();
        let current_path = current_path.clone();
        let right_clicked_path = right_clicked_path.clone();
        let refresh = refresh.clone();
        create_folder_action.connect_activate(move |_, _| {
            let Some(window) = weak_window.upgrade() else {
                return;
            };
            let target = right_clicked_path
                .borrow_mut()
                .take()
                .unwrap_or_else(|| current_path.borrow().clone());

            let dialog = adw::AlertDialog::builder()
                .heading(gettext("Create Folder"))
                .build();
            dialog.add_response("cancel", &gettext("Cancel"));
            dialog.add_response("create", &gettext("Create"));
            dialog.set_response_appearance("create", adw::ResponseAppearance::Suggested);
            dialog.set_default_response(Some("create"));
            dialog.set_close_response("cancel");

            let entry = Entry::new();
            entry.set_placeholder_text(Some(&gettext("Folder name")));
            dialog.set_extra_child(Some(&entry));

            let weak_window2 = window.downgrade();
            let refresh = refresh.clone();
            dialog.choose(Some(&window), None::<&gio::Cancellable>, move |response| {
                if response != "create" {
                    return;
                }
                let name = entry.text().to_string();
                if name.is_empty() {
                    return;
                }
                let Some(window) = weak_window2.upgrade() else {
                    return;
                };
                let full_path = if target == "/" {
                    format!("/{}", name)
                } else {
                    format!("{}/{}", target, name)
                };
                {
                    let mut store_ref = window.store_mut();
                    if let Some(store) = store_ref.as_mut() {
                        let _ = store.mkdir(&full_path);
                    }
                }
                refresh();
            });
        });
    }

    // ── Export folder action ────────────────────────────────────────────────
    let export_folder_action = SimpleAction::new("export-folder", None);
    window.add_action(&export_folder_action);
    {
        let weak_window = window.downgrade();
        let current_path = current_path.clone();
        let busy_box = busy_box.clone();
        let busy_spinner = busy_spinner.clone();
        export_folder_action.connect_activate(move |_, _| {
            let Some(window) = weak_window.upgrade() else {
                return;
            };
            let dialog = gtk::FileDialog::new();
            dialog.set_title(&gettext("Export Folder"));
            let current_path = current_path.clone();
            let weak_window2 = window.downgrade();
            let busy_box = busy_box.clone();
            let busy_spinner = busy_spinner.clone();
            dialog.select_folder(Some(&window), None::<&gio::Cancellable>, move |result| {
                if let Ok(file) = result {
                    if let Some(dest) = file.path() {
                        let Some(window) = weak_window2.upgrade() else {
                            return;
                        };
                        let cwd = current_path.borrow().clone();
                        let name = if cwd == "/" {
                            "vault".to_string()
                        } else {
                            cwd.rsplit('/').next().unwrap_or("export").to_string()
                        };
                        let dest_path = dest.join(&name).to_string_lossy().to_string();
                        let store = { window.store_mut().take() };
                        if let Some(store) = store {
                            busy_box.set_visible(true);
                            busy_spinner.set_spinning(true);
                            let busy_box = busy_box.clone();
                            let busy_spinner = busy_spinner.clone();
                            let weak = window.downgrade();
                            let (tx, rx) = std::sync::mpsc::channel();
                            std::thread::spawn(move || {
                                let _ = store.get(&cwd, &dest_path);
                                let _ = tx.send(store);
                            });
                            glib::timeout_add_local(
                                std::time::Duration::from_millis(50),
                                move || match rx.try_recv() {
                                    Ok(store) => {
                                        if let Some(w) = weak.upgrade() {
                                            *w.store_mut() = Some(store);
                                        }
                                        busy_box.set_visible(false);
                                        busy_spinner.set_spinning(false);
                                        glib::ControlFlow::Break
                                    }
                                    Err(_) => glib::ControlFlow::Continue,
                                },
                            );
                        }
                    }
                }
            });
        });
    }

    // ── Paste action ────────────────────────────────────────────────────────
    // Cut: performs an in-store `mv` for each path, then clears the clipboard.
    // Copy: decrypt each source to a temp dir, re-encrypt into the target
    // folder on a background thread, then clean up.
    let paste_action = SimpleAction::new("paste", None);
    window.add_action(&paste_action);
    {
        let weak_window = window.downgrade();
        let current_path = current_path.clone();
        let right_clicked_path = right_clicked_path.clone();
        let clipboard = clipboard.clone();
        let refresh = refresh.clone();
        let busy_box = busy_box.clone();
        let busy_spinner = busy_spinner.clone();
        paste_action.connect_activate(move |_, _| {
            let Some(window) = weak_window.upgrade() else {
                return;
            };
            let ClipboardState { paths, is_cut } = clipboard.borrow().clone();
            if paths.is_empty() {
                return;
            }
            let cwd = right_clicked_path
                .borrow_mut()
                .take()
                .unwrap_or_else(|| current_path.borrow().clone());

            if is_cut {
                {
                    let mut store_ref = window.store_mut();
                    let Some(store) = store_ref.as_mut() else {
                        return;
                    };
                    for src in &paths {
                        let name = src.rsplit('/').next().unwrap_or("item");
                        let dst = if cwd == "/" {
                            format!("/{}", name)
                        } else {
                            format!("{}/{}", cwd, name)
                        };
                        if store.mv(src, &dst).is_ok() {
                            let _ = store.save();
                        }
                    }
                }
                *clipboard.borrow_mut() = ClipboardState::default();
                refresh();
            } else {
                let store = { window.store_mut().take() };
                if let Some(mut store) = store {
                    busy_box.set_visible(true);
                    busy_spinner.set_spinning(true);
                    let busy_box = busy_box.clone();
                    let busy_spinner = busy_spinner.clone();
                    let weak = window.downgrade();
                    let refresh = refresh.clone();
                    let (tx, rx) = std::sync::mpsc::channel();
                    std::thread::spawn(move || {
                        for src in &paths {
                            let name = src.rsplit('/').next().unwrap_or("item");
                            let temp_dir = std::env::temp_dir().join("void-gui-paste");
                            let _ = std::fs::create_dir_all(&temp_dir);
                            let temp_path = temp_dir.join(name);
                            if temp_path.is_dir() {
                                let _ = std::fs::remove_dir_all(&temp_path);
                            } else if temp_path.exists() {
                                let _ = std::fs::remove_file(&temp_path);
                            }
                            if store.get(src, &temp_path.to_string_lossy()).is_ok() {
                                let _ = store.add(&temp_path.to_string_lossy(), &cwd);
                            }
                            let _ = std::fs::remove_dir_all(&temp_dir);
                        }
                        let _ = tx.send(store);
                    });
                    glib::timeout_add_local(std::time::Duration::from_millis(50), move || match rx
                        .try_recv()
                    {
                        Ok(store) => {
                            if let Some(w) = weak.upgrade() {
                                *w.store_mut() = Some(store);
                            }
                            busy_box.set_visible(false);
                            busy_spinner.set_spinning(false);
                            refresh();
                            glib::ControlFlow::Break
                        }
                        Err(_) => glib::ControlFlow::Continue,
                    });
                }
            }
        });
    }
}
