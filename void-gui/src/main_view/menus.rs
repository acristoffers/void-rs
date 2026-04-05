/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use libadwaita as adw;

use adw::gio::{self, Menu, MenuItem};
use adw::gtk::{
    self, gdk, Align, Box, Button, GestureClick, GridView, Label, ListView, MenuButton,
    Orientation, Stack,
};
use adw::prelude::*;

use super::grid::populate_grid;
use super::utils::*;
use crate::i18n::gettext;
use crate::window::VoidWindow;

// ── Menu construction ───────────────────────────────────────────────────────────

/// Collected widget references produced by [`build_menus`] for the hamburger and context menus.
pub(crate) struct MenuWidgets {
    /// The hamburger menu button shown in the header bar.
    pub menu_button: MenuButton,
    /// Menu model for right-clicking on empty grid background.
    pub bg_menu: Menu,
    /// Menu model for right-clicking on a specific grid item.
    pub item_menu: Menu,
    /// Button that decreases icon size in the hamburger menu.
    pub minus_btn: Button,
    /// Button that increases icon size in the hamburger menu.
    pub plus_btn: Button,
}

/// Builds the hamburger menu model, background context menu, item context menu,
/// and the icon-size +/− custom widget row.
///
/// Returns a [`MenuWidgets`] bundle with the `MenuButton` and context menu models.
pub(crate) fn build_menus(saved_size: i32) -> MenuWidgets {
    let new_window_section = Menu::new();
    new_window_section.append(Some(&gettext("New Window")), Some("app.new-window"));

    let import_section = Menu::new();
    import_section.append(Some(&gettext("Import File…")), Some("win.import-file"));
    import_section.append(Some(&gettext("Import Folder…")), Some("win.import-folder"));
    let create_folder_item =
        MenuItem::new(Some(&gettext("Create Folder…")), Some("win.create-folder"));
    create_folder_item.set_attribute_value("accel", Some(&"<Control>n".to_variant()));
    import_section.append_item(&create_folder_item);

    let view_section = Menu::new();
    let size_item = MenuItem::new(None, None);
    size_item.set_attribute_value("custom", Some(&"icon-size".to_variant()));
    view_section.append_item(&size_item);
    view_section.append(Some(&gettext("A → Z")), Some("win.sort::az"));
    view_section.append(Some(&gettext("Z → A")), Some("win.sort::za"));

    let settings_section = Menu::new();
    settings_section.append(Some(&gettext("Settings")), Some("win.settings"));
    settings_section.append(
        Some(&gettext("Change Password…")),
        Some("win.change-password"),
    );

    let help_section = Menu::new();
    help_section.append(Some(&gettext("Keyboard Shortcuts")), Some("win.shortcuts"));
    help_section.append(Some(&gettext("About Void")), Some("win.about"));

    let menu_model = Menu::new();
    menu_model.append_section(None, &new_window_section);
    menu_model.append_section(None, &import_section);
    menu_model.append_section(None, &view_section);
    menu_model.append_section(None, &settings_section);
    menu_model.append_section(None, &help_section);

    // Context menus
    let bg_menu = Menu::new();
    bg_menu.append(Some(&gettext("Import File…")), Some("win.import-file"));
    bg_menu.append(Some(&gettext("Import Folder…")), Some("win.import-folder"));
    let create_folder_item =
        MenuItem::new(Some(&gettext("Create Folder…")), Some("win.create-folder"));
    create_folder_item.set_attribute_value("accel", Some(&"<Control>n".to_variant()));
    bg_menu.append_item(&create_folder_item);
    bg_menu.append(Some(&gettext("Export Folder…")), Some("win.export-folder"));
    bg_menu.append(Some(&gettext("Paste")), Some("win.paste"));

    let item_menu = Menu::new();
    item_menu.append(Some(&gettext("Open")), Some("win.item-open"));
    item_menu.append(Some(&gettext("Rename…")), Some("win.item-rename"));
    item_menu.append(Some(&gettext("Export…")), Some("win.item-export"));
    item_menu.append(Some(&gettext("Copy")), Some("win.item-copy"));
    item_menu.append(Some(&gettext("Cut")), Some("win.item-cut"));
    item_menu.append(Some(&gettext("Delete")), Some("win.item-delete"));

    // Icon-size custom widget
    let size_label = Label::new(Some(&gettext("Icon Size")));
    size_label.set_halign(Align::Start);
    size_label.set_hexpand(true);

    let minus_btn = Button::from_icon_name("list-remove-symbolic");
    let plus_btn = Button::from_icon_name("list-add-symbolic");
    minus_btn.add_css_class("flat");
    plus_btn.add_css_class("flat");
    minus_btn.set_sensitive(saved_size > ICON_SIZES[0]);
    plus_btn.set_sensitive(saved_size < *ICON_SIZES.last().unwrap());

    let btn_box = Box::new(Orientation::Horizontal, 0);
    btn_box.add_css_class("linked");
    btn_box.append(&minus_btn);
    btn_box.append(&plus_btn);

    let size_row = Box::new(Orientation::Horizontal, 12);
    size_row.append(&size_label);
    size_row.append(&btn_box);
    size_row.set_margin_start(12);
    size_row.set_margin_end(12);
    size_row.set_margin_top(6);
    size_row.set_margin_bottom(6);

    let popover = gtk::PopoverMenu::from_model(Some(&menu_model));
    popover.add_child(&size_row, "icon-size");

    let menu_button = MenuButton::builder()
        .icon_name("open-menu-symbolic")
        .popover(&popover)
        .build();

    MenuWidgets {
        menu_button,
        bg_menu,
        item_menu,
        minus_btn,
        plus_btn,
    }
}

