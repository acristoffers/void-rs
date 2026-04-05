/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use libadwaita as adw;
use regex::RegexBuilder;

use adw::gio::{self, SimpleAction};
use adw::glib;
use adw::gtk::{
    self, gdk, Align, Box, GridView, Label, MultiSelection, Orientation, ScrolledWindow,
    SignalListItemFactory, Stack,
};
use adw::prelude::*;

use super::utils::*;
use crate::file_viewer::open_file;
use crate::i18n::gettext;
use crate::window::VoidWindow;

// ── Query parsing ───────────────────────────────────────────────────────────────

/// Intermediate representation of a store entry during search/filter.
///
/// Extends the concept of [`StoreEntry`] with tag information needed for
/// query matching before the final grid population.
struct SearchCandidate {
    /// Display name shown in the grid cell.
    display_name: String,
    /// Full path inside the void store.
    full_path: String,
    /// `true` for files, `false` for directories.
    is_file: bool,
    /// Tags attached to this entry (used for `tag:value` query filtering).
    tags: Vec<String>,
}

/// Parses a search query string into tag filters and an optional regex pattern.
///
/// Tokens prefixed with `tag:` are collected as required tags; the remaining
/// tokens are joined into a case-insensitive regex.
pub(crate) fn parse_search_query(query: &str) -> (Vec<String>, Option<regex::Regex>) {
    let mut tags = Vec::new();
    let mut pattern_parts = Vec::new();
    for token in query.split_whitespace() {
        if let Some(tag) = token.strip_prefix("tag:") {
            if !tag.is_empty() {
                tags.push(tag.to_string());
            }
        } else {
            pattern_parts.push(token);
        }
    }
    let pattern = if pattern_parts.is_empty() {
        None
    } else {
        let pat = pattern_parts.join(" ");
        RegexBuilder::new(&pat).case_insensitive(true).build().ok()
    };
    (tags, pattern)
}

// ── Grid population ─────────────────────────────────────────────────────────────

