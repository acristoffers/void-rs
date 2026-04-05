/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::cell::RefCell;
use std::rc::Rc;

use libadwaita as adw;

use adw::gio::{self};
use adw::glib;
use adw::gtk::{
    self, Align, Box, Button, Entry, Label, ListView, MultiSelection, Orientation, ScrolledWindow,
    SignalListItemFactory, SingleSelection, TreeExpander, TreeListModel, TreeListRow,
};
use adw::prelude::*;

use super::grid::folder_model;
use super::utils::*;
use crate::i18n::gettext;
use crate::window::VoidWindow;

// ── Folder tree (left sidebar) ──────────────────────────────────────────────────

/// Collected widget references produced by [`build_folder_tree`] for the folder tree sidebar.
pub(crate) struct TreeWidgets {
    /// Root list store backing the top-level folders in the tree.
    pub tree_root: gio::ListStore,
    /// Single-selection model wrapping the recursive tree list model.
    pub tree_selection: SingleSelection,
    /// The `ListView` displaying the folder tree.
    pub tree_view: ListView,
    /// Scrolled window containing the tree view.
    pub tree_scroll: ScrolledWindow,
}

/// Constructs the folder tree sidebar using a [`TreeListModel`] backed by
/// recursive [`folder_model`] queries. Returns a [`TreeWidgets`] bundle.
pub(crate) fn build_folder_tree(window: &VoidWindow) -> TreeWidgets {
    let tree_root = folder_model(window, "/");

    let weak_window = window.downgrade();
    let tree_model = TreeListModel::new(tree_root.clone(), false, false, move |item| {
        let window = weak_window.upgrade()?;
        let obj = item.downcast_ref::<glib::BoxedAnyObject>()?;
        let entry = obj.borrow::<StoreEntry>();
        let children = folder_model(&window, &entry.path);
        if children.n_items() > 0 {
            Some(children.upcast())
        } else {
            None
        }
    });

    let tree_selection = SingleSelection::new(Some(tree_model));
    tree_selection.set_autoselect(false);
    tree_selection.set_can_unselect(true);

    let tree_factory = SignalListItemFactory::new();
    tree_factory.connect_setup(|_, list_item| {
        let list_item = list_item.downcast_ref::<gtk::ListItem>().unwrap();
        let expander = TreeExpander::new();

        let icon = gtk::Image::from_icon_name("folder-symbolic");
        let label = Label::new(None);
        label.set_halign(Align::Start);
        label.set_hexpand(true);

        let hbox = Box::new(Orientation::Horizontal, 6);
        hbox.append(&icon);
        hbox.append(&label);
        hbox.set_margin_start(4);
        hbox.set_margin_end(4);
        hbox.set_margin_top(2);
        hbox.set_margin_bottom(2);

        expander.set_child(Some(&hbox));
        list_item.set_child(Some(&expander));
    });

    tree_factory.connect_bind(|_, list_item| {
        let list_item = list_item.downcast_ref::<gtk::ListItem>().unwrap();
        let expander = list_item.child().and_downcast::<TreeExpander>().unwrap();
        let row = list_item.item().and_downcast::<TreeListRow>().unwrap();

        expander.set_list_row(Some(&row));

        if let Some(item) = row.item().and_downcast::<glib::BoxedAnyObject>() {
            let entry = item.borrow::<StoreEntry>();
            if let Some(hbox) = expander.child().and_downcast::<Box>() {
                hbox.set_widget_name(&entry.path);
                if let Some(label) = hbox.last_child().and_downcast::<Label>() {
                    label.set_text(&entry.name);
                }
            }
        }
    });

    let tree_view = ListView::new(Some(tree_selection.clone()), Some(tree_factory));
    tree_view.add_css_class("navigation-sidebar");

    let tree_scroll = ScrolledWindow::builder()
        .child(&tree_view)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .build();

    TreeWidgets {
        tree_root,
        tree_selection,
        tree_view,
        tree_scroll,
    }
}

/// Updates the tree selection to match the current working directory.
///
/// Called from the navigate action so the folder tree sidebar stays in sync
/// with breadcrumb / grid navigation. Clears the selection when the path
/// is the vault root or is not currently visible (parent not expanded).
pub(crate) fn sync_tree_selection(tree_selection: &SingleSelection, path: &str) {
    if path == "/" {
        tree_selection.set_selected(gtk::INVALID_LIST_POSITION);
        return;
    }
    if let Some(model) = tree_selection.model() {
        for i in 0..model.n_items() {
            if let Some(row) = model.item(i).and_downcast::<TreeListRow>() {
                if let Some(item) = row.item().and_downcast::<glib::BoxedAnyObject>() {
                    let entry = item.borrow::<StoreEntry>();
                    if entry.path == path {
                        tree_selection.set_selected(i);
                        return;
                    }
                }
            }
        }
    }
    // Path not visible in tree (parent not expanded) — clear selection
    tree_selection.set_selected(gtk::INVALID_LIST_POSITION);
}

