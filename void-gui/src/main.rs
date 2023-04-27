mod home;
mod i18n;
mod login;
mod tree_view;

use std::path::PathBuf;

use crate::home::HomeComponent;
use crate::login::{login_component, LoginPage};

use i18n::{Language, I18N};
use iced::theme::Theme;
use iced::widget::container;
use iced::window::icon::from_file_data;
use iced::window::Position;
use iced::{executor, Subscription};
use iced::{Application, Command, Element, Length, Settings};
use iced_native::Event;
use login::login_update;
use rfd::FileDialog;
use rfd::MessageDialog;
use rust_embed::RustEmbed;
use tree_view::tree_view;
use void::Store;

fn main() -> iced::Result {
    let font_data = Box::leak(Box::new(Assets::get("Inconsolata.ttf").unwrap().data));

    let mut settings = Settings::default();
    settings.window.position = Position::Centered;
    settings.default_font = Some(font_data);
    settings.default_text_size = 36f32;
    settings.text_multithreading = true;
    settings.antialiasing = true;
    if let Some(png_icon) = Assets::get("icon.png") {
        let icon = from_file_data(&png_icon.data, None).unwrap();
        settings.window.icon = Some(icon);
    }

    VoidGui::run(settings)
}

#[derive(RustEmbed)]
#[folder = "assets/"]
pub struct Assets;

pub struct VoidGui {
    i18n: I18N,
    theme: Theme,
    route: VoidRoute,
    store_path: Option<PathBuf>,
    file_pick_result: Option<Vec<PathBuf>>,
    password: Option<String>,
    home_state: Option<HomeComponent>,
}

