/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use adw::gio::SimpleAction;
use adw::glib;
use adw::prelude::*;
use adw::subclass::prelude::ObjectSubclassIsExt;
use adw::NavigationView;
use libadwaita as adw;

use std::cell::{Ref, RefMut};

use crate::application::VoidApplication;
use crate::i18n::gettext;

mod imp {
    use adw::glib;
    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use adw::NavigationView;
    use libadwaita as adw;
    use std::cell::{OnceCell, RefCell};

    #[derive(Default)]
    pub struct VoidWindow {
        pub(super) nav: OnceCell<NavigationView>,
        pub(super) store: RefCell<Option<void::Store>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for VoidWindow {
        const NAME: &'static str = "VoidWindow";
        type Type = super::VoidWindow;
        type ParentType = adw::ApplicationWindow;
    }

    impl ObjectImpl for VoidWindow {
        /// Called after the window GObject is constructed; sets the title, default size,
        /// creates the [`NavigationView`], and pushes the login page.
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.set_title(Some("Void"));
            obj.set_default_size(800, 600);

            let nav = NavigationView::new();
            self.nav
                .set(nav)
                .expect("NavigationView already initialized");
            let nav = self.nav.get().unwrap();

            obj.setup_actions();
            nav.add(&crate::pages::login_page(&obj));
            obj.set_content(Some(nav));
        }
    }

    impl WidgetImpl for VoidWindow {}
    impl WindowImpl for VoidWindow {}
    impl ApplicationWindowImpl for VoidWindow {}
    impl AdwApplicationWindowImpl for VoidWindow {}
}

glib::wrapper! {
    pub struct VoidWindow(ObjectSubclass<imp::VoidWindow>)
        @extends adw::ApplicationWindow, adw::gtk::ApplicationWindow,
                 adw::gtk::Window, adw::gtk::Widget,
        @implements adw::gio::ActionGroup, adw::gio::ActionMap,
                    adw::gtk::Accessible, adw::gtk::Buildable,
                    adw::gtk::ConstraintTarget, adw::gtk::Native,
                    adw::gtk::Root, adw::gtk::ShortcutManager;
}

impl VoidWindow {
    /// Creates a new application window and associates it with the given `app`.
    pub fn new(app: &VoidApplication) -> Self {
        glib::Object::builder().property("application", app).build()
    }

    /// Returns a reference to the window's [`NavigationView`].
    pub fn nav(&self) -> &NavigationView {
        self.imp()
            .nav
            .get()
            .expect("NavigationView not initialized")
    }

    /// Enables or disables a named `SimpleAction` registered on this window.
    pub fn set_action_enabled(&self, name: &str, enabled: bool) {
        if let Some(action) = self.lookup_action(name) {
            if let Some(action) = action.downcast_ref::<SimpleAction>() {
                action.set_enabled(enabled);
            }
        }
    }

    /// Replaces the window's vault store with `store`.
    pub fn set_store(&self, store: void::Store) {
        self.imp().store.replace(Some(store));
    }