// ── Info pane (right sidebar) ───────────────────────────────────────────────────

/// Collected widget references produced by [`build_info_pane`] for the right-side information panel.
pub(crate) struct InfoPaneWidgets {
    /// Top-level vertical box containing the entire info pane.
    pub info_pane: Box,
    /// Image widget showing the selected item's icon or thumbnail.
    pub info_pane_icon: gtk::Image,
    /// Label displaying the selected item's file/folder name.
    pub info_pane_title: Label,
    /// List box holding metadata rows (size, MIME type, custom key-value pairs).
    pub info_list: gtk::ListBox,
    /// List box holding tag rows.
    pub tag_list: gtk::ListBox,
    /// Button that opens the "Add Metadata" dialog.
    pub add_metadata_btn: Button,
    /// Button that opens the "Add Tag" dialog.
    pub add_tag_btn: Button,
}

/// Constructs the right-side information pane showing file icon, title,
/// metadata rows, and tag list. Returns an [`InfoPaneWidgets`] bundle.
pub(crate) fn build_info_pane() -> InfoPaneWidgets {
    let info_pane_title = Label::new(None);
    info_pane_title.add_css_class("title-2");
    info_pane_title.set_margin_top(12);
    info_pane_title.set_margin_start(12);
    info_pane_title.set_margin_end(12);

    let info_pane_icon = gtk::Image::new();
    info_pane_icon.set_pixel_size(96);
    info_pane_icon.set_margin_top(12);

    let icon_box = Box::new(Orientation::Vertical, 0);
    icon_box.set_halign(Align::Center);
    icon_box.append(&info_pane_icon);
    icon_box.append(&info_pane_title);

    let info_list = gtk::ListBox::new();
    info_list.set_selection_mode(gtk::SelectionMode::None);
    info_list.add_css_class("boxed-list");
    info_list.set_margin_start(12);
    info_list.set_margin_end(12);
    info_list.set_margin_top(12);

    let add_metadata_btn = Button::with_label(&gettext("Add Metadata"));
    add_metadata_btn.add_css_class("pill");
    add_metadata_btn.set_margin_start(12);
    add_metadata_btn.set_margin_end(12);
    add_metadata_btn.set_margin_top(8);
    add_metadata_btn.set_margin_bottom(4);
    add_metadata_btn.set_halign(Align::Fill);

    let tag_list = gtk::ListBox::new();
    tag_list.set_selection_mode(gtk::SelectionMode::None);
    tag_list.add_css_class("boxed-list");
    tag_list.set_margin_start(12);
    tag_list.set_margin_end(12);
    tag_list.set_margin_top(12);

    let add_tag_btn = Button::with_label(&gettext("Add Tag"));
    add_tag_btn.add_css_class("pill");
    add_tag_btn.set_margin_start(12);
    add_tag_btn.set_margin_end(12);
    add_tag_btn.set_margin_top(4);
    add_tag_btn.set_margin_bottom(12);
    add_tag_btn.set_halign(Align::Fill);

    let info_pane = Box::new(Orientation::Vertical, 0);
    info_pane.set_width_request(250);
    info_pane.append(&icon_box);
    info_pane.append(&info_list);
    info_pane.append(&add_metadata_btn);
    info_pane.append(&tag_list);
    info_pane.append(&add_tag_btn);

    InfoPaneWidgets {
        info_pane,
        info_pane_icon,
        info_pane_title,
        info_list,
        tag_list,
        add_metadata_btn,
        add_tag_btn,
    }
}

// ── Side panel update closure ───────────────────────────────────────────────────