impl Default for VoidGui {
    fn default() -> Self {
        Self {
            i18n: I18N::new(i18n::Language::English),
            theme: Theme::Dark,
            route: Default::default(),
            store_path: None,
            file_pick_result: None,
            password: None,
            home_state: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PickFileOptions {
    pick_file: bool,
    pick_store: bool,
    many: bool,
    save: bool,
    file_name: Option<String>,
    title: Option<String>,
    next_message_success: Box<VoidMessage>,
    next_message_cancel: Box<VoidMessage>,
}

#[derive(Debug, Clone)]
pub enum VoidMessage {
    ThemeChanged(ThemeType),
    RouteChanged(VoidRoute),
    LanguageChanged(Language),
    PickFile(PickFileOptions),
    EventOccurred(Event),
    PasswordChanged(String),
    CreateStore,
    OpenStore,
    CloseStore,
    TreeViewToggleExpand(String),
    NoOp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeType {
    Dark,
    Light,
}

#[derive(Debug, Clone)]
pub enum VoidRoute {
    Login(LoginPage),
    Home(String),
}

impl Default for VoidRoute {
    fn default() -> Self {
        VoidRoute::Login(LoginPage::Home)
    }
}

impl ThemeType {
    fn to_iced_theme(self) -> Theme {
        match self {
            ThemeType::Dark => Theme::Dark,
            ThemeType::Light => Theme::Light,
        }
    }

    fn from_iced_theme(theme: Theme) -> ThemeType {
        match theme {
            Theme::Dark => ThemeType::Dark,
            Theme::Light => ThemeType::Light,
            _ => ThemeType::Dark,
        }
    }
}

impl Application for VoidGui {
    type Message = VoidMessage;
    type Theme = Theme;
    type Executor = executor::Default;
    type Flags = ();

    fn new(_flags: ()) -> (VoidGui, Command<VoidMessage>) {
        (VoidGui::default(), Command::none())
    }

    fn title(&self) -> String {
        String::from("Void")
    }

    fn update(&mut self, message: VoidMessage) -> Command<VoidMessage> {
        let new_message = match &message {
            VoidMessage::ThemeChanged(theme) => {
                self.theme = theme.to_iced_theme();
                None
            }

            VoidMessage::EventOccurred(event) => match event {
                Event::Keyboard(_) => match &self.route {
                    VoidRoute::Login(page) => login_update(
                        &self.i18n,
                        self.theme.clone(),
                        page.clone(),
                        self.password.clone(),
                        message.clone(),
                    ),
                    VoidRoute::Home(_) => {
                        if let Some(state) = &self.home_state {
                            state.update(self, message.clone())
                        } else {
                            panic!("Home route without home state.");
                        }
                    }
                },
                _ => None,
            },

            VoidMessage::RouteChanged(route) => {
                match route {
                    VoidRoute::Login(_) => {
                        if let Some(paths) = &self.file_pick_result {
                            self.store_path = Some(paths[0].clone());
                            self.file_pick_result = None;
                        }
                    }
                    VoidRoute::Home(path) => self
                        .home_state
                        .as_mut()
                        .unwrap()
                        .tree_view
                        .select_path(path),
                }
                self.password = None;
                self.route = route.clone();
                None
            }

            VoidMessage::PickFile(options) => {
                let mut files = FileDialog::new().set_directory("~");

                if let Some(file_name) = &options.file_name {
                    files = files.set_file_name(file_name);
                }

                if let Some(title) = &options.title {
                    files = files.set_title(title);
                }

                if options.pick_store {
                    files = files.add_filter("Void Store", &["void"])
                }

                if options.save {
                    self.file_pick_result = files.save_file().map(|x| vec![x]);
                } else {
                    self.file_pick_result = match (options.pick_file, options.many) {
                        (false, false) => files.pick_folder().map(|x| vec![x]),
                        (false, true) => files.pick_folders(),
                        (true, false) => files.pick_file().map(|x| vec![x]),
                        (true, true) => files.pick_files(),
                    };
                }

                if self.file_pick_result.is_some() {
                    Some(*options.next_message_success.clone())
                } else {
                    Some(*options.next_message_cancel.clone())
                }
            }

            VoidMessage::NoOp => None,

            VoidMessage::PasswordChanged(password) => {
                self.password = Some(password.clone());
                None
            }

            VoidMessage::CreateStore => 'blk: {
                if let Some(store_path) = &self.store_path {
                    if let Some(password) = &self.password {
                        if let Some(store_path_str) = store_path.as_os_str().to_str() {
                            match Store::create(store_path_str, password) {
                                Ok(store) => {
                                    let mut boxed = Box::new(store);
                                    let tv = tree_view(boxed.as_mut());
                                    self.home_state = Some(HomeComponent {
                                        store: boxed,
                                        tree_view: tv,
                                    });
                                    break 'blk Some(VoidMessage::RouteChanged(VoidRoute::Home(
                                        "/".into(),
                                    )));
                                }
                                Err(error) => {
                                    let error_message = void_errors(&self.i18n, error);
                                    MessageDialog::new()
                                        .set_title("Error")
                                        .set_description(
                                            &("An error occurred: ".to_owned() + &error_message),
                                        )
                                        .set_level(rfd::MessageLevel::Error)
                                        .set_buttons(rfd::MessageButtons::Ok)
                                        .show();
                                }
                            }
                        }
                    }
                }
                None
            }

            VoidMessage::OpenStore => 'blk: {
                if let (Some(store_path), Some(password)) = (&self.store_path, &self.password) {
                    if let Some(store_path_str) = store_path.parent().unwrap().as_os_str().to_str()
                    {
                        match Store::open(store_path_str, password) {
                            Ok(store) => {
                                let mut boxed = Box::new(store);
                                let tv = tree_view(boxed.as_mut());
                                self.home_state = Some(HomeComponent {
                                    store: boxed,
                                    tree_view: tv,
                                });
                                break 'blk Some(VoidMessage::RouteChanged(VoidRoute::Home(
                                    "/".into(),
                                )));
                            }
                            Err(error) => {
                                let error_message = void_errors(&self.i18n, error);
                                MessageDialog::new()
                                    .set_title("Error")
                                    .set_description(
                                        &("An error occurred: ".to_owned() + &error_message),
                                    )
                                    .set_level(rfd::MessageLevel::Error)
                                    .set_buttons(rfd::MessageButtons::Ok)
                                    .show();
                            }
                        }
                    }
                }
                None
            }

            VoidMessage::LanguageChanged(language) => {
                self.i18n.set_language(language.clone());
                None
            }

            VoidMessage::CloseStore => {
                self.home_state = None;
                self.password = None;
                self.store_path = None;
                self.file_pick_result = None;
                Some(VoidMessage::RouteChanged(VoidRoute::Login(LoginPage::Home)))
            }

            VoidMessage::TreeViewToggleExpand(path) => {
                self.home_state
                    .as_mut()
                    .unwrap()
                    .tree_view
                    .toggle_expand_path(path);
                None
            }
        };

        if let Some(m) = new_message {
            self.update(m)
        } else {
            Command::none()
        }
    }

    fn view(&self) -> Element<VoidMessage> {
        let root = match &self.route {
            VoidRoute::Login(page) => login_component(
                &self.i18n,
                self.theme.clone(),
                page.clone(),
                self.password.clone(),
            ),
            VoidRoute::Home(path) => match &self.home_state {
                Some(state) => state.view(&self.i18n, path),
                None => panic!("Got into Home view state without a home_state"),
            },
        };

        container(root)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .into()
    }

    fn theme(&self) -> Theme {
        self.theme.clone()
    }

    fn subscription(&self) -> Subscription<VoidMessage> {
        iced_native::subscription::events().map(VoidMessage::EventOccurred)
    }
}

fn void_errors(i18n: &I18N, error: void::Error) -> String {
    match error {
        void::Error::CannotCreateDirectoryError => i18n.tr("CannotCreateDirectoryError"),
        void::Error::CannotCreateFileError => i18n.tr("CannotCreateFileError"),
        void::Error::CannotDecryptFileError => i18n.tr("CannotDecryptFileError"),
        void::Error::CannotDeserializeError => i18n.tr("CannotDeserializeError"),
        void::Error::CannotParseError => i18n.tr("CannotParseError"),
        void::Error::CannotReadFileError => i18n.tr("CannotReadFileError"),
        void::Error::CannotRemoveFilesError(_) => i18n.tr("CannotRemoveFilesError"),
        void::Error::CannotSerializeError => i18n.tr("CannotSerializeError"),
        void::Error::CannotWriteFileError => i18n.tr("CannotWriteFileError"),
        void::Error::FileAlreadyExistsError => i18n.tr("FileAlreadyExistsError"),
        void::Error::FileDoesNotExistError => i18n.tr("FileDoesNotExistError"),
        void::Error::FolderDoesNotExistError => i18n.tr("FolderDoesNotExistError"),
        void::Error::StoreFileAlreadyExistsError => i18n.tr("StoreFileAlreadyExistsError"),
        void::Error::NoSuchMetadataKey => i18n.tr("NoSuchMetadataKey"),
        void::Error::InternalStructureError => i18n.tr("InternalStructureError"),
        void::Error::CannotEncryptFileError => i18n.tr("CannotEncryptFileError"),
    }
}