/// Populates the grid store with entries matching `query` in either filter or search mode.
///
/// Filter mode restricts results to direct children of `current_path`;
/// search mode scans the entire vault. Thumbnail generation for matching
/// files is kicked off on background threads.
pub(crate) fn populate_grid_search(
    grid_store: &gio::ListStore,
    window: &VoidWindow,
    query: &str,
    search_mode: u8,
    current_path: &str,
    reverse: bool,
    generation: &Rc<Cell<u64>>,
) {
    let gen = generation.get() + 1;
    generation.set(gen);
    grid_store.remove_all();

    if query.is_empty() {
        return;
    }

    let (tags, pattern) = parse_search_query(query);

    let store_ref = window.store();
    let Some(store) = store_ref.as_ref() else {
        return;
    };

    let entries: Vec<SearchCandidate> = if search_mode == SEARCH_MODE_FILTER {
        let Ok(files) = store.list(current_path) else {
            return;
        };
        files
            .into_iter()
            .map(|f| {
                let full_path = if current_path == "/" {
                    format!("/{}", f.name)
                } else {
                    format!("{}/{}", current_path, f.name)
                };
                SearchCandidate {
                    display_name: f.name,
                    full_path,
                    is_file: f.is_file,
                    tags: f.tags,
                }
            })
            .collect()
    } else {
        let Ok(files) = store.list("*") else {
            return;
        };
        files
            .into_iter()
            .filter(|f| f.id != 0)
            .map(|f| {
                let display_name = f.name.rsplit('/').next().unwrap_or(&f.name).to_string();
                SearchCandidate {
                    display_name,
                    full_path: f.name,
                    is_file: f.is_file,
                    tags: f.tags,
                }
            })
            .collect()
    };

    let mut filtered: Vec<StoreEntry> = entries
        .into_iter()
        .filter(|c| {
            if !tags.is_empty() && !tags.iter().all(|t| c.tags.contains(t)) {
                return false;
            }
            if let Some(re) = &pattern {
                if !re.is_match(&c.display_name) {
                    return false;
                }
            }
            true
        })
        .map(|c| StoreEntry {
            name: c.display_name,
            path: c.full_path,
            is_file: c.is_file,
            thumbnail: None,
        })
        .collect();

    // Sort folders before files, then alphabetically (optionally reversed).
    filtered.sort_by(|a, b| match (a.is_file, b.is_file) {
        (false, true) => std::cmp::Ordering::Less,
        (true, false) => std::cmp::Ordering::Greater,
        _ => {
            let cmp = a.name.to_lowercase().cmp(&b.name.to_lowercase());
            if reverse {
                cmp.reverse()
            } else {
                cmp
            }
        }
    });

    let store_clone = store.clone();
    drop(store_ref);

    let mut work: Vec<ThumbnailWork> = Vec::new();

    for (i, entry) in filtered.iter().enumerate() {
        let thumbnail = if entry.is_file {
            if let Some(cached) = crate::thumbnails::cached_thumbnail(window, &entry.path) {
                Some(cached)
            } else if crate::thumbnails::supports_thumbnail(&entry.name) {
                work.push(ThumbnailWork {
                    grid_index: i as u32,
                    store_path: entry.path.clone(),
                    file_name: entry.name.clone(),
                });
                None
            } else {
                None
            }
        } else {
            None
        };

        grid_store.append(&glib::BoxedAnyObject::new(StoreEntry {
            name: entry.name.clone(),
            path: entry.path.clone(),
            is_file: entry.is_file,
            thumbnail,
        }));
    }

    if work.is_empty() {
        return;
    }

    let (tx, rx) = std::sync::mpsc::channel::<ThumbnailResult>();
    let pending = Rc::new(Cell::new(work.len()));

    // Spawn one background thread per thumbnail work item.
    for ThumbnailWork {
        grid_index: idx,
        store_path,
        file_name: name,
    } in work
    {
        let tx = tx.clone();
        let store = store_clone.clone();
        std::thread::spawn(move || {
            let thumb = (|| {
                let bytes = store.get_bytes(&store_path).ok()?;
                crate::thumbnails::generate_thumbnail(&name, bytes)
            })();
            if let Some(thumb_bytes) = thumb {
                let _ = tx.send(ThumbnailResult {
                    grid_index: idx,
                    store_path,
                    jpeg_bytes: thumb_bytes,
                });
            }
        });
    }
    drop(tx);

    let grid_store = grid_store.clone();
    let generation = generation.clone();
    let weak_window = window.downgrade();

    // Poll the channel every 50 ms to collect completed thumbnail results.
    glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
        // Drain all available results from the background threads.
        while let Ok(ThumbnailResult {
            grid_index: idx,
            store_path,
            jpeg_bytes: thumb_bytes,
        }) = rx.try_recv()
        {
            pending.set(pending.get().saturating_sub(1));
            // Skip stale results if a new generation has started.
            if generation.get() != gen {
                continue;
            }
            // Cache the generated thumbnail for future use.
            if let Some(window) = weak_window.upgrade() {
                crate::thumbnails::cache_thumbnail(&window, &store_path, &thumb_bytes);
            }
            // Replace the grid item with an updated entry that includes the thumbnail.
            if let Some(obj) = grid_store.item(idx).and_downcast::<glib::BoxedAnyObject>() {
                let old = obj.borrow::<StoreEntry>();
                let updated = StoreEntry {
                    name: old.name.clone(),
                    path: old.path.clone(),
                    is_file: old.is_file,
                    thumbnail: Some(thumb_bytes),
                };
                drop(old);
                grid_store.splice(idx, 1, &[glib::BoxedAnyObject::new(updated)]);
            }
        }
        // Stop polling once all thumbnails are done or generation has changed.
        if pending.get() == 0 || generation.get() != gen {
            if let Some(window) = weak_window.upgrade() {
                let mut store_ref = window.store_mut();
                if let Some(store) = store_ref.as_mut() {
                    let _ = store.save();
                }
            }
            return glib::ControlFlow::Break;
        }
        glib::ControlFlow::Continue
    });
}