// ── Context menu popovers ───────────────────────────────────────────────────────

/// Attaches right-click context menus to the grid view and the empty-folder
/// placeholder. Picks between the item menu and background menu based on
/// whether the click lands on a grid cell.
pub(crate) fn setup_context_menus(
    grid_view: &GridView,
    grid_stack: &Stack,
    bg_menu: &Menu,
    item_menu: &Menu,
    right_clicked_path: &Rc<RefCell<Option<String>>>,
) {
    let bg_popover = gtk::PopoverMenu::from_model(Some(bg_menu));
    bg_popover.set_parent(grid_view);
    bg_popover.set_has_arrow(false);

    let item_popover = gtk::PopoverMenu::from_model(Some(item_menu));
    item_popover.set_parent(grid_view);
    item_popover.set_has_arrow(false);

    let grid_view_ref = grid_view.clone();
    let right_clicked_path_grid = right_clicked_path.clone();
    let right_clicked_path_empty = right_clicked_path.clone();
    let gesture = GestureClick::new();
    gesture.set_button(3);
    gesture.connect_released(move |_, _, x, y| {
        let rect = gdk::Rectangle::new(x as i32, y as i32, 1, 1);

        if let Some(picked) = grid_view_ref.pick(x, y, gtk::PickFlags::DEFAULT) {
            // Walk up the widget tree from the innermost widget at (x, y)
            // looking for one whose name starts with '/' — that name is the
            // vault path assigned to the grid cell in the factory's bind step.
            let mut widget = picked;
            let mut found_path = None;
            loop {
                let name = widget.widget_name();
                // Grid cell widgets are named with their vault path (e.g. "/photos/cat.jpg").
                if name.starts_with('/') {
                    found_path = Some(name.to_string());
                    break;
                }
                // Stop if we've walked all the way up to the GridView itself.
                if widget == grid_view_ref.clone().upcast::<gtk::Widget>() {
                    break;
                }
                match widget.parent() {
                    Some(p) => widget = p,
                    None => break,
                }
            }

            if let Some(path) = found_path {
                *right_clicked_path_grid.borrow_mut() = Some(path);
                item_popover.set_pointing_to(Some(&rect));
                item_popover.popup();
            } else {
                *right_clicked_path_grid.borrow_mut() = None;
                bg_popover.set_pointing_to(Some(&rect));
                bg_popover.popup();
            }
        } else {
            *right_clicked_path_grid.borrow_mut() = None;
            bg_popover.set_pointing_to(Some(&rect));
            bg_popover.popup();
        }
    });
    grid_view.add_controller(gesture);

    // Context menu on the empty-folder placeholder
    if let Some(empty_page) = grid_stack.child_by_name("empty") {
        let empty_popover = gtk::PopoverMenu::from_model(Some(bg_menu));
        empty_popover.set_parent(&empty_page);
        empty_popover.set_has_arrow(false);

        let gesture = GestureClick::new();
        gesture.set_button(3);
        gesture.connect_released(move |_, _, x, y| {
            *right_clicked_path_empty.borrow_mut() = None;
            let rect = gdk::Rectangle::new(x as i32, y as i32, 1, 1);
            empty_popover.set_pointing_to(Some(&rect));
            empty_popover.popup();
        });
        empty_page.add_controller(gesture);
    }
}

// ── Tree-view context menu ──────────────────────────────────────────────────────

