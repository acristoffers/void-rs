/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use libadwaita as adw;

use adw::gtk::{Box, Button, Image, Label, Orientation, TextView};
use adw::prelude::*;
use adw::{Application, ApplicationWindow, HeaderBar};

use crate::settings;
use settings::settings_window;

pub fn login_window(app: &Application) -> ApplicationWindow {
    let open_button = Button::builder().label("Open").build();
    open_button.connect_clicked(|_| {
        eprintln!("Open!");
    });

    let create_button = Button::builder().label("Create").build();
    create_button.connect_clicked(|_| {
        eprintln!("Create!");
    });

    let setting_button = Button::builder().icon_name("settings-symbolic").build();
    setting_button.connect_clicked(|_| {
        let settings = settings_window();
        settings.show();
    });

    let header_bar = HeaderBar::new();
    header_bar.pack_start(&open_button);
    header_bar.pack_start(&create_button);
    header_bar.pack_end(&setting_button);

    let image = Image::new();
    image.set_from_resource(Some("/icon.png"));
    image.set_width_request(256);
    image.set_height_request(256);

    let name = Label::new(Some("Void"));
    name.set_css_classes(&["title-1"]);

    let text = TextView::new();
    text.set_css_classes(&["body"]);
    text.set_justification(adw::gtk::Justification::Center);
    let text_buffer = text.buffer();
    text_buffer.set_text("Please use the Open and Create buttons.");

    let view = Box::new(Orientation::Vertical, 0);
    view.set_valign(adw::gtk::Align::Center);
    view.set_hexpand(true);
    view.set_vexpand(true);
    view.set_spacing(16);
    view.append(&image);
    view.append(&name);
    view.append(&text);

    let content = Box::new(Orientation::Vertical, 0);
    content.append(&header_bar);
    content.append(&view);

    let window = ApplicationWindow::builder()
        .application(app)
        .title("Void")
        .default_width(800)
        .default_height(600)
        .icon_name("/icon.png")
        .content(&content)
        .build();

    return window;
}
