/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

mod dnd;
mod file_actions;
mod grid;
mod header;
mod item_actions;
mod menus;
mod sidebar;
mod utils;

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use libadwaita as adw;

use adw::gio::{self, SimpleAction};
use adw::glib;
use adw::gtk;
use adw::gtk::TreeListRow;
use adw::prelude::*;
use adw::{NavigationPage, OverlaySplitView};

use crate::window::VoidWindow;
use grid::{folder_model, populate_grid};
use utils::StoreEntry;
use utils::*;

/// Builds the main vault browser page shown after a store is opened.
///
/// Assembles the header bar, file grid, folder tree sidebar, and info pane,
/// then wires up navigation, search/filter, drag-and-drop, and all file/item
/// actions. Returns the completed [`NavigationPage`] tagged `"main"`.
pub fn main_page(window: &VoidWindow) -> NavigationPage {
    // ── GSettings ───────────────────────────────────────────────────────────
    let settings = gio::Settings::new("me.acristoffers.void");

    // Restore persisted icon-size, falling back to 48 if the value is invalid.
    let saved_size = settings.int("icon-size");
    let saved_size = if ICON_SIZES.contains(&saved_size) {
        saved_size
    } else {
        48
    };
    let saved_sort = settings.string("sort-order");
    let saved_reverse = saved_sort.as_str() == "za";

    // ── Shared state ────────────────────────────────────────────────────────
    let icon_size: Rc<Cell<i32>> = Rc::new(Cell::new(saved_size));
    let reverse_sort: Rc<Cell<bool>> = Rc::new(Cell::new(saved_reverse));
    let current_path: Rc<RefCell<String>> = Rc::new(RefCell::new("/".to_string()));
    let grid_generation: Rc<Cell<u64>> = Rc::new(Cell::new(0));
    let clipboard: Rc<RefCell<ClipboardState>> = Rc::new(RefCell::new(ClipboardState::default()));
    let right_clicked_path: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    let search_mode: Rc<Cell<u8>> = Rc::new(Cell::new(SEARCH_MODE_NONE));

    // ── Actions ─────────────────────────────────────────────────────────────
    let sort_action = SimpleAction::new_stateful(
        "sort",
        Some(&String::static_variant_type()),
        &saved_sort.to_variant(),
    );
    window.add_action(&sort_action);

    let edit_path_action = SimpleAction::new("edit-path", None);
    window.add_action(&edit_path_action);

    let filter_action = SimpleAction::new("filter-view", None);
    window.add_action(&filter_action);
    let search_action = SimpleAction::new("search-view", None);
    window.add_action(&search_action);

    // ── Build UI components ─────────────────────────────────────────────────
    let menu_w = menus::build_menus(saved_size);
    let hw = header::build_header(&menu_w.menu_button);
    let gw = grid::build_grid(&icon_size, &clipboard);
    let tw = sidebar::build_folder_tree(window);
    let ip = sidebar::build_info_pane();

    // ── Navigate action ─────────────────────────────────────────────────────
    // Central navigation handler: updates the grid, breadcrumbs, path bar,
    // and syncs the sidebar tree selection whenever the user changes folder.
    let navigate_action = SimpleAction::new("navigate", Some(&String::static_variant_type()));
    window.add_action(&navigate_action);

    {
        let grid_store = gw.grid_store.clone();
        let current_path = current_path.clone();
        let reverse_sort = reverse_sort.clone();
        let weak_window = window.downgrade();
        let breadcrumb_box = hw.breadcrumb_box.clone();
        let path_entry = hw.path_entry.clone();
        let path_stack = hw.path_stack.clone();
        let grid_generation = grid_generation.clone();
        let search_mode = search_mode.clone();
        let grid_selection = gw.grid_selection.clone();
        let tree_selection = tw.tree_selection.clone();
        // Guard flag prevents navigate ↔ tree-selection infinite loops.
        let syncing_tree = Rc::new(Cell::new(false));
        let syncing_tree2 = syncing_tree.clone();
        navigate_action.connect_activate(move |_, param| {
            let Some(value) = param else { return };
            let Some(path) = value.get::<String>() else {
                return;
            };
            search_mode.set(SEARCH_MODE_NONE);
            *current_path.borrow_mut() = path.clone();
            grid_selection.unselect_all();
            if let Some(window) = weak_window.upgrade() {
                populate_grid(
                    &grid_store,
                    &window,
                    &path,
                    reverse_sort.get(),
                    &grid_generation,
                );
            }
            path_entry.set_text(&path);
            path_stack.set_visible_child_name("breadcrumbs");
            rebuild_breadcrumbs(&breadcrumb_box, &path);
            if !syncing_tree.get() {
                syncing_tree.set(true);
                sidebar::sync_tree_selection(&tree_selection, &path);
                syncing_tree.set(false);
            }
        });

        // Wire tree selection → navigate (with guard to prevent loops)
        {
            let navigate_action = navigate_action.clone();
            tw.tree_selection
                .connect_selection_changed(move |sel, _, _| {
                    if syncing_tree2.get() {
                        return;
                    }
                    let path = sel
                        .selected_item()
                        .and_downcast::<TreeListRow>()
                        .and_then(|row| row.item())
                        .and_downcast::<glib::BoxedAnyObject>()
                        .map(|obj| {
                            let entry = obj.borrow::<StoreEntry>();
                            entry.path.clone()
                        })
                        .unwrap_or_else(|| "/".to_string());
                    navigate_action.activate(Some(&path.to_variant()));
                });
        }

        // Start with no selection (root "/" is not in the tree)
        tw.tree_selection.set_selected(gtk::INVALID_LIST_POSITION);
    }

    // ── Wire path bar, search, grid ─────────────────────────────────────────
    header::wire_path_bar(
        window,
        &hw,
        &current_path,
        &edit_path_action,
        &navigate_action,
    );
    header::wire_search(
        window,
        &hw,
        &gw.grid_store,
        &current_path,
        &reverse_sort,
        &grid_generation,
        &search_mode,
        &filter_action,
        &search_action,
    );
    grid::wire_grid_activation(&gw.grid_view, &gw.grid_store, &navigate_action, window);
    grid::wire_empty_folder_toggle(&gw.grid_store, &gw.grid_stack);
    menus::setup_context_menus(
        &gw.grid_view,
        &gw.grid_stack,
        &menu_w.bg_menu,
        &menu_w.item_menu,
        &right_clicked_path,
    );
    menus::setup_tree_context_menu(&tw.tree_view, &right_clicked_path);

    // ── Refresh helpers ─────────────────────────────────────────────────────
    // Closures shared across action handlers to re-populate the grid and tree
    // after any store mutation (import, paste, rename, delete, etc.).
    let refresh_grid: Rc<dyn Fn()> = {
        let grid_store = gw.grid_store.clone();
        let weak_window = window.downgrade();
        let current_path = current_path.clone();
        let reverse_sort = reverse_sort.clone();
        let grid_generation = grid_generation.clone();
        let search_mode = search_mode.clone();
        let path_stack = hw.path_stack.clone();
        Rc::new(move || {
            if let Some(window) = weak_window.upgrade() {
                search_mode.set(SEARCH_MODE_NONE);
                path_stack.set_visible_child_name("breadcrumbs");
                populate_grid(
                    &grid_store,
                    &window,
                    &current_path.borrow(),
                    reverse_sort.get(),
                    &grid_generation,
                );
            }
        })
    };

    let refresh_tree: Rc<dyn Fn()> = {
        let tree_root = tw.tree_root.clone();
        let weak_window = window.downgrade();
        Rc::new(move || {
            if let Some(window) = weak_window.upgrade() {
                let fresh = folder_model(&window, "/");
                tree_root.remove_all();
                for i in 0..fresh.n_items() {
                    if let Some(obj) = fresh.item(i) {
                        tree_root.append(&obj);
                    }
                }
            }
        })
    };

    let refresh: Rc<dyn Fn()> = {
        let refresh_grid = refresh_grid.clone();
        let refresh_tree = refresh_tree.clone();
        Rc::new(move || {
            refresh_grid();
            refresh_tree();
        })
    };

    // ── Register actions ────────────────────────────────────────────────────
    file_actions::setup(
        window,
        &current_path,
        &right_clicked_path,
        &refresh,
        &hw.busy_box,
        &hw.busy_spinner,
        &hw.import_progress,
        &hw.import_label,
        &clipboard,
    );
    item_actions::setup(
        window,
        &gw.grid_store,
        &gw.grid_selection,
        &right_clicked_path,
        &current_path,
        &navigate_action,
        &refresh,
        &clipboard,
        &hw.busy_box,
        &hw.busy_spinner,
    );
    dnd::setup(
        window,
        &gw.grid_view,
        &gw.grid_scroll,
        &gw.grid_stack,
        &gw.grid_store,
        &gw.grid_selection,
        &current_path,
        &refresh,
        &hw.busy_box,
        &hw.busy_spinner,
        &hw.import_progress,
        &hw.import_label,
    );

    // ── Info pane wiring ────────────────────────────────────────────────────
    let update_side_panel = sidebar::build_update_side_panel(
        window,
        &ip,
        &gw.grid_store,
        &gw.grid_selection,
        &right_clicked_path,
    );
    sidebar::wire_info_pane_buttons(
        window,
        &ip,
        &gw.grid_store,
        &gw.grid_selection,
        &right_clicked_path,
        &update_side_panel,
    );
    sidebar::wire_grid_selection_to_panel(&gw.grid_selection, &update_side_panel);

    // ── Layout ──────────────────────────────────────────────────────────────
    // inner_split: file grid + right-side info pane.
    // outer_split: inner_split + left-side folder tree sidebar.
    let inner_split = OverlaySplitView::builder()
        .content(&gw.grid_stack)
        .sidebar(&ip.info_pane)
        .sidebar_position(adw::gtk::PackType::End)
        .show_sidebar(false)
        .build();

    let outer_split = OverlaySplitView::builder()
        .content(&inner_split)
        .sidebar(&tw.tree_scroll)
        .min_sidebar_width(180.0)
        .sidebar_width_fraction(0.2)
        .build();

    hw.info_button
        .bind_property("active", &inner_split, "show-sidebar")
        .sync_create()
        .bidirectional()
        .build();

    // ── Wire icon-size +/- ──────────────────────────────────────────────────
    menus::wire_icon_size(
        window,
        &menu_w.minus_btn,
        &menu_w.plus_btn,
        &icon_size,
        &gw.grid_store,
        &current_path,
        &reverse_sort,
        &grid_generation,
        &settings,
    );

    // ── Wire sort action ────────────────────────────────────────────────────
    {
        let reverse_sort = reverse_sort.clone();
        let grid_store = gw.grid_store.clone();
        let weak_window = window.downgrade();
        let current_path = current_path.clone();
        let settings = settings.clone();
        let grid_generation = grid_generation.clone();
        sort_action.connect_activate(move |action, param| {
            if let Some(value) = param {
                action.set_state(value);
                let sort_str = value.get::<String>().unwrap_or_else(|| "az".to_string());
                let is_reverse = sort_str == "za";
                reverse_sort.set(is_reverse);
                let _ = settings.set_string("sort-order", &sort_str);
                if let Some(window) = weak_window.upgrade() {
                    populate_grid(
                        &grid_store,
                        &window,
                        &current_path.borrow(),
                        is_reverse,
                        &grid_generation,
                    );
                }
            }
        });
    }

    // ── Initial navigation ──────────────────────────────────────────────────
    navigate_action.activate(Some(&"/".to_variant()));

    // ── Page ────────────────────────────────────────────────────────────────
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&hw.header_bar);
    toolbar.set_content(Some(&outer_split));

    NavigationPage::builder()
        .tag("main")
        .title("Void")
        .child(&toolbar)
        .build()
}
