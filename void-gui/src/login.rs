use crate::i18n::Language;
use crate::i18n::I18N;
use crate::Assets;
use crate::PickFileOptions;
use crate::ThemeType;
use crate::VoidMessage;
use crate::VoidRoute;

use iced::alignment::{Horizontal, Vertical};
use iced::theme::Button;
use iced::theme::Theme;
use iced::widget::text_input;
use iced::widget::{button, column, row, svg, text};
use iced::{Alignment, Element, Length};
use iced_native::{keyboard, Event};

pub fn login_component<'a>(
    i18n: &I18N,
    theme: Theme,
    page: LoginPage,
    password: Option<String>,
) -> Element<'a, VoidMessage> {
    LoginComponent {
        i18n,
        theme,
        page,
        password: password.unwrap_or("".into()),
    }
    .view()
}

pub fn login_update(
    i18n: &I18N,
    theme: Theme,
    page: LoginPage,
    password: Option<String>,
    event: VoidMessage,
) -> Option<VoidMessage> {
    LoginComponent {
        i18n,
        theme,
        page,
        password: password.unwrap_or("".into()),
    }
    .update(event)
}

#[derive(Debug, Clone)]
pub struct LoginComponent<'a> {
    i18n: &'a I18N,
    theme: Theme,
    page: LoginPage,
    password: String,
}

#[derive(Debug, Clone, Default)]
pub enum LoginPage {
    Create,
    Open,
    #[default]
    Home,
}

