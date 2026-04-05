/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use adw::gtk::{self, Box, Button};
use adw::prelude::*;

use libadwaita as adw;

/// Available icon pixel sizes for the grid view.
pub(crate) const ICON_SIZES: &[i32] = &[32, 48, 64, 96, 128];

/// Search is inactive — the grid shows normal directory contents.
pub(crate) const SEARCH_MODE_NONE: u8 = 0;
/// Filter mode — restrict results to direct children of the current path.
pub(crate) const SEARCH_MODE_FILTER: u8 = 1;
/// Full search mode — scan the entire vault for matching entries.
pub(crate) const SEARCH_MODE_SEARCH: u8 = 2;

/// A single entry (file or folder) displayed in the grid or tree view.
#[derive(Clone, Debug)]
pub(crate) struct StoreEntry {
    pub name: String,
    pub path: String,
    pub is_file: bool,
    pub thumbnail: Option<Vec<u8>>,
}

/// Internal clipboard state for copy/cut operations within the vault.
#[derive(Clone, Debug, Default)]
pub(crate) struct ClipboardState {
    /// Paths of items that have been copied or cut.
    pub paths: Vec<String>,
    /// `true` if the operation is cut (move), `false` for copy.
    pub is_cut: bool,
}

/// Descriptor for a pending thumbnail generation job.
#[derive(Clone, Debug)]
pub(crate) struct ThumbnailWork {
    /// Index of the item inside the grid `ListStore`.
    pub grid_index: u32,
    /// Full path of the file inside the void store.
    pub store_path: String,
    /// Display file name (used to pick the correct thumbnail generator).
    pub file_name: String,
}

/// Result of a completed thumbnail generation job.
pub(crate) struct ThumbnailResult {
    /// Index of the item inside the grid `ListStore`.
    pub grid_index: u32,
    /// Full path of the file inside the void store.
    pub store_path: String,
    /// JPEG-encoded thumbnail bytes.
    pub jpeg_bytes: Vec<u8>,
}

/// Replaces the contents of `container` with clickable breadcrumb buttons for `path`.
pub(crate) fn rebuild_breadcrumbs(container: &Box, path: &str) {
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }

    let root_btn = Button::with_label("/");
    root_btn.add_css_class("flat");
    root_btn.set_action_name(Some("win.navigate"));
    root_btn.set_action_target_value(Some(&"/".to_variant()));
    container.append(&root_btn);

    let mut accumulated = String::new();
    for segment in path.split('/').filter(|s| !s.is_empty()) {
        accumulated = format!("{}/{}", accumulated, segment);

        let sep = gtk::Image::from_icon_name("go-next-symbolic");
        sep.set_pixel_size(16);
        sep.set_opacity(0.5);
        sep.set_margin_top(8);
        sep.set_margin_bottom(8);
        container.append(&sep);

        let btn = Button::with_label(segment);
        btn.add_css_class("flat");
        btn.set_action_name(Some("win.navigate"));
        btn.set_action_target_value(Some(&accumulated.to_variant()));
        container.append(&btn);
    }
}

/// Returns the GTK icon name best matching the file extension of `filename`.
pub(crate) fn icon_name_for_mime_type(filename: &str) -> &'static str {
    let ext = filename.rsplit('.').next().unwrap_or("").to_lowercase();

    match ext.as_str() {
        // Music
        "mp3" | "flac" | "aac" | "m4a" | "ogg" | "wav" | "wma" | "opus" => "audio-x-generic",
        // Documents
        "pdf" => "application-pdf",
        "doc" | "docx" => "x-office-document",
        "odt" => "x-office-document",
        "xls" | "xlsx" => "x-office-spreadsheet",
        "ods" => "x-office-spreadsheet",
        "ppt" | "pptx" => "x-office-presentation",
        "odp" => "x-office-presentation",
        // Text/Source code
        "txt" | "md" | "rst" | "tex" | "org" => "text-x-generic",
        "rs" | "c" | "cpp" | "h" | "hpp" | "py" | "js" | "ts" | "go" | "java" | "rb" | "php"
        | "css" | "html" | "xml" | "json" | "yaml" | "toml" | "sh" | "bash" | "fish" | "zsh" => {
            "text-x-generic"
        }
        _ => "text-x-generic",
    }
}

/// Formats a byte count as a human-readable string (e.g. `1.50 MB`).
pub(crate) fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;
    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }
    if unit_idx == 0 {
        format!("{} {}", size as u64, UNITS[unit_idx])
    } else {
        format!("{:.2} {}", size, UNITS[unit_idx])
    }
}

/// Recursively computes the total byte size of all files under `path` in the store.
pub(crate) fn folder_total_size(store: &void::Store, path: &str) -> u64 {
    let Ok(entries) = store.list(path) else {
        return 0;
    };
    entries
        .iter()
        .map(|e| {
            if e.is_file {
                e.size
            } else {
                let child_path = if path == "/" {
                    format!("/{}", e.name)
                } else {
                    format!("{}/{}", path, e.name)
                };
                folder_total_size(store, &child_path)
            }
        })
        .sum()
}
