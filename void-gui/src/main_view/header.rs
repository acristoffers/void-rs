/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use libadwaita as adw;

use adw::gio::{self, SimpleAction};
use adw::glib;
use adw::gtk::{
    self, gdk, Align, Box, Button, Entry, EventControllerKey, Label, MenuButton, Orientation,
    ProgressBar, ScrolledWindow, Spinner, Stack,
};
use adw::prelude::*;
use adw::HeaderBar;

use super::grid::{populate_grid, populate_grid_search};
use super::utils::*;
use crate::i18n::gettext;
use crate::window::VoidWindow;

// ── Header widgets ──────────────────────────────────────────────────────────────

/// Collected widget references produced by [`build_header`] for the main view's header bar.
pub(crate) struct HeaderWidgets {
    /// The top-level header bar container.
    pub header_bar: HeaderBar,
    /// Stack switching between breadcrumbs, path entry, and search row.
    pub path_stack: Stack,
    /// Horizontal box holding the clickable breadcrumb path segments.
    pub breadcrumb_box: Box,
    /// Text entry for typing a vault path directly (toggled by the edit button or `Ctrl+L`).
    pub path_entry: Entry,
    /// Text entry for search/filter queries.
    pub search_entry: Entry,
    /// Label showing "Filter:" or "Search:" next to the search entry.
    pub search_label: Label,
    /// Button that cancels the current search/filter and returns to breadcrumbs.
    pub search_cancel_btn: Button,
    /// Button that switches the path bar from breadcrumbs to the text entry.
    pub edit_btn: Button,
    /// Container box for the busy spinner and import progress widgets.
    pub busy_box: Box,
    /// Spinner displayed while a background operation is in progress.
    pub busy_spinner: Spinner,
    /// Progress bar showing import progress (bytes transferred).
    pub import_progress: ProgressBar,
    /// Label displaying the import status text (e.g. "3 MB / 10 MB").
    pub import_label: Label,
    /// Toggle button that shows or hides the right-side information pane.
    pub info_button: gtk::ToggleButton,
}

/// Constructs the main view header bar containing the breadcrumb path bar,
/// search/filter entry row, progress indicators, and action buttons.
///
/// Returns a [`HeaderWidgets`] bundle with references to all interactive elements.
pub(crate) fn build_header(menu_button: &MenuButton) -> HeaderWidgets {
    // Path bar
    let breadcrumb_box = Box::new(Orientation::Horizontal, 0);
    let path_entry = Entry::new();
    path_entry.set_hexpand(true);

    let breadcrumb_scroll = ScrolledWindow::builder()
        .child(&breadcrumb_box)
        .hscrollbar_policy(gtk::PolicyType::Automatic)
        .vscrollbar_policy(gtk::PolicyType::Never)
        .hexpand(true)
        .build();

    let edit_btn = Button::from_icon_name("document-edit-symbolic");
    edit_btn.add_css_class("flat");

    let breadcrumb_row = Box::new(Orientation::Horizontal, 0);
    breadcrumb_row.append(&breadcrumb_scroll);
    breadcrumb_row.append(&edit_btn);

    let path_stack = Stack::new();
    path_stack.set_hexpand(true);
    path_stack.set_transition_type(gtk::StackTransitionType::Crossfade);
    path_stack.add_named(&breadcrumb_row, Some("breadcrumbs"));
    path_stack.add_named(&path_entry, Some("entry"));

    // Search / Filter entry row
    let search_label = Label::new(None);
    search_label.add_css_class("dim-label");

    let search_entry = Entry::new();
    search_entry.set_hexpand(true);
    search_entry.set_placeholder_text(Some(&gettext("name or tag:value")));

    let search_cancel_btn = Button::from_icon_name("window-close-symbolic");
    search_cancel_btn.add_css_class("flat");
    search_cancel_btn.set_tooltip_text(Some(&gettext("Cancel")));

    let search_row = Box::new(Orientation::Horizontal, 6);
    search_row.set_hexpand(true);
    search_row.append(&search_label);
    search_row.append(&search_entry);
    search_row.append(&search_cancel_btn);

    path_stack.add_named(&search_row, Some("search"));
    path_stack.set_visible_child_name("breadcrumbs");

    let path_bar = Box::new(Orientation::Horizontal, 0);
    path_bar.add_css_class("pathbar");
    path_bar.set_hexpand(true);
    path_bar.append(&path_stack);

    // Busy / progress widgets
    let busy_spinner = Spinner::new();

    let import_progress = ProgressBar::new();
    import_progress.set_visible(false);
    import_progress.set_size_request(120, -1);
    import_progress.set_valign(Align::Center);

    let import_label = Label::new(None);
    import_label.set_visible(false);
    import_label.add_css_class("caption");

    let busy_box = Box::new(Orientation::Horizontal, 6);
    busy_box.set_visible(false);
    busy_box.append(&busy_spinner);
    busy_box.append(&import_progress);
    busy_box.append(&import_label);

    let info_button = gtk::ToggleButton::builder()
        .icon_name("sidebar-show-right-symbolic")
        .tooltip_text(&gettext("Information"))
        .build();

    // Assemble header bar
    let header_bar = HeaderBar::new();
    header_bar.set_title_widget(Some(&path_bar));
    header_bar.pack_end(menu_button);
    header_bar.pack_end(&info_button);
    header_bar.pack_end(&busy_box);

    HeaderWidgets {
        header_bar,
        path_stack,
        breadcrumb_box,
        path_entry,
        search_entry,
        search_label,
        search_cancel_btn,
        edit_btn,
        busy_box,
        busy_spinner,
        import_progress,
        import_label,
        info_button,
    }
}