/// Builds a closure that refreshes the info pane to reflect the currently
/// selected (or right-clicked) item's metadata, size, and tags.
pub(crate) fn build_update_side_panel(
    window: &VoidWindow,
    ip: &InfoPaneWidgets,
    grid_store: &gio::ListStore,
    grid_selection: &MultiSelection,
    right_clicked_path: &Rc<RefCell<Option<String>>>,
) -> Rc<dyn Fn()> {
    let weak_window = window.downgrade();
    let info_pane_icon = ip.info_pane_icon.clone();
    let info_pane_title = ip.info_pane_title.clone();
    let info_list = ip.info_list.clone();
    let tag_list = ip.tag_list.clone();
    let grid_store = grid_store.clone();
    let grid_selection = grid_selection.clone();
    let right_clicked_path = right_clicked_path.clone();
    Rc::new(move || {
        while let Some(child) = info_list.first_child() {
            info_list.remove(&child);
        }
        while let Some(child) = tag_list.first_child() {
            tag_list.remove(&child);
        }

        // Determine which item to display:
        // 1. Use the first item in the grid's multi-selection if any.
        // 2. Fall back to the path stored by the most recent right-click.
        let mut selected_path: Option<String> = None;
        let bitset = grid_selection.selection();
        if bitset.size() > 0 {
            let idx = bitset.nth(0);
            if let Some(obj) = grid_store.item(idx).and_downcast::<glib::BoxedAnyObject>() {
                selected_path = Some(obj.borrow::<StoreEntry>().path.clone());
            }
        }
        if selected_path.is_none() {
            selected_path = right_clicked_path.borrow().clone();
        }

        let Some(path) = selected_path else {
            info_pane_title.set_text("");
            info_pane_icon.set_icon_name(None);
            return;
        };

        let Some(window) = weak_window.upgrade() else {
            return;
        };

        let store_ref = window.store();
        let Some(store) = store_ref.as_ref() else {
            return;
        };

        // Get the StoreEntry from grid_store for icon/thumbnail
        let entry_opt: Option<StoreEntry> = (0..grid_store.n_items()).find_map(|i| {
            grid_store
                .item(i)
                .and_downcast::<glib::BoxedAnyObject>()
                .map(|obj| obj.borrow::<StoreEntry>().clone())
                .filter(|e| e.path == path)
        });

        let name = path.rsplit('/').next().unwrap_or(&path).to_string();
        info_pane_title.set_text(&name);

        // Set icon/thumbnail
        if let Some(ref entry) = entry_opt {
            if let Some(thumb) = &entry.thumbnail {
                let bytes = glib::Bytes::from(thumb.as_slice());
                if let Ok(texture) = gtk::gdk::Texture::from_bytes(&bytes) {
                    info_pane_icon.set_paintable(Some(&texture));
                } else {
                    let icon = if entry.is_file {
                        icon_name_for_mime_type(&name)
                    } else {
                        "folder"
                    };
                    info_pane_icon.set_icon_name(Some(icon));
                }
            } else {
                let icon = if entry.is_file {
                    icon_name_for_mime_type(&name)
                } else {
                    "folder"
                };
                info_pane_icon.set_icon_name(Some(icon));
            }
        }

        let is_file = entry_opt.as_ref().map_or(true, |e| e.is_file);

        // Size row
        let size_str = if is_file {
            store
                .list(&path)
                .ok()
                .and_then(|v| v.into_iter().next())
                .map(|f| format_size(f.size))
                .unwrap_or_else(|| "—".to_string())
        } else {
            let total = folder_total_size(store, &path);
            format_size(total)
        };
        let size_row = adw::ActionRow::builder()
            .title(&gettext("Size"))
            .subtitle(&size_str)
            .activatable(false)
            .build();
        info_list.append(&size_row);

        // Children row (folders only)
        if !is_file {
            let children = store.list(&path).map(|v| v.len()).unwrap_or(0);
            let children_row = adw::ActionRow::builder()
                .title(&gettext("Children"))
                .subtitle(children.to_string())
                .activatable(false)
                .build();
            info_list.append(&children_row);
        }

        // Metadata rows — each editable key-value pair gets an ActionRow with
        // a delete button (suffix) and an on-activate edit dialog. Read-only
        // keys like "mimetype" are displayed without delete/edit controls.
        if let Ok(metadata) = store.metadata_list(&path) {
            let mut keys: Vec<String> = metadata
                .keys()
                .filter(|k| k.as_str() != "_thumbnail_")
                .cloned()
                .collect();
            keys.sort();

            for key in keys {
                let value = metadata[&key].clone();
                let readonly = key.as_str() == "mimetype";
                let display_title = if readonly {
                    gettext("MIME Type")
                } else {
                    key.clone()
                };

                let row = adw::ActionRow::builder()
                    .title(glib::markup_escape_text(&display_title))
                    .subtitle(glib::markup_escape_text(&value))
                    .activatable(!readonly)
                    .build();

                if !readonly {
                    let del_btn = Button::new();
                    del_btn.set_icon_name("edit-delete-symbolic");
                    del_btn.add_css_class("flat");
                    del_btn.set_valign(Align::Center);
                    row.add_suffix(&del_btn);

                    {
                        let path = path.clone();
                        let weak_win = window.downgrade();
                        let info_list = info_list.clone();
                        let row_ref = row.clone();
                        let key = key.clone();
                        del_btn.connect_clicked(move |_| {
                            if let Some(w) = weak_win.upgrade() {
                                let mut store_ref = w.store_mut();
                                if let Some(store) = store_ref.as_mut() {
                                    let _ = store.metadata_remove(&path, &key);
                                }
                                drop(store_ref);
                                info_list.remove(&row_ref);
                            }
                        });
                    }

                    {
                        let path = path.clone();
                        let key = key.clone();
                        let weak_win = window.downgrade();
                        let row_ref = row.clone();
                        row.connect_activated(move |_| {
                            let Some(w) = weak_win.upgrade() else { return };
                            let dialog = adw::AlertDialog::builder()
                                .heading(glib::markup_escape_text(&key))
                                .build();
                            dialog.add_response("cancel", &gettext("Cancel"));
                            dialog.add_response("save", &gettext("Save"));
                            dialog.set_default_response(Some("save"));
                            dialog.set_response_appearance(
                                "save",
                                adw::ResponseAppearance::Suggested,
                            );
                            dialog.set_close_response("cancel");
                            let val_entry = Entry::new();
                            val_entry.set_text(&value);
                            dialog.set_extra_child(Some(&val_entry));
                            let path = path.clone();
                            let key = key.clone();
                            let weak_win2 = w.downgrade();
                            let row_ref = row_ref.clone();
                            dialog.choose(Some(&w), None::<&gio::Cancellable>, move |resp| {
                                if resp != "save" {
                                    return;
                                }
                                let new_value = val_entry.text().to_string();
                                if let Some(w) = weak_win2.upgrade() {
                                    let mut store_ref = w.store_mut();
                                    if let Some(store) = store_ref.as_mut() {
                                        let _ = store.metadata_set(&path, &key, &new_value);
                                    }
                                    drop(store_ref);
                                    row_ref.set_subtitle(
                                        glib::markup_escape_text(&new_value).as_str(),
                                    );
                                }
                            });
                        });
                    }
                }

                info_list.append(&row);
            }
        }

        // Tag rows — each tag is an ActionRow with a delete button that
        // removes the tag from the store and the list on click.
        if let Ok(mut tags) = store.tag_get(&path) {
            tags.sort();
            for tag in tags {
                let row = adw::ActionRow::builder()
                    .title(glib::markup_escape_text(&tag))
                    .activatable(false)
                    .build();

                let del_btn = Button::new();
                del_btn.set_icon_name("edit-delete-symbolic");
                del_btn.add_css_class("flat");
                del_btn.set_valign(Align::Center);
                row.add_suffix(&del_btn);

                {
                    let path = path.clone();
                    let tag = tag.clone();
                    let weak_win = window.downgrade();
                    let tag_list = tag_list.clone();
                    let row_ref = row.clone();
                    del_btn.connect_clicked(move |_| {
                        if let Some(w) = weak_win.upgrade() {
                            let mut store_ref = w.store_mut();
                            if let Some(store) = store_ref.as_mut() {
                                let _ = store.tag_rm(&path, &tag);
                            }
                            drop(store_ref);
                            tag_list.remove(&row_ref);
                        }
                    });
                }

                tag_list.append(&row);
            }
        }
    })
}

