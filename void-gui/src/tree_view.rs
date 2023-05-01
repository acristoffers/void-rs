/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use iced::alignment::Vertical;
use iced::theme::Button;
use iced::widget::{button, column, row, text};
use iced::{Color, Element, Length, Padding};
use void::Path;
use void::Store;

use crate::{VoidMessage, VoidRoute};

pub fn tree_view(store: &mut Store) -> TreeView {
    let mut tree_view = TreeView {
        root: TreeViewNode {
            name: "".into(),
            path: "".into(),
            expanded: false,
            primary: false,
            children: vec![],
        },
    };

    tree_view.update_tree(store);

    tree_view
}

pub struct TreeView {
    root: TreeViewNode,
}

#[derive(Default)]
struct TreeViewNode {
    name: String,
    path: String,
    expanded: bool,
    primary: bool,
    children: Vec<TreeViewNode>,
}

impl TreeViewNode {
    fn tree_node(&self) -> Element<VoidMessage> {
        let collapse_icon = match (self.expanded, self.children.len()) {
            (false, 0) => text(" "),
            (true, _) => text("-"),
            (false, _) => text("+"),
        }
        .size(24)
        .height(Length::Fixed(24f32))
        .width(Length::Shrink);

        let collapse_button = button(collapse_icon)
            .on_press(VoidMessage::TreeViewToggleExpand(self.path.clone()))
            .style(Button::Text);

        let label = button(
            text(self.name.clone())
                .style(if self.primary {
                    Color::from_rgb8(0x4F, 0x74, 0xD9)
                } else {
                    Color::WHITE
                })
                .size(24)
                .height(Length::Fill)
                .vertical_alignment(Vertical::Center),
        )
        .style(Button::Text)
        .width(Length::Fill)
        .height(Length::Fixed(48f32))
        .on_press(VoidMessage::RouteChanged(VoidRoute::Home(
            self.path.clone(),
        )));

        let el = row![collapse_button, label]
            .width(Length::Fill)
            .height(Length::Fixed(42f32))
            .spacing(8)
            .align_items(iced::Alignment::Center);

        let children: Vec<Element<VoidMessage>> = if self.expanded {
            self.children.iter().map(|c| c.tree_node()).collect()
        } else {
            vec![]
        };

        let children_container = column(children).padding(Padding::from([0, 0, 0, 36]));

        column![el, children_container].into()
    }
}

impl TreeView {
    pub fn view(&self) -> Element<VoidMessage> {
        self.root.tree_node()
    }

    pub fn update_tree(&mut self, store: &mut Store) {
        self.root = build_tree(store, "Home".into(), "/");
    }

    pub fn select_path(&mut self, path: &str) {
        select_path(&mut self.root, path);
    }

    pub fn toggle_expand_path(&mut self, path: &str) {
        toggle_expand_path(&mut self.root, path);
    }
}

fn build_tree(store: &mut Store, name: String, path: &str) -> TreeViewNode {
    TreeViewNode {
        name,
        expanded: false,
        primary: false,
        path: path.to_string(),
        children: store
            .list(path)
            .unwrap() // The possible errors are impossible in this context.
            .iter()
            .filter(|c| !c.is_file)
            .map(|c| {
                build_tree(
                    store,
                    c.name.clone(),
                    &path_join(path.to_string(), c.name.clone()),
                )
            })
            .collect(),
    }
}

fn path_join(p1: String, p2: String) -> String {
    // This is an internal function, so I expect both p1 and p2 to always be
    // valid path components, since they come from Store.
    let path = Path::new(&p1).unwrap();
    path.join(p2).unwrap().path
}

fn select_path(node: &mut TreeViewNode, path: &str) {
    node.primary = node.path == path;
    for child in node.children.iter_mut() {
        select_path(child, path);
    }
}

fn toggle_expand_path(node: &mut TreeViewNode, path: &str) {
    if node.path == path {
        node.expanded = !node.expanded;
    } else {
        for child in node.children.iter_mut() {
            toggle_expand_path(child, path);
        }
    }
}