impl LoginComponent<'_> {
    pub fn update(self, event: VoidMessage) -> Option<VoidMessage> {
        match event {
            VoidMessage::EventOccurred(Event::Keyboard(keyboard::Event::KeyPressed {
                key_code: keyboard::KeyCode::T,
                modifiers: _,
            })) => match ThemeType::from_iced_theme(self.theme) {
                ThemeType::Dark => Some(VoidMessage::ThemeChanged(ThemeType::Light)),
                ThemeType::Light => Some(VoidMessage::ThemeChanged(ThemeType::Dark)),
            },

            VoidMessage::EventOccurred(Event::Keyboard(keyboard::Event::KeyPressed {
                key_code: keyboard::KeyCode::O,
                modifiers: _,
            })) => Some(VoidMessage::PickFile(PickFileOptions {
                pick_file: true,
                pick_store: true,
                save: false,
                many: false,
                file_name: None,
                title: Some("Open Void Store".into()),
                next_message_cancel: Box::new(VoidMessage::NoOp),
                next_message_success: Box::new(VoidMessage::RouteChanged(VoidRoute::Login(
                    LoginPage::Open,
                ))),
            })),

            VoidMessage::EventOccurred(Event::Keyboard(keyboard::Event::KeyPressed {
                key_code: keyboard::KeyCode::C,
                modifiers: _,
            })) => Some(VoidMessage::PickFile(PickFileOptions {
                pick_file: true,
                pick_store: false,
                save: true,
                many: false,
                file_name: None,
                title: Some("Create Void Store".into()),
                next_message_cancel: Box::new(VoidMessage::NoOp),
                next_message_success: Box::new(VoidMessage::RouteChanged(VoidRoute::Login(
                    LoginPage::Create,
                ))),
            })),

            VoidMessage::EventOccurred(Event::Keyboard(keyboard::Event::KeyPressed {
                key_code: keyboard::KeyCode::Escape,
                modifiers: _,
            })) => Some(VoidMessage::RouteChanged(VoidRoute::Login(LoginPage::Home))),

            _ => None,
        }
    }

    pub fn view<'a>(self) -> Element<'a, VoidMessage> {
        match &self.page {
            LoginPage::Home => self.home_view(),
            LoginPage::Create => self.create_view(),
            LoginPage::Open => self.open_view(),
        }
    }

    pub fn home_view<'a>(self) -> Element<'a, VoidMessage> {
        let svg_file = Assets::get("icon.svg").unwrap();
        let logo = svg(svg::Handle::from_memory(svg_file.data))
            .width(Length::Fixed(256f32))
            .height(Length::Fixed(256f32));

        let name = text("Void").size(48);

        let create = button(
            text(self.i18n.tr("create"))
                .size(24)
                .width(Length::Fill)
                .height(Length::Fill)
                .horizontal_alignment(Horizontal::Center)
                .vertical_alignment(Vertical::Center),
        )
        .height(Length::Fixed(48f32))
        .width(Length::Fixed(192f32))
        .on_press(VoidMessage::PickFile(PickFileOptions {
            pick_file: true,
            pick_store: false,
            save: true,
            many: false,
            file_name: None,
            title: Some("create_void_store".into()),
            next_message_cancel: Box::new(VoidMessage::NoOp),
            next_message_success: Box::new(VoidMessage::RouteChanged(VoidRoute::Login(
                LoginPage::Create,
            ))),
        }));

        let open = button(
            text(self.i18n.tr("open"))
                .size(24)
                .width(Length::Fill)
                .height(Length::Fill)
                .horizontal_alignment(Horizontal::Center)
                .vertical_alignment(Vertical::Center),
        )
        .height(Length::Fixed(48f32))
        .width(Length::Fixed(192f32))
        .on_press(VoidMessage::PickFile(PickFileOptions {
            pick_file: true,
            pick_store: true,
            save: false,
            many: false,
            file_name: Some("Store.void".into()),
            title: Some("open_void_store".into()),
            next_message_cancel: Box::new(VoidMessage::NoOp),
            next_message_success: Box::new(VoidMessage::RouteChanged(VoidRoute::Login(
                LoginPage::Open,
            ))),
        }));

        let row = row![open, create]
            .spacing(16)
            .padding(16)
            .align_items(Alignment::Center);

        let en_svg = svg::Handle::from_memory(Assets::get("en.svg").unwrap().data);
        let de_svg = svg::Handle::from_memory(Assets::get("de.svg").unwrap().data);
        let fr_svg = svg::Handle::from_memory(Assets::get("fr.svg").unwrap().data);
        let pt_svg = svg::Handle::from_memory(Assets::get("pt.svg").unwrap().data);

        let flags_size = Length::Fixed(32f32);
        let flag_button = |svg_handle, language| {
            button(svg(svg_handle).width(flags_size).height(flags_size))
                .on_press(VoidMessage::LanguageChanged(language))
                .style(Button::Text)
        };

        let en_button = flag_button(en_svg, Language::English);
        let de_button = flag_button(de_svg, Language::German);
        let fr_button = flag_button(fr_svg, Language::French);
        let pt_button = flag_button(pt_svg, Language::Portuguese);

        let flags = row![en_button, de_button, fr_button, pt_button]
            .spacing(16)
            .padding(16)
            .align_items(Alignment::Center);

        let theme_mode_file = match ThemeType::from_iced_theme(self.theme.clone()) {
            ThemeType::Dark => Assets::get("light-mode.svg").unwrap(),
            ThemeType::Light => Assets::get("dark-mode.svg").unwrap(),
        };

        let theme_mode_handle = svg::Handle::from_memory(theme_mode_file.data);
        let theme_toggle = button(
            svg(theme_mode_handle)
                .width(Length::Fixed(32f32))
                .height(Length::Fixed(32f32)),
        )
        .on_press(VoidMessage::ThemeChanged(
            match ThemeType::from_iced_theme(self.theme) {
                ThemeType::Dark => ThemeType::Light,
                ThemeType::Light => ThemeType::Dark,
            },
        ))
        .style(Button::Text);

        column![logo, name, row, flags, theme_toggle]
            .spacing(16)
            .padding(16)
            .max_width(600)
            .align_items(Alignment::Center)
            .into()
    }

    pub fn open_view<'a>(self) -> Element<'a, VoidMessage> {
        let svg_file = Assets::get("icon.svg").unwrap();
        let logo = svg(svg::Handle::from_memory(svg_file.data))
            .width(Length::Fixed(256f32))
            .height(Length::Fixed(256f32));

        let name = text("Void").size(48);

        let password = text_input(&self.i18n.tr("password"), &self.password)
            .on_input(VoidMessage::PasswordChanged)
            .password()
            .size(24)
            .on_submit(VoidMessage::OpenStore);

        let cancel = button(
            text(self.i18n.tr("cancel"))
                .size(24)
                .width(Length::Fill)
                .height(Length::Fill)
                .horizontal_alignment(Horizontal::Center)
                .vertical_alignment(Vertical::Center),
        )
        .height(Length::Fixed(48f32))
        .width(Length::Fixed(192f32))
        .style(Button::Destructive)
        .on_press(VoidMessage::RouteChanged(crate::VoidRoute::Login(
            LoginPage::Home,
        )));

        let open = button(
            text(self.i18n.tr("open"))
                .size(24)
                .width(Length::Fill)
                .height(Length::Fill)
                .horizontal_alignment(Horizontal::Center)
                .vertical_alignment(Vertical::Center),
        )
        .height(Length::Fixed(48f32))
        .width(Length::Fixed(192f32))
        .on_press(VoidMessage::OpenStore);

        let row = row![open, cancel]
            .spacing(16)
            .padding(16)
            .align_items(Alignment::Center);

        column![logo, name, text(self.i18n.tr("open")), password, row]
            .spacing(16)
            .padding(16)
            .max_width(600)
            .align_items(Alignment::Center)
            .into()
    }

    pub fn create_view<'a>(self) -> Element<'a, VoidMessage> {
        let svg_file = Assets::get("icon.svg").unwrap();
        let logo = svg(svg::Handle::from_memory(svg_file.data))
            .width(Length::Fixed(256f32))
            .height(Length::Fixed(256f32));

        let name = text("Void").size(48);

        let password = text_input(&self.i18n.tr("password"), &self.password)
            .on_input(VoidMessage::PasswordChanged)
            .password()
            .size(24)
            .on_submit(VoidMessage::CreateStore);

        let cancel = button(
            text(self.i18n.tr("cancel"))
                .size(24)
                .width(Length::Fill)
                .height(Length::Fill)
                .horizontal_alignment(Horizontal::Center)
                .vertical_alignment(Vertical::Center),
        )
        .height(Length::Fixed(48f32))
        .width(Length::Fixed(192f32))
        .style(Button::Destructive)
        .on_press(VoidMessage::RouteChanged(crate::VoidRoute::Login(
            LoginPage::Home,
        )));

        let create = button(
            text(self.i18n.tr("create"))
                .size(24)
                .width(Length::Fill)
                .height(Length::Fill)
                .horizontal_alignment(Horizontal::Center)
                .vertical_alignment(Vertical::Center),
        )
        .height(Length::Fixed(48f32))
        .width(Length::Fixed(192f32))
        .on_press(VoidMessage::CreateStore);

        let row = row![create, cancel]
            .spacing(16)
            .padding(16)
            .align_items(Alignment::Center);

        column![logo, name, text(&self.i18n.tr("create")), password, row]
            .spacing(16)
            .padding(16)
            .max_width(600)
            .align_items(Alignment::Center)
            .into()
    }
}
