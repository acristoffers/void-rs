/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::i18n::{Language, I18N};
use crate::tree_view::TreeView;
use crate::ThemeType;
use crate::VoidMessage;
use crate::{Assets, VoidGui};

use iced::alignment::Vertical::Center;
use iced::theme::Button;
use iced::theme::Container;
use iced::widget::{button, column, container, row, scrollable, svg, text};
use iced::{Alignment, Element, Length};
use iced::{Background, Color, Padding, Theme};
use iced_native::{keyboard, Event};
use void::Store;

pub struct HomeComponent {
    pub store: Box<Store>,
    pub tree_view: TreeView,
}

impl HomeComponent {
    pub fn update(&self, state: &VoidGui, message: VoidMessage) -> Option<VoidMessage> {
        match message {
            VoidMessage::EventOccurred(Event::Keyboard(keyboard::Event::KeyPressed {
                key_code: keyboard::KeyCode::Q,
                modifiers: keyboard::Modifiers::CTRL,
            })) => Some(VoidMessage::CloseStore),

            VoidMessage::EventOccurred(Event::Keyboard(keyboard::Event::KeyPressed {
                key_code: keyboard::KeyCode::T,
                modifiers: _,
            })) => match ThemeType::from_iced_theme(state.theme.clone()) {
                ThemeType::Dark => Some(VoidMessage::ThemeChanged(ThemeType::Light)),
                ThemeType::Light => Some(VoidMessage::ThemeChanged(ThemeType::Dark)),
            },

            _ => None,
        }
    }

    pub fn view(&self, i18n: &I18N, _path: &str) -> Element<VoidMessage> {
        let banner = self.banner_widget(i18n);

        let sidebar_left = self.sidebar_left(i18n);
        let content = column![].width(Length::Fill);
        let sidebar_right = self.sidebar_right(i18n);

        let body = row![sidebar_left, content, sidebar_right]
            .height(Length::Fill)
            .width(Length::Fill);

        column![banner, body].into()
    }

    fn banner_widget(&self, i18n: &I18N) -> Element<VoidMessage> {
        let logo_svg = Assets::get("icon.svg").unwrap();
        let logo = svg(svg::Handle::from_memory(logo_svg.data))
            .width(Length::Fixed(48f32))
            .height(Length::Fixed(48f32));

        let app_name = text(i18n.tr("Void"))
            .style(Color::WHITE)
            .size(36)
            .vertical_alignment(Center)
            .height(Length::Fixed(48f32))
            .width(Length::Fill);

        let en_svg = svg::Handle::from_memory(Assets::get("en.svg").unwrap().data);
        let de_svg = svg::Handle::from_memory(Assets::get("de.svg").unwrap().data);
        let fr_svg = svg::Handle::from_memory(Assets::get("fr.svg").unwrap().data);
        let pt_svg = svg::Handle::from_memory(Assets::get("pt.svg").unwrap().data);

        let flags_size = Length::Fixed(24f32);
        let flag_button = |svg_handle, language| {
            button(svg(svg_handle).width(flags_size).height(flags_size))
                .on_press(VoidMessage::LanguageChanged(language))
                .height(Length::Fixed(48f32))
                .style(Button::Text)
        };

        let en_button = flag_button(en_svg, Language::English);
        let de_button = flag_button(de_svg, Language::German);
        let fr_button = flag_button(fr_svg, Language::French);
        let pt_button = flag_button(pt_svg, Language::Portuguese);

        let flags = row![en_button, de_button, fr_button, pt_button];

        let container_style: fn(&Theme) -> container::Appearance =
            |theme: &Theme| container::Appearance {
                text_color: None,
                background: Some(Background::Color(
                    theme.extended_palette().primary.strong.color,
                )),
                border_radius: 0.0,
                border_width: 0.0,
                border_color: Color::BLACK,
            };

        container(
            row![logo, app_name, flags]
                .spacing(8)
                .padding(8)
                .align_items(Alignment::Start)
                .height(Length::Fill)
                .width(Length::Fill),
        )
        .style(Container::Custom(Box::new(container_style)))
        .height(Length::Fixed(48f32 + 16f32))
        .width(Length::Fill)
        .into()
    }

    fn sidebar_left(&self, _i18n: &I18N) -> Element<VoidMessage> {
        let tree_view = self.tree_view.view();

        let sidebar_style: fn(&Theme) -> container::Appearance =
            |_: &Theme| container::Appearance {
                text_color: None,
                background: Some(Background::Color(Color::from_rgb8(0x25, 0x27, 0x2A))),
                border_radius: 0.0,
                border_width: 0.0,
                border_color: Color::TRANSPARENT,
            };

        let container = container(scrollable(tree_view).height(Length::Fill))
            .style(Container::Custom(Box::new(sidebar_style)))
            .height(Length::Fill)
            .padding(Padding::from([8, 8, 8, 8]))
            .width(Length::Fixed(300f32));

        container.into()
    }

    fn sidebar_right(&self, _i18n: &I18N) -> Element<VoidMessage> {
        let sidebar_style: fn(&Theme) -> container::Appearance =
            |_: &Theme| container::Appearance {
                text_color: None,
                background: Some(Background::Color(Color::from_rgb8(0x25, 0x27, 0x2A))),
                border_radius: 0.0,
                border_width: 0.0,
                border_color: Color::TRANSPARENT,
            };

        container(
            column![]
                .spacing(8)
                .padding(8)
                .align_items(Alignment::Start)
                .height(Length::Fill)
                .width(Length::Fill),
        )
        .style(Container::Custom(Box::new(sidebar_style)))
        .height(Length::Fill)
        .width(Length::Fixed(300f32))
        .into()
    }
}