    /// Borrows the window's vault store immutably.
    pub fn store(&self) -> Ref<'_, Option<void::Store>> {
        self.imp().store.borrow()
    }

    /// Borrows the window's vault store mutably.
    pub fn store_mut(&self) -> RefMut<'_, Option<void::Store>> {
        self.imp().store.borrow_mut()
    }

    /// Registers all top-level window actions (open, create, settings, shortcuts, help, about).
    fn setup_actions(&self) {
        // win.open
        let open_action = SimpleAction::new("open", None);
        let weak_self = self.downgrade();
        open_action.connect_activate(move |_, _| {
            if let Some(window) = weak_self.upgrade() {
                window.nav().push(&crate::pages::open_page(&window));
            }
        });
        self.add_action(&open_action);

        // win.create
        let create_action = SimpleAction::new("create", None);
        let weak_self = self.downgrade();
        create_action.connect_activate(move |_, _| {
            if let Some(window) = weak_self.upgrade() {
                match window.nav().visible_page().and_then(|p| p.tag()).as_deref() {
                    Some("main") => {
                        if let Some(a) = window.lookup_action("create-folder") {
                            a.activate(None);
                        }
                    }
                    _ => window.nav().push(&crate::pages::create_page(&window)),
                }
            }
        });
        self.add_action(&create_action);

        // win.settings
        let settings_action = SimpleAction::new("settings", None);
        let weak_self = self.downgrade();
        settings_action.connect_activate(move |_, _| {
            if let Some(window) = weak_self.upgrade() {
                crate::settings::settings_window(Some(window.upcast_ref())).present();
            }
        });
        self.add_action(&settings_action);

        // win.shortcuts
        let shortcuts_action = SimpleAction::new("shortcuts", None);
        let weak_self = self.downgrade();
        shortcuts_action.connect_activate(move |_, _| {
            if let Some(window) = weak_self.upgrade() {
                match window.nav().visible_page().and_then(|p| p.tag()).as_deref() {
                    Some("login") | Some("open") | Some("create") => {
                        login_shortcuts(&window);
                    }
                    Some("main") => main_shortcuts(&window),
                    _ => {}
                }
            }
        });
        self.add_action(&shortcuts_action);

        // win.help
        let help_action = SimpleAction::new("help", None);
        help_action.connect_activate(|_, _| {});
        self.add_action(&help_action);

        // win.about
        let about_action = SimpleAction::new("about", None);
        let weak_self = self.downgrade();
        about_action.connect_activate(move |_, _| {
            if let Some(window) = weak_self.upgrade() {
                adw::AboutDialog::builder()
                    .application_icon("me.acristoffers.void")
                    .application_name("Void")
                    .comments(&gettext(
                        "An encrypted file store with a filesystem-like structure.",
                    ))
                    .copyright("© Álan Crístoffer e Sousa")
                    .developer_name("Álan Crístoffer")
                    .developers(["Álan Crístoffer <acristoffers@startmail.com>"])
                    .issue_url("https://github.com/acristoffers/void-rs/issues")
                    .license_type(adw::gtk::License::Mpl20)
                    .version(env!("CARGO_PKG_VERSION"))
                    .website("https://github.com/acristoffers/void-rs")
                    .build()
                    .present(Some(&window));
            }
        });
        self.add_action(&about_action);
    }
}

/// A single keyboard shortcut entry displayed in the shortcuts dialog.
struct ShortcutBinding<'a> {
    /// Human-readable description of the action (e.g. "Open Vault").
    label: &'a str,
    /// Key names rendered as keycaps (e.g. `&["Ctrl", "O"]`).
    keys: &'a [&'a str],
}

/// A named group of keyboard shortcuts shown as a section in the shortcuts dialog.
struct ShortcutGroup<'a> {
    /// Section heading (e.g. "General", "File Operations").
    title: &'a str,
    /// The shortcut entries belonging to this group.
    bindings: &'a [ShortcutBinding<'a>],
}

/// Builds a single `ActionRow` displaying a shortcut binding's label and keycap widgets.
fn shortcut_row(binding: &ShortcutBinding) -> adw::ActionRow {
    use adw::gtk;

    let keys_box = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    keys_box.set_valign(gtk::Align::Center);

    for key in binding.keys {
        let label = gtk::Label::new(Some(key));
        label.add_css_class("keycap");
        keys_box.append(&label);
    }

    let row = adw::ActionRow::builder()
        .title(binding.label)
        .activatable(false)
        .build();
    row.add_suffix(&keys_box);
    row
}