// ── Wire info pane buttons ──────────────────────────────────────────────────────

/// Connects the 'Add Metadata' and 'Add Tag' buttons in the info pane,
/// presenting input dialogs and updating the store on confirmation.
pub(crate) fn wire_info_pane_buttons(
    window: &VoidWindow,
    ip: &InfoPaneWidgets,
    grid_store: &gio::ListStore,
    grid_selection: &MultiSelection,
    right_clicked_path: &Rc<RefCell<Option<String>>>,
    update_side_panel: &Rc<dyn Fn()>,
) {
    // Add metadata button
    {
        let weak_window = window.downgrade();
        let grid_selection = grid_selection.clone();
        let grid_store = grid_store.clone();
        let right_clicked_path = right_clicked_path.clone();
        let update_side_panel = update_side_panel.clone();
        ip.add_metadata_btn.connect_clicked(move |_| {
            let Some(window) = weak_window.upgrade() else {
                return;
            };

            let mut selected_path: Option<String> = None;
            let bitset = grid_selection.selection();
            if bitset.size() > 0 {
                let idx = bitset.nth(0);
                if let Some(obj) = grid_store.item(idx).and_downcast::<glib::BoxedAnyObject>() {
                    selected_path = Some(obj.borrow::<StoreEntry>().path.clone());
                }
            }
            if selected_path.is_none() {
                selected_path = right_clicked_path.borrow().clone();
            }
            let Some(path) = selected_path else { return };

            let dialog = adw::AlertDialog::builder()
                .heading(&gettext("Add Metadata"))
                .build();
            dialog.add_response("cancel", &gettext("Cancel"));
            dialog.add_response("add", &gettext("Add"));
            dialog.set_default_response(Some("add"));
            dialog.set_response_appearance("add", adw::ResponseAppearance::Suggested);
            dialog.set_close_response("cancel");

            let entries_box = gtk::Box::new(gtk::Orientation::Vertical, 8);
            let key_entry = Entry::new();
            key_entry.set_placeholder_text(Some(&gettext("Key")));
            let val_entry = Entry::new();
            val_entry.set_placeholder_text(Some(&gettext("Value")));
            entries_box.append(&key_entry);
            entries_box.append(&val_entry);
            dialog.set_extra_child(Some(&entries_box));

            let weak = window.downgrade();
            let update_side_panel = update_side_panel.clone();
            dialog.choose(Some(&window), None::<&gio::Cancellable>, move |resp| {
                if resp != "add" {
                    return;
                }
                let key = key_entry.text().to_string();
                let value = val_entry.text().to_string();
                if key.is_empty() {
                    return;
                }
                if key == "mimetype" || key == "_thumbnail_" {
                    return;
                }
                if let Some(w) = weak.upgrade() {
                    let mut store_ref = w.store_mut();
                    if let Some(store) = store_ref.as_mut() {
                        let _ = store.metadata_set(&path, &key, &value);
                    }
                    drop(store_ref);
                    update_side_panel();
                }
            });
        });
    }

    // Add tag button
    {
        let weak_window = window.downgrade();
        let grid_selection = grid_selection.clone();
        let grid_store = grid_store.clone();
        let right_clicked_path = right_clicked_path.clone();
        let update_side_panel = update_side_panel.clone();
        ip.add_tag_btn.connect_clicked(move |_| {
            let Some(window) = weak_window.upgrade() else {
                return;
            };

            let mut selected_path: Option<String> = None;
            let bitset = grid_selection.selection();
            if bitset.size() > 0 {
                let idx = bitset.nth(0);
                if let Some(obj) = grid_store.item(idx).and_downcast::<glib::BoxedAnyObject>() {
                    selected_path = Some(obj.borrow::<StoreEntry>().path.clone());
                }
            }
            if selected_path.is_none() {
                selected_path = right_clicked_path.borrow().clone();
            }
            let Some(path) = selected_path else { return };

            let dialog = adw::AlertDialog::builder()
                .heading(&gettext("Add Tag"))
                .build();
            dialog.add_response("cancel", &gettext("Cancel"));
            dialog.add_response("add", &gettext("Add"));
            dialog.set_default_response(Some("add"));
            dialog.set_response_appearance("add", adw::ResponseAppearance::Suggested);
            dialog.set_close_response("cancel");

            let tag_entry = Entry::new();
            tag_entry.set_placeholder_text(Some(&gettext("Tag name")));
            dialog.set_extra_child(Some(&tag_entry));

            let weak = window.downgrade();
            let update_side_panel = update_side_panel.clone();
            dialog.choose(Some(&window), None::<&gio::Cancellable>, move |resp| {
                if resp != "add" {
                    return;
                }
                let tag = tag_entry.text().to_string();
                if tag.is_empty() {
                    return;
                }
                if let Some(w) = weak.upgrade() {
                    let mut store_ref = w.store_mut();
                    if let Some(store) = store_ref.as_mut() {
                        let _ = store.tag_add(&path, &tag);
                    }
                    drop(store_ref);
                    update_side_panel();
                }
            });
        });
    }
}

/// Connects the grid's selection-changed signal to automatically refresh the info pane.
pub(crate) fn wire_grid_selection_to_panel(
    grid_selection: &MultiSelection,
    update_side_panel: &Rc<dyn Fn()>,
) {
    let update_side_panel = update_side_panel.clone();
    grid_selection.connect_selection_changed(move |_, _, _| {
        update_side_panel();
    });
}