// ── Path bar wiring ─────────────────────────────────────────────────────────────

/// Connects the path bar's edit button, `Ctrl+L` action, `Enter` to navigate,
/// and `Escape` to cancel, linking the breadcrumb display with the text entry.
pub(crate) fn wire_path_bar(
    window: &VoidWindow,
    header: &HeaderWidgets,
    current_path: &Rc<RefCell<String>>,
    edit_path_action: &SimpleAction,
    navigate_action: &SimpleAction,
) {
    // Edit button → switch to entry mode
    {
        let path_stack = header.path_stack.clone();
        let path_entry = header.path_entry.clone();
        let current_path = current_path.clone();
        header.edit_btn.connect_clicked(move |_| {
            path_entry.set_text(&current_path.borrow());
            path_stack.set_visible_child_name("entry");
            path_entry.grab_focus();
        });
    }

    // Ctrl+L action
    {
        let path_stack = header.path_stack.clone();
        let path_entry = header.path_entry.clone();
        let current_path = current_path.clone();
        edit_path_action.connect_activate(move |_, _| {
            path_entry.set_text(&current_path.borrow());
            path_stack.set_visible_child_name("entry");
            path_entry.grab_focus();
        });
    }

    // Enter on path entry → navigate
    {
        let weak_window = window.downgrade();
        let current_path = current_path.clone();
        let path_stack = header.path_stack.clone();
        let navigate_action = navigate_action.clone();
        header.path_entry.connect_activate(move |entry| {
            let text = entry.text().to_string();
            let path = if text.is_empty() {
                "/".to_string()
            } else {
                text
            };
            let valid = weak_window
                .upgrade()
                .and_then(|w| {
                    let s = w.store();
                    s.as_ref().map(|store| store.list(&path).is_ok())
                })
                .unwrap_or(false);
            if valid {
                navigate_action.activate(Some(&path.to_variant()));
            } else {
                entry.set_text(&current_path.borrow());
                path_stack.set_visible_child_name("breadcrumbs");
            }
        });
    }

    // Escape on path entry → revert
    {
        let path_stack = header.path_stack.clone();
        let current_path = current_path.clone();
        let path_entry = header.path_entry.clone();
        let key_ctrl = EventControllerKey::new();
        key_ctrl.connect_key_pressed(move |_, key, _, _| {
            if key == gdk::Key::Escape {
                path_entry.set_text(&current_path.borrow());
                path_stack.set_visible_child_name("breadcrumbs");
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
        header.path_entry.add_controller(key_ctrl);
    }
}

// ── Search / filter wiring ──────────────────────────────────────────────────────

/// Connects the search/filter entry to the grid, handling `Ctrl+F` (filter),
/// `Ctrl+/` (search), cancel/escape, and live query-as-you-type updates.
pub(crate) fn wire_search(
    window: &VoidWindow,
    header: &HeaderWidgets,
    grid_store: &gio::ListStore,
    current_path: &Rc<RefCell<String>>,
    reverse_sort: &Rc<Cell<bool>>,
    grid_generation: &Rc<Cell<u64>>,
    search_mode: &Rc<Cell<u8>>,
    filter_action: &SimpleAction,
    search_action: &SimpleAction,
) {
    // Ctrl+F → filter mode
    {
        let path_stack = header.path_stack.clone();
        let search_entry = header.search_entry.clone();
        let search_label = header.search_label.clone();
        let search_mode = search_mode.clone();
        filter_action.connect_activate(move |_, _| {
            search_mode.set(SEARCH_MODE_FILTER);
            search_label.set_text(&gettext("Filter:"));
            search_entry.set_text("");
            path_stack.set_visible_child_name("search");
            search_entry.grab_focus();
        });
    }

    // Ctrl+/ → search mode
    {
        let path_stack = header.path_stack.clone();
        let search_entry = header.search_entry.clone();
        let search_label = header.search_label.clone();
        let search_mode = search_mode.clone();
        search_action.connect_activate(move |_, _| {
            search_mode.set(SEARCH_MODE_SEARCH);
            search_label.set_text(&gettext("Search:"));
            search_entry.set_text("");
            path_stack.set_visible_child_name("search");
            search_entry.grab_focus();
        });
    }

    // Cancel button
    {
        let path_stack = header.path_stack.clone();
        let search_mode = search_mode.clone();
        let grid_store = grid_store.clone();
        let weak_window = window.downgrade();
        let current_path = current_path.clone();
        let reverse_sort = reverse_sort.clone();
        let grid_generation = grid_generation.clone();
        header.search_cancel_btn.connect_clicked(move |_| {
            search_mode.set(SEARCH_MODE_NONE);
            path_stack.set_visible_child_name("breadcrumbs");
            if let Some(window) = weak_window.upgrade() {
                populate_grid(
                    &grid_store,
                    &window,
                    &current_path.borrow(),
                    reverse_sort.get(),
                    &grid_generation,
                );
            }
        });
    }

    // Escape on search entry
    {
        let path_stack = header.path_stack.clone();
        let search_mode = search_mode.clone();
        let grid_store = grid_store.clone();
        let weak_window = window.downgrade();
        let current_path = current_path.clone();
        let reverse_sort = reverse_sort.clone();
        let grid_generation = grid_generation.clone();
        let key_ctrl = EventControllerKey::new();
        key_ctrl.connect_key_pressed(move |_, key, _, _| {
            if key == gdk::Key::Escape {
                search_mode.set(SEARCH_MODE_NONE);
                path_stack.set_visible_child_name("breadcrumbs");
                if let Some(window) = weak_window.upgrade() {
                    populate_grid(
                        &grid_store,
                        &window,
                        &current_path.borrow(),
                        reverse_sort.get(),
                        &grid_generation,
                    );
                }
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
        header.search_entry.add_controller(key_ctrl);
    }

    // Live filtering on text change
    {
        let grid_store = grid_store.clone();
        let weak_window = window.downgrade();
        let current_path = current_path.clone();
        let reverse_sort = reverse_sort.clone();
        let grid_generation = grid_generation.clone();
        let search_mode = search_mode.clone();
        header.search_entry.connect_changed(move |entry| {
            let Some(window) = weak_window.upgrade() else {
                return;
            };
            let query = entry.text().to_string();
            let mode = search_mode.get();
            if mode == SEARCH_MODE_NONE {
                return;
            }
            if query.is_empty() {
                grid_store.remove_all();
                return;
            }
            populate_grid_search(
                &grid_store,
                &window,
                &query,
                mode,
                &current_path.borrow(),
                reverse_sort.get(),
                &grid_generation,
            );
        });
    }
}