/// Attaches a right-click context menu to the folder tree sidebar.
pub(crate) fn setup_tree_context_menu(
    tree_view: &ListView,
    right_clicked_path: &Rc<RefCell<Option<String>>>,
) {
    let tree_menu = Menu::new();
    let create_folder_item =
        MenuItem::new(Some(&gettext("Create Folder…")), Some("win.create-folder"));
    create_folder_item.set_attribute_value("accel", Some(&"<Control>n".to_variant()));
    tree_menu.append_item(&create_folder_item);
    tree_menu.append(Some(&gettext("Copy")), Some("win.item-copy"));
    tree_menu.append(Some(&gettext("Cut")), Some("win.item-cut"));
    tree_menu.append(Some(&gettext("Paste")), Some("win.paste"));
    tree_menu.append(Some(&gettext("Delete")), Some("win.item-delete"));

    let popover = gtk::PopoverMenu::from_model(Some(&tree_menu));
    popover.set_parent(tree_view);
    popover.set_has_arrow(false);

    let tree_view_ref = tree_view.clone();
    let right_clicked_path = right_clicked_path.clone();
    let gesture = GestureClick::new();
    gesture.set_button(3);
    gesture.connect_released(move |_, _, x, y| {
        let mut folder_path = None;
        if let Some(picked) = tree_view_ref.pick(x, y, gtk::PickFlags::DEFAULT) {
            let mut widget = picked;
            loop {
                let name = widget.widget_name();
                if name.starts_with('/') {
                    folder_path = Some(name.to_string());
                    break;
                }
                if widget == tree_view_ref.clone().upcast::<gtk::Widget>() {
                    break;
                }
                match widget.parent() {
                    Some(p) => widget = p,
                    None => break,
                }
            }
        }

        if let Some(path) = folder_path {
            *right_clicked_path.borrow_mut() = Some(path);
            let rect = gdk::Rectangle::new(x as i32, y as i32, 1, 1);
            popover.set_pointing_to(Some(&rect));
            popover.popup();
        }
    });
    tree_view.add_controller(gesture);
}

/// Connects the icon-size +/− buttons so they cycle through [`ICON_SIZES`],
/// persist the choice to GSettings, and refresh the grid.
pub(crate) fn wire_icon_size(
    window: &VoidWindow,
    minus_btn: &Button,
    plus_btn: &Button,
    icon_size: &Rc<Cell<i32>>,
    grid_store: &gio::ListStore,
    current_path: &Rc<RefCell<String>>,
    reverse_sort: &Rc<Cell<bool>>,
    grid_generation: &Rc<Cell<u64>>,
    settings: &gio::Settings,
) {
    {
        let icon_size = icon_size.clone();
        let grid_store = grid_store.clone();
        let weak_window = window.downgrade();
        let current_path = current_path.clone();
        let reverse_sort = reverse_sort.clone();
        let plus_btn = plus_btn.clone();
        let settings = settings.clone();
        let grid_generation = grid_generation.clone();
        minus_btn.connect_clicked(move |btn| {
            let idx = ICON_SIZES
                .iter()
                .position(|&s| s == icon_size.get())
                .unwrap_or(1);
            if idx > 0 {
                let new_size = ICON_SIZES[idx - 1];
                icon_size.set(new_size);
                let _ = settings.set_int("icon-size", new_size);
                btn.set_sensitive(idx - 1 > 0);
                plus_btn.set_sensitive(true);
                if let Some(window) = weak_window.upgrade() {
                    populate_grid(
                        &grid_store,
                        &window,
                        &current_path.borrow(),
                        reverse_sort.get(),
                        &grid_generation,
                    );
                }
            }
        });
    }

    {
        let icon_size = icon_size.clone();
        let grid_store = grid_store.clone();
        let weak_window = window.downgrade();
        let current_path = current_path.clone();
        let reverse_sort = reverse_sort.clone();
        let minus_btn = minus_btn.clone();
        let settings = settings.clone();
        let grid_generation = grid_generation.clone();
        plus_btn.connect_clicked(move |btn| {
            let idx = ICON_SIZES
                .iter()
                .position(|&s| s == icon_size.get())
                .unwrap_or(1);
            if idx < ICON_SIZES.len() - 1 {
                let new_size = ICON_SIZES[idx + 1];
                icon_size.set(new_size);
                let _ = settings.set_int("icon-size", new_size);
                btn.set_sensitive(idx + 1 < ICON_SIZES.len() - 1);
                minus_btn.set_sensitive(true);
                if let Some(window) = weak_window.upgrade() {
                    populate_grid(
                        &grid_store,
                        &window,
                        &current_path.borrow(),
                        reverse_sort.get(),
                        &grid_generation,
                    );
                }
            }
        });
    }
}
