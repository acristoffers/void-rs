/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::cell::RefCell;
use std::rc::Rc;

use libadwaita as adw;

use adw::gio::{self, SimpleAction};
use adw::glib;
use adw::gtk::{self, Box, Entry, MultiSelection, Spinner};
use adw::prelude::*;

use super::utils::{ClipboardState, StoreEntry};
use crate::file_viewer::open_file;
use crate::i18n::gettext;
use crate::window::VoidWindow;

/// Registers window actions for item-level operations (open, export, copy, cut, rename, delete).
///
/// Each action operates on either the grid selection or the most recently
/// right-clicked path. Destructive operations (rename, delete) present a
/// confirmation dialog before proceeding.
pub(crate) fn setup(
    window: &VoidWindow,
    grid_store: &gio::ListStore,
    grid_selection: &MultiSelection,
    right_clicked_path: &Rc<RefCell<Option<String>>>,
    current_path: &Rc<RefCell<String>>,
    navigate_action: &SimpleAction,
    refresh: &Rc<dyn Fn()>,
    clipboard: &Rc<RefCell<ClipboardState>>,
    busy_box: &Box,
    busy_spinner: &Spinner,
) {
    // ── Item open action ────────────────────────────────────────────────────
    let item_open_action = SimpleAction::new("item-open", None);
    window.add_action(&item_open_action);
    {
        let right_clicked_path = right_clicked_path.clone();
        let navigate_action = navigate_action.clone();
        let grid_store = grid_store.clone();
        let weak_window = window.downgrade();
        item_open_action.connect_activate(move |_, _| {
            let path = right_clicked_path.borrow().clone();
            if let Some(path) = path {
                for i in 0..grid_store.n_items() {
                    if let Some(obj) = grid_store.item(i).and_downcast::<glib::BoxedAnyObject>() {
                        let entry = obj.borrow::<StoreEntry>();
                        let is_file = entry.is_file;
                        let entry_path = entry.path.clone();
                        drop(entry);
                        if entry_path == path {
                            if is_file {
                                if let Some(w) = weak_window.upgrade() {
                                    open_file(&w, &path);
                                }
                            } else {
                                navigate_action.activate(Some(&path.to_variant()));
                            }
                            break;
                        }
                    }
                }
            }
        });
    }

    // ── Collect selected paths helper ───────────────────────────────────────
    // Returns the union of the grid multi-selection and the right-click target,
    // so context-menu actions always include the item the user clicked on.
    let selected_paths = {
        let grid_store = grid_store.clone();
        let grid_selection = grid_selection.clone();
        let right_clicked_path = right_clicked_path.clone();
        Rc::new(move || -> Vec<String> {
            let mut paths = Vec::new();
            let bitset = grid_selection.selection();
            for i in 0..bitset.size() {
                let idx = bitset.nth(i as u32);
                if let Some(obj) = grid_store.item(idx) {
                    if let Some(boxed) = obj.downcast_ref::<glib::BoxedAnyObject>() {
                        let entry = boxed.borrow::<StoreEntry>();
                        paths.push(entry.path.clone());
                    }
                }
            }
            if let Some(rcp) = right_clicked_path.borrow().clone() {
                if !paths.contains(&rcp) {
                    paths.push(rcp);
                }
            }
            paths
        })
    };

    // ── Item export action ──────────────────────────────────────────────────
    let item_export_action = SimpleAction::new("item-export", None);
    window.add_action(&item_export_action);
    {
        let weak_window = window.downgrade();
        let selected_paths = selected_paths.clone();
        let busy_box = busy_box.clone();
        let busy_spinner = busy_spinner.clone();
        item_export_action.connect_activate(move |_, _| {
            let Some(window) = weak_window.upgrade() else {
                return;
            };
            let paths = selected_paths();
            if paths.is_empty() {
                return;
            }

            let dialog = gtk::FileDialog::new();
            dialog.set_title(&gettext("Export"));
            let weak_window2 = window.downgrade();
            let busy_box = busy_box.clone();
            let busy_spinner = busy_spinner.clone();
            dialog.select_folder(Some(&window), None::<&gio::Cancellable>, move |result| {
                if let Ok(file) = result {
                    if let Some(dest_dir) = file.path() {
                        let Some(window) = weak_window2.upgrade() else {
                            return;
                        };
                        let store = { window.store_mut().take() };
                        if let Some(store) = store {
                            busy_box.set_visible(true);
                            busy_spinner.set_spinning(true);
                            let busy_box = busy_box.clone();
                            let busy_spinner = busy_spinner.clone();
                            let weak = window.downgrade();
                            let dest_dir = dest_dir.to_string_lossy().to_string();
                            let (tx, rx) = std::sync::mpsc::channel();
                            std::thread::spawn(move || {
                                for src in &paths {
                                    let name = src.rsplit('/').next().unwrap_or("export");
                                    let dest_path = std::path::Path::new(&dest_dir).join(name);
                                    let _ = store.get(src, &dest_path.to_string_lossy());
                                }
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

    // ── Item copy action ────────────────────────────────────────────────────
    let item_copy_action = SimpleAction::new("item-copy", None);
    window.add_action(&item_copy_action);
    {
        let selected_paths = selected_paths.clone();
        let clipboard = clipboard.clone();
        let refresh = refresh.clone();
        item_copy_action.connect_activate(move |_, _| {
            let paths = selected_paths();
            if !paths.is_empty() {
                *clipboard.borrow_mut() = ClipboardState {
                    paths,
                    is_cut: false,
                };
                refresh();
            }
        });
    }

    // ── Item cut action ─────────────────────────────────────────────────────
    let item_cut_action = SimpleAction::new("item-cut", None);
    window.add_action(&item_cut_action);
    {
        let selected_paths = selected_paths.clone();
        let clipboard = clipboard.clone();
        let refresh = refresh.clone();
        item_cut_action.connect_activate(move |_, _| {
            let paths = selected_paths();
            if !paths.is_empty() {
                *clipboard.borrow_mut() = ClipboardState {
                    paths,
                    is_cut: true,
                };
                refresh();
            }
        });
    }

    // ── Item rename action ─────────────────────────────────────────────────
    let item_rename_action = SimpleAction::new("item-rename", None);
    window.add_action(&item_rename_action);
    {
        let weak_window = window.downgrade();
        let right_clicked_path = right_clicked_path.clone();
        let refresh = refresh.clone();
        item_rename_action.connect_activate(move |_, _| {
            let Some(window) = weak_window.upgrade() else {
                return;
            };
            let src = right_clicked_path.borrow().clone();
            let Some(src) = src else { return };

            let old_name = src.rsplit('/').next().unwrap_or(&src).to_string();

            let dialog = adw::AlertDialog::builder()
                .heading(gettext("Rename"))
                .build();
            dialog.add_response("cancel", &gettext("Cancel"));
            dialog.add_response("rename", &gettext("Rename"));
            dialog.set_response_appearance("rename", adw::ResponseAppearance::Suggested);
            dialog.set_default_response(Some("rename"));
            dialog.set_close_response("cancel");

            let entry = Entry::new();
            entry.set_text(&old_name);
            dialog.set_extra_child(Some(&entry));

            let weak_window2 = window.downgrade();
            let refresh = refresh.clone();
            dialog.choose(Some(&window), None::<&gio::Cancellable>, move |response| {
                if response != "rename" {
                    return;
                }
                let new_name = entry.text().to_string();
                if new_name.is_empty() || new_name == old_name {
                    return;
                }
                let Some(window) = weak_window2.upgrade() else {
                    return;
                };
                let parent = src
                    .rsplit_once('/')
                    .map_or("/", |(p, _)| if p.is_empty() { "/" } else { p });
                let dst = if parent == "/" {
                    format!("/{}", new_name)
                } else {
                    format!("{}/{}", parent, new_name)
                };
                {
                    let mut store_ref = window.store_mut();
                    if let Some(store) = store_ref.as_mut() {
                        if store.mv(&src, &dst).is_ok() {
                            let _ = store.save();
                        }
                    }
                }
                refresh();
            });
        });
    }

    // ── Item delete action ──────────────────────────────────────────────────
    let item_delete_action = SimpleAction::new("item-delete", None);
    window.add_action(&item_delete_action);
    {
        let weak_window = window.downgrade();
        let selected_paths = selected_paths.clone();
        let current_path = current_path.clone();
        let navigate_action = navigate_action.clone();
        let refresh = refresh.clone();
        item_delete_action.connect_activate(move |_, _| {
            let Some(window) = weak_window.upgrade() else {
                return;
            };
            let paths = selected_paths();
            if paths.is_empty() {
                return;
            }

            let heading = gettext("Delete?");
            let body = if paths.len() == 1 {
                let name = paths[0].rsplit('/').next().unwrap_or(&paths[0]);
                format!("\"{}\" {}", name, gettext("will be permanently deleted."))
            } else {
                format!(
                    "{} {}",
                    paths.len(),
                    gettext("items will be permanently deleted.")
                )
            };

            let dialog = adw::AlertDialog::builder()
                .heading(&heading)
                .body(&body)
                .build();
            dialog.add_response("cancel", &gettext("Cancel"));
            dialog.add_response("delete", &gettext("Delete"));
            dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);
            dialog.set_default_response(Some("cancel"));
            dialog.set_close_response("cancel");

            let weak_window2 = window.downgrade();
            let current_path = current_path.clone();
            let navigate_action = navigate_action.clone();
            let refresh = refresh.clone();
            dialog.choose(Some(&window), None::<&gio::Cancellable>, move |response| {
                if response == "delete" {
                    if let Some(window) = weak_window2.upgrade() {
                        {
                            let mut store_ref = window.store_mut();
                            if let Some(store) = store_ref.as_mut() {
                                for path in &paths {
                                    let _ = store.remove(path);
                                }
                            }
                        }
                        // If current folder was deleted, walk up the tree to the
                        // nearest surviving ancestor before refreshing.
                        let cwd = current_path.borrow().clone();
                        let cwd_gone = paths
                            .iter()
                            .any(|p| cwd == *p || cwd.starts_with(&format!("{}/", p)));
                        if cwd_gone {
                            let mut target = cwd.as_str();
                            loop {
                                target = match target.rsplit_once('/') {
                                    Some(("", _)) | None => "/",
                                    Some((parent, _)) => parent,
                                };
                                if target == "/" {
                                    break;
                                }
                                let exists = {
                                    let store_ref = window.store();
                                    store_ref.as_ref().map_or(false, |s| s.list(target).is_ok())
                                };
                                if exists {
                                    break;
                                }
                            }
                            refresh();
                            navigate_action.activate(Some(&target.to_variant()));
                        } else {
                            refresh();
                        }
                    }
                }
            });
        });
    }
}
