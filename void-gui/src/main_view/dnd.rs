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

use adw::gio;
use adw::glib;
use adw::gtk::{
    self, gdk, Box, DragSource, DropTarget, GridView, Label, MultiSelection, ProgressBar,
    ScrolledWindow, Spinner, Stack,
};
use adw::prelude::*;

use super::utils::*;
use crate::i18n::gettext;
use crate::window::VoidWindow;

/// Recursively computes the size in bytes of a file or directory on the local filesystem.
fn disk_size(path: &str) -> u64 {
    let meta = match std::fs::metadata(path) {
        Ok(m) => m,
        Err(_) => return 0,
    };
    if meta.is_file() {
        meta.len()
    } else if meta.is_dir() {
        let mut total = 0u64;
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                total += disk_size(&entry.path().to_string_lossy());
            }
        }
        total
    } else {
        0
    }
}

/// Creates a [`DropTarget`] that accepts file drops from the OS and imports them
/// into the vault under the current working directory.
///
/// Import runs on a background thread with progress bar updates.
fn make_drop_target(
    window: &VoidWindow,
    current_path: &Rc<RefCell<String>>,
    refresh: &Rc<dyn Fn()>,
    busy_box: &Box,
    busy_spinner: &Spinner,
    import_progress: &ProgressBar,
    import_label: &Label,
) -> DropTarget {
    let drop_target = DropTarget::new(
        gdk::FileList::static_type(),
        gdk::DragAction::COPY | gdk::DragAction::MOVE,
    );
    let weak_window = window.downgrade();
    let current_path = current_path.clone();
    let refresh = refresh.clone();
    let busy_box = busy_box.clone();
    let busy_spinner = busy_spinner.clone();
    let import_progress = import_progress.clone();
    let import_label = import_label.clone();
    drop_target.connect_drop(move |_, value, _, _| {
        let Ok(file_list) = value.get::<gdk::FileList>() else {
            return false;
        };
        let files: Vec<String> = file_list
            .files()
            .iter()
            .filter_map(|f| f.path())
            .map(|p| p.to_string_lossy().to_string())
            .collect();
        if files.is_empty() {
            return false;
        }
        let Some(window) = weak_window.upgrade() else {
            return false;
        };

        let total_bytes: u64 = files.iter().map(|p| disk_size(p)).sum();
        let bytes_done = Arc::new(AtomicU64::new(0));
        let bytes_done_th = bytes_done.clone();

        let cwd = current_path.borrow().clone();
        let store = { window.store_mut().take() };
        let Some(mut store) = store else {
            return false;
        };
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
        let refresh = refresh.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            for file_path in &files {
                let _ = store.add_with_progress(file_path, &cwd, bytes_done_th.clone());
            }
            let _ = tx.send(store);
        });
        glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
            match rx.try_recv() {
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
                        import_progress.set_fraction(done as f64 / total_bytes as f64);
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
            }
        });
        true
    });
    drop_target
}