/// Presents a dialog listing keyboard shortcut groups, each rendered as a `PreferencesGroup`.
fn shortcuts_dialog(window: &VoidWindow, groups: &[ShortcutGroup]) {
    use adw::prelude::*;

    let content_box = adw::gtk::Box::new(adw::gtk::Orientation::Vertical, 18);
    for group in groups {
        let section = adw::PreferencesGroup::builder().title(group.title).build();
        for binding in group.bindings {
            section.add(&shortcut_row(binding));
        }
        content_box.append(&section);
    }

    let clamp = adw::Clamp::builder()
        .maximum_size(400)
        .child(&content_box)
        .build();
    clamp.set_margin_top(24);
    clamp.set_margin_bottom(24);
    clamp.set_margin_start(12);
    clamp.set_margin_end(12);

    let page = adw::ToolbarView::new();
    page.add_top_bar(&adw::HeaderBar::new());
    page.set_content(Some(&clamp));

    let dialog = adw::Dialog::builder()
        .title(&gettext("Keyboard Shortcuts"))
        .child(&page)
        .content_width(450)
        .content_height(350)
        .build();
    dialog.present(Some(window));
}

/// Shows the keyboard shortcuts dialog with bindings available on the login/open/create pages.
fn login_shortcuts(window: &VoidWindow) {
    let general = gettext("General");
    let open = gettext("Open Vault");
    let create = gettext("Create Vault");
    let settings = gettext("Settings");
    let shortcuts = gettext("Keyboard Shortcuts");

    shortcuts_dialog(
        window,
        &[ShortcutGroup {
            title: &general,
            bindings: &[
                ShortcutBinding {
                    label: &open,
                    keys: &["Ctrl", "O"],
                },
                ShortcutBinding {
                    label: &create,
                    keys: &["Ctrl", "N"],
                },
                ShortcutBinding {
                    label: &settings,
                    keys: &["Ctrl", ","],
                },
                ShortcutBinding {
                    label: &shortcuts,
                    keys: &["Ctrl", "?"],
                },
            ],
        }],
    );
}

/// Shows the keyboard shortcuts dialog with bindings available on the main vault browser page.
fn main_shortcuts(window: &VoidWindow) {
    let general = gettext("General");
    let new_window = gettext("New Window");
    let create_folder = gettext("Create Folder");
    let settings = gettext("Settings");
    let shortcuts = gettext("Keyboard Shortcuts");
    let edit_path = gettext("Edit Path");
    let filter = gettext("Filter");
    let search = gettext("Search");
    let file_ops = gettext("File Operations");
    let copy = gettext("Copy");
    let cut = gettext("Cut");
    let paste = gettext("Paste");
    let export = gettext("Export");
    let import_file = gettext("Import File");
    let rename = gettext("Rename");
    let delete = gettext("Delete");

    shortcuts_dialog(
        window,
        &[
            ShortcutGroup {
                title: &general,
                bindings: &[
                    ShortcutBinding {
                        label: &new_window,
                        keys: &["Ctrl", "Shift", "N"],
                    },
                    ShortcutBinding {
                        label: &create_folder,
                        keys: &["Ctrl", "N"],
                    },
                    ShortcutBinding {
                        label: &edit_path,
                        keys: &["Ctrl", "L"],
                    },
                    ShortcutBinding {
                        label: &filter,
                        keys: &["Ctrl", "F"],
                    },
                    ShortcutBinding {
                        label: &search,
                        keys: &["Ctrl", "/"],
                    },
                    ShortcutBinding {
                        label: &settings,
                        keys: &["Ctrl", ","],
                    },
                    ShortcutBinding {
                        label: &shortcuts,
                        keys: &["Ctrl", "?"],
                    },
                ],
            },
            ShortcutGroup {
                title: &file_ops,
                bindings: &[
                    ShortcutBinding {
                        label: &copy,
                        keys: &["Ctrl", "C"],
                    },
                    ShortcutBinding {
                        label: &cut,
                        keys: &["Ctrl", "X"],
                    },
                    ShortcutBinding {
                        label: &paste,
                        keys: &["Ctrl", "V"],
                    },
                    ShortcutBinding {
                        label: &export,
                        keys: &["Ctrl", "E"],
                    },
                    ShortcutBinding {
                        label: &import_file,
                        keys: &["Ctrl", "I"],
                    },
                    ShortcutBinding {
                        label: &rename,
                        keys: &["F2"],
                    },
                    ShortcutBinding {
                        label: &delete,
                        keys: &["Delete"],
                    },
                ],
            },
        ],
    );
}