/// Returns a `ListStore` of folder-only `StoreEntry` items that are direct children of `path`.
pub(crate) fn folder_model(window: &VoidWindow, path: &str) -> gio::ListStore {
    let model = gio::ListStore::new::<glib::BoxedAnyObject>();
    let store_ref = window.store();
    if let Some(store) = store_ref.as_ref() {
        if let Ok(entries) = store.list(path) {
            for entry in entries.iter().filter(|e| !e.is_file) {
                let full_path = if path == "/" {
                    format!("/{}", entry.name)
                } else {
                    format!("{}/{}", path, entry.name)
                };
                model.append(&glib::BoxedAnyObject::new(StoreEntry {
                    name: entry.name.clone(),
                    path: full_path,
                    is_file: false,
                    thumbnail: None,
                }));
            }
        }
    }
    model
}

/// Populates the grid store with all entries (files and folders) under `path`.
///
/// Entries are sorted folders-first, then alphabetically (optionally reversed).
/// Thumbnail generation for eligible files is dispatched to background threads.
pub(crate) fn populate_grid(
    grid_store: &gio::ListStore,
    window: &VoidWindow,
    path: &str,
    reverse: bool,
    generation: &Rc<Cell<u64>>,
) {
    let gen = generation.get() + 1;
    generation.set(gen);

    grid_store.remove_all();

    let (entries, store_clone) = {
        let store_ref = window.store();
        let Some(store) = store_ref.as_ref() else {
            return;
        };
        let Ok(mut entries) = store.list(path) else {
            return;
        };

        entries.sort_by(|a, b| match (a.is_file, b.is_file) {
            (false, true) => std::cmp::Ordering::Less,
            (true, false) => std::cmp::Ordering::Greater,
            _ => {
                let cmp = a.name.to_lowercase().cmp(&b.name.to_lowercase());
                if reverse {
                    cmp.reverse()
                } else {
                    cmp
                }
            }
        });
        (entries, store.clone())
    };

    let mut work: Vec<ThumbnailWork> = Vec::new();

    for (i, entry) in entries.iter().enumerate() {
        let full_path = if path == "/" {
            format!("/{}", entry.name)
        } else {
            format!("{}/{}", path, entry.name)
        };

        let thumbnail = if entry.is_file {
            if let Some(cached) = crate::thumbnails::cached_thumbnail(window, &full_path) {
                Some(cached)
            } else if crate::thumbnails::supports_thumbnail(&entry.name) {
                work.push(ThumbnailWork {
                    grid_index: i as u32,
                    store_path: full_path.clone(),
                    file_name: entry.name.clone(),
                });
                None
            } else {
                None
            }
        } else {
            None
        };

        grid_store.append(&glib::BoxedAnyObject::new(StoreEntry {
            name: entry.name.clone(),
            path: full_path,
            is_file: entry.is_file,
            thumbnail,
        }));
    }

    if work.is_empty() {
        return;
    }

    let (tx, rx) = std::sync::mpsc::channel::<ThumbnailResult>();
    let pending = Rc::new(Cell::new(work.len()));

    // Spawn one background thread per thumbnail work item.
    for ThumbnailWork {
        grid_index: idx,
        store_path,
        file_name: name,
    } in work
    {
        let tx = tx.clone();
        let store = store_clone.clone();

        std::thread::spawn(move || {
            let thumb = (|| {
                let bytes = store.get_bytes(&store_path).ok()?;
                crate::thumbnails::generate_thumbnail(&name, bytes)
            })();
            if let Some(thumb_bytes) = thumb {
                let _ = tx.send(ThumbnailResult {
                    grid_index: idx,
                    store_path,
                    jpeg_bytes: thumb_bytes,
                });
            }
        });
    }
    drop(tx);

    let grid_store = grid_store.clone();
    let generation = generation.clone();
    let weak_window = window.downgrade();

    // Poll the channel every 50 ms to collect completed thumbnail results.
    glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
        // Drain all available results from the background threads.
        while let Ok(ThumbnailResult {
            grid_index: idx,
            store_path,
            jpeg_bytes: thumb_bytes,
        }) = rx.try_recv()
        {
            pending.set(pending.get().saturating_sub(1));

            // Skip stale results if a new generation has started.
            if generation.get() != gen {
                continue;
            }

            // Cache the generated thumbnail for future use.
            if let Some(window) = weak_window.upgrade() {
                crate::thumbnails::cache_thumbnail(&window, &store_path, &thumb_bytes);
            }

            // Replace the grid item with an updated entry that includes the thumbnail.
            if let Some(obj) = grid_store.item(idx).and_downcast::<glib::BoxedAnyObject>() {
                let old = obj.borrow::<StoreEntry>();
                let updated = StoreEntry {
                    name: old.name.clone(),
                    path: old.path.clone(),
                    is_file: old.is_file,
                    thumbnail: Some(thumb_bytes),
                };
                drop(old);
                grid_store.splice(idx, 1, &[glib::BoxedAnyObject::new(updated)]);
            }
        }

        // Stop polling once all thumbnails are done or generation has changed.
        if pending.get() == 0 || generation.get() != gen {
            if let Some(window) = weak_window.upgrade() {
                let mut store_ref = window.store_mut();
                if let Some(store) = store_ref.as_mut() {
                    let _ = store.save();
                }
            }
            return glib::ControlFlow::Break;
        }

        glib::ControlFlow::Continue
    });
}