/// Wires drag-and-drop for the main view: attaches drop targets (import) to the
/// grid scroll area and empty-folder placeholder, and a drag source (export) to
/// the grid view for dragging items out to the OS file manager.
pub(crate) fn setup(
    window: &VoidWindow,
    grid_view: &GridView,
    grid_scroll: &ScrolledWindow,
    grid_stack: &Stack,
    grid_store: &gio::ListStore,
    grid_selection: &MultiSelection,
    current_path: &Rc<RefCell<String>>,
    refresh: &Rc<dyn Fn()>,
    busy_box: &Box,
    busy_spinner: &Spinner,
    import_progress: &ProgressBar,
    import_label: &Label,
) {
    // ── Drop targets: import (drag files in) ────────────────────────────────
    {
        let dt_grid = make_drop_target(
            window,
            current_path,
            refresh,
            busy_box,
            busy_spinner,
            import_progress,
            import_label,
        );
        grid_scroll.add_controller(dt_grid);

        if let Some(empty_page) = grid_stack.child_by_name("empty") {
            let dt_empty = make_drop_target(
                window,
                current_path,
                refresh,
                busy_box,
                busy_spinner,
                import_progress,
                import_label,
            );
            empty_page.add_controller(dt_empty);
        }
    }

    // ── Drag source: export (drag items out) ────────────────────────────────
    {
        let drag_source = DragSource::new();
        drag_source.set_actions(gdk::DragAction::COPY);
        let grid_view_ref = grid_view.clone();
        let weak_window = window.downgrade();
        let grid_selection = grid_selection.clone();
        let grid_store = grid_store.clone();
        drag_source.connect_prepare(move |_, x, y| {
            let window = weak_window.upgrade()?;

            // ── Widget-tree walk: identify which grid item was dragged ───
            // `pick()` returns the innermost widget at (x, y).  Walk up the
            // parent chain until we find a widget whose name starts with '/'
            // — that name is the vault path set by the grid factory bind step.
            let picked = grid_view_ref.pick(x, y, gtk::PickFlags::DEFAULT)?;
            let mut widget = picked;
            let mut found_path = None;
            loop {
                let name = widget.widget_name();
                if name.starts_with('/') {
                    found_path = Some(name.to_string());
                    break;
                }
                if widget == grid_view_ref.clone().upcast::<gtk::Widget>() {
                    break;
                }
                match widget.parent() {
                    Some(p) => widget = p,
                    None => break,
                }
            }
            let dragged_path = found_path?;

            // ── Selection check: decide which items to export ────────────
            // If the dragged item is part of the current multi-selection,
            // export all selected *files* (folders are skipped).
            // Otherwise export only the single dragged item (if it is a file).
            let mut paths = Vec::new();
            let bitset = grid_selection.selection();
            let mut dragged_in_selection = false;
            for i in 0..bitset.size() {
                let idx = bitset.nth(i as u32);
                if let Some(obj) = grid_store.item(idx) {
                    if let Some(boxed) = obj.downcast_ref::<glib::BoxedAnyObject>() {
                        let entry = boxed.borrow::<StoreEntry>();
                        if entry.path == dragged_path {
                            dragged_in_selection = true;
                        }
                        if entry.is_file {
                            paths.push(entry.path.clone());
                        }
                    }
                }
            }
            if !dragged_in_selection {
                // Dragged item is not in the selection — treat it as a solo drag.
                let is_file = (0..grid_store.n_items()).any(|i| {
                    grid_store
                        .item(i)
                        .and_downcast::<glib::BoxedAnyObject>()
                        .map_or(false, |b| {
                            let e = b.borrow::<StoreEntry>();
                            e.path == dragged_path && e.is_file
                        })
                });
                if !is_file {
                    return None;
                }
                paths = vec![dragged_path];
            }
            if paths.is_empty() {
                return None;
            }

            // ── Temp-file export dance ──────────────────────────────────
            // Decrypt each vault file into a temporary directory, then wrap
            // the resulting OS paths in a `gdk::FileList` content provider
            // so the desktop file manager can receive plain files via DnD.
            let temp_dir = std::env::temp_dir().join("void-dnd");
            let _ = std::fs::remove_dir_all(&temp_dir);
            std::fs::create_dir_all(&temp_dir).ok();

            let files = {
                let store_ref = window.store();
                let store = store_ref.as_ref()?;
                let mut files = Vec::new();
                for src in &paths {
                    let name = src.rsplit('/').next().unwrap_or("export");
                    let dest = temp_dir.join(name);
                    if store.get(src, &dest.to_string_lossy()).is_ok() {
                        files.push(gio::File::for_path(&dest));
                    }
                }
                files
            };

            if files.is_empty() {
                return None;
            }

            let file_list = gdk::FileList::from_array(&files);
            Some(gdk::ContentProvider::for_value(&file_list.to_value()))
        });
        grid_view.add_controller(drag_source);
    }
}
