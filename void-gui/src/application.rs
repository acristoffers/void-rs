/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use adw::glib;
use libadwaita as adw;

mod imp {
    use adw::glib;
    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use libadwaita as adw;

    #[derive(Default)]
    pub struct VoidApplication;

    #[glib::object_subclass]
    impl ObjectSubclass for VoidApplication {
        const NAME: &'static str = "VoidApplication";
        type Type = super::VoidApplication;
        type ParentType = adw::Application;
    }

    impl ObjectImpl for VoidApplication {}

    impl ApplicationImpl for VoidApplication {
        /// Called when the application is activated; creates and presents a new [`VoidWindow`].
        fn activate(&self) {
            self.parent_activate();
            let window = crate::window::VoidWindow::new(&*self.obj());
            window.present();
        }

        /// Called once at application startup; registers global keyboard accelerators,
        /// the new-window action, and loads the application stylesheet.
        fn startup(&self) {
            self.parent_startup();
            let app = self.obj();
            app.set_accels_for_action("win.open", &["<Control>o"]);
            app.set_accels_for_action("win.create", &["<Control>n"]);
            app.set_accels_for_action("win.settings", &["<Control>comma"]);
            app.set_accels_for_action("win.shortcuts", &["<Control>question"]);
            app.set_accels_for_action("app.new-window", &["<Control><Shift>n"]);
            app.set_accels_for_action("win.edit-path", &["<Control>l"]);
            app.set_accels_for_action("win.item-copy", &["<Control>c"]);
            app.set_accels_for_action("win.item-cut", &["<Control>x"]);
            app.set_accels_for_action("win.paste", &["<Control>v"]);
            app.set_accels_for_action("win.item-export", &["<Control>e"]);
            app.set_accels_for_action("win.import-file", &["<Control>i"]);
            app.set_accels_for_action("win.item-rename", &["F2"]);
            app.set_accels_for_action("win.item-delete", &["Delete"]);
            app.set_accels_for_action("win.filter-view", &["<Control>f"]);
            app.set_accels_for_action("win.search-view", &["<Control>slash"]);

            let new_window_action = adw::gio::SimpleAction::new("new-window", None);
            let weak_app = app.downgrade();
            new_window_action.connect_activate(move |_, _| {
                if let Some(app) = weak_app.upgrade() {
                    let window = crate::window::VoidWindow::new(&app);
                    window.present();
                }
            });
            app.add_action(&new_window_action);

            let Some(display) = adw::gdk::Display::default() else {
                eprintln!("Fatal: could not connect to a display server.");
                std::process::exit(1);
            };

            let css = adw::gtk::CssProvider::new();
            css.load_from_resource("/style.css");
            adw::gtk::style_context_add_provider_for_display(
                &display,
                &css,
                adw::gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
    }

    impl GtkApplicationImpl for VoidApplication {}
    impl AdwApplicationImpl for VoidApplication {}
}

glib::wrapper! {
    pub struct VoidApplication(ObjectSubclass<imp::VoidApplication>)
        @extends adw::Application, adw::gtk::Application, adw::gio::Application,
        @implements adw::gio::ActionGroup, adw::gio::ActionMap;
}

impl VoidApplication {
    /// Creates a new `VoidApplication` instance with the `me.acristoffers.void` application ID.
    pub fn new() -> Self {
        glib::Object::builder()
            .property("application-id", "me.acristoffers.void")
            .build()
    }
}