// ── Grid widget construction ────────────────────────────────────────────────────

/// Collected widget references produced by [`build_grid`].
pub(crate) struct GridWidgets {
    pub grid_store: gio::ListStore,
    pub grid_selection: MultiSelection,
    pub grid_view: GridView,
    pub grid_scroll: ScrolledWindow,
    pub grid_stack: Stack,
}

/// Constructs the file/folder grid view and its backing model.
///
/// Returns a [`GridWidgets`] bundle containing the `ListStore`, selection
/// model, `GridView`, scroll wrapper, and a `Stack` that toggles between
/// the grid and an empty-folder placeholder.
pub(crate) fn build_grid(
    icon_size: &Rc<Cell<i32>>,
    clipboard: &Rc<RefCell<ClipboardState>>,
) -> GridWidgets {
    let grid_store = gio::ListStore::new::<glib::BoxedAnyObject>();

    let grid_selection = MultiSelection::new(Some(grid_store.clone()));

    let grid_factory = SignalListItemFactory::new();
    grid_factory.connect_setup(|_, list_item| {
        let list_item = list_item.downcast_ref::<gtk::ListItem>().unwrap();

        let icon = gtk::Image::new();

        let label = Label::new(None);
        label.set_ellipsize(adw::gtk::pango::EllipsizeMode::End);
        label.set_max_width_chars(14);
        label.set_halign(Align::Center);
        label.set_wrap(true);
        label.set_justify(gtk::Justification::Center);

        let vbox = Box::new(Orientation::Vertical, 6);
        vbox.set_halign(Align::Center);
        vbox.set_valign(Align::Start);
        vbox.set_margin_top(6);
        vbox.set_margin_bottom(6);
        vbox.set_margin_start(6);
        vbox.set_margin_end(6);
        vbox.append(&icon);
        vbox.append(&label);

        list_item.set_child(Some(&vbox));
    });

    grid_factory.connect_bind({
        let icon_size = icon_size.clone();
        let clipboard = clipboard.clone();
        move |_, list_item| {
            let list_item = list_item.downcast_ref::<gtk::ListItem>().unwrap();
            if let Some(item) = list_item.item().and_downcast::<glib::BoxedAnyObject>() {
                let entry = item.borrow::<StoreEntry>();
                let size = icon_size.get();
                if let Some(vbox) = list_item.child().and_downcast::<Box>() {
                    vbox.set_widget_name(&entry.path);
                    vbox.set_size_request(size + 32, -1);

                    let cb = clipboard.borrow();
                    if cb.is_cut && cb.paths.contains(&entry.path) {
                        vbox.set_opacity(0.4);
                    } else {
                        vbox.set_opacity(1.0);
                    }

                    if let Some(icon) = vbox.first_child().and_downcast::<gtk::Image>() {
                        icon.set_pixel_size(size);
                        if let Some(thumb) = &entry.thumbnail {
                            let bytes = glib::Bytes::from(thumb.as_slice());
                            if let Ok(texture) = gdk::Texture::from_bytes(&bytes) {
                                icon.set_paintable(Some(&texture));
                            } else {
                                icon.set_icon_name(Some("text-x-generic"));
                            }
                        } else {
                            icon.set_icon_name(Some(if entry.is_file {
                                icon_name_for_mime_type(&entry.name)
                            } else {
                                "folder"
                            }));
                        }
                        if let Some(label) = icon.next_sibling().and_downcast::<Label>() {
                            label.set_text(&entry.name);
                        }
                    }
                }
            }
        }
    });

    let grid_view = GridView::new(Some(grid_selection.clone()), Some(grid_factory));
    grid_view.set_min_columns(2);
    grid_view.set_max_columns(20);

    let grid_scroll = ScrolledWindow::builder()
        .child(&grid_view)
        .hexpand(true)
        .vexpand(true)
        .build();

    let empty_page = adw::StatusPage::new();
    empty_page.set_icon_name(Some("folder-open-symbolic"));
    empty_page.set_title(&gettext("Empty Folder"));
    empty_page.set_hexpand(true);
    empty_page.set_vexpand(true);

    let grid_stack = Stack::new();
    grid_stack.add_named(&grid_scroll, Some("grid"));
    grid_stack.add_named(&empty_page, Some("empty"));
    grid_stack.set_visible_child_name("grid");

    GridWidgets {
        grid_store,
        grid_selection,
        grid_view,
        grid_scroll,
        grid_stack,
    }
}

// ── Grid wiring helpers ─────────────────────────────────────────────────────────

/// Connects the grid view's `activate` signal so that double-clicking a folder
/// navigates into it and double-clicking a file opens it in the file viewer.
pub(crate) fn wire_grid_activation(
    grid_view: &GridView,
    grid_store: &gio::ListStore,
    navigate_action: &SimpleAction,
    window: &VoidWindow,
) {
    let grid_store = grid_store.clone();
    let navigate_action = navigate_action.clone();
    let weak_window = window.downgrade();
    grid_view.connect_activate(move |_, position| {
        let Some(item) = grid_store.item(position) else {
            return;
        };
        let Some(obj) = item.downcast_ref::<glib::BoxedAnyObject>() else {
            return;
        };
        let entry = obj.borrow::<StoreEntry>();
        let path = entry.path.clone();
        let is_file = entry.is_file;
        drop(entry);
        if is_file {
            if let Some(w) = weak_window.upgrade() {
                open_file(&w, &path);
            }
        } else {
            navigate_action.activate(Some(&path.to_variant()));
        }
    });
}

/// Watches the grid store's item count and automatically switches between the
/// grid page and the empty-folder placeholder.
pub(crate) fn wire_empty_folder_toggle(grid_store: &gio::ListStore, grid_stack: &Stack) {
    let grid_stack = grid_stack.clone();
    grid_store.connect_items_changed(move |store, _, _, _| {
        if store.n_items() == 0 {
            grid_stack.set_visible_child_name("empty");
        } else {
            grid_stack.set_visible_child_name("grid");
        }
    });
}
