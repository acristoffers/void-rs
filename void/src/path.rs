/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use path_absolutize::*;
use std::convert::From;
use std::path;

pub trait EasyPath {
    fn abs(&self) -> Option<String>;
    fn abs_par(&self) -> Option<String>;
}

impl EasyPath for path::Path {
    fn abs(&self) -> Option<String> {
        self.absolutize().ok()?.to_str().map(String::from)
    }

    fn abs_par(&self) -> Option<String> {
        self.parent()?.absolutize().ok()?.to_str().map(String::from)
    }
}

#[derive(Clone, Debug)]
pub struct Path {
    pub name: String,
    pub path: String,
    pub parent: String,
}

impl Path {
    /// Creates a new `Path` structure containing the path `path`.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to represent.
    pub fn new<'a, S: Into<&'a String>>(path: S) -> Option<Self> {
        let regex = regex::Regex::new("[/]+").ok()?;
        let path: &String = path.into();
        let path = regex.replace_all(path, "/");
        let fs_path = path::Path::new(path.as_ref());

        let path: String = match fs_path.abs() {
            Some(path) => {
                if path != "/" && path.ends_with('/') {
                    path[..path.len() - 1].into()
                } else {
                    path
                }
            }
            None => return None,
        };

        let name: String = match fs_path.file_name() {
            Some(name) => name.to_string_lossy().into(),
            None => {
                if path == "/" {
                    "".into()
                } else {
                    return None;
                }
            }
        };

        let parent: String = match fs_path.abs_par() {
            Some(path) => path,
            None => {
                if path == "/" {
                    "/".into()
                } else {
                    return None;
                }
            }
        };

        Some(Path { name, path, parent })
    }

    /// Changes the root of the path.
    /// If this path is "/folder/file" and you call with_root with
    /// remove="/folder" and new_root="/dir", you get a new `Path` containing
    /// "/dir/file"
    ///
    /// # Arguments
    ///
    /// * `remove` - Portion of this path to replace.
    /// * `new_root` - What to replace with
    pub fn with_root<S: Into<String>>(&self, remove: S, new_root: S) -> Option<Self> {
        let remove: String = remove.into();
        let new_root: String = new_root.into();

        let remove: String = if remove.ends_with('/') {
            remove[..remove.len() - 1].into()
        } else {
            remove
        };

        let new_root: String = if new_root.ends_with('/') && new_root != "/" {
            new_root[..new_root.len() - 1].into()
        } else {
            new_root
        };

        if !self.path.contains(&remove) {
            return None;
        }

        let removed = self.path.replacen(&remove, "", 1);
        if !removed.starts_with('/') && !removed.is_empty() {
            return None;
        }

        let new_path = self.path.replacen(&remove, &new_root, 1);

        Path::new(&new_path)
    }

    /// Returns a new path that joins this path with node.
    /// If this path is "/folder" and you pass node="file", the returned path
    /// will be "/folder/file".
    ///
    /// # Arguments
    ///
    /// * `node` - Node to append to this path.
    pub fn join<S: Into<String>>(&self, node: S) -> Option<Self> {
        let node: String = node.into();
        let path = path::Path::new(&self.path).join(node);
        let path = path.abs()?;
        Path::new(&path)
    }

    /// Returns whether this file/folder exists.
    /// Only makes sense to call on filesystem paths.
    pub fn exists(&self) -> bool {
        path::Path::new(&self.path).exists()
    }

    /// Returns true if the path is a directory.
    /// Only works for filesystem paths.
    pub fn is_dir(&self) -> bool {
        path::Path::new(&self.path).is_dir()
    }

    /// Returns the components of the path
    pub fn components(&self) -> Vec<String> {
        path::Path::new(&self.path)
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into())
            .collect()
    }
}

impl From<path::PathBuf> for Path {
    fn from(path: path::PathBuf) -> Self {
        let path = path.abs().unwrap();
        Path::new(&path).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_path() {
        let cwd = std::env::current_dir().unwrap();
        let cwd = cwd.as_path();

        let path = Path::new(&"/path".into()).unwrap();
        assert_eq!(path.name, "path");
        assert_eq!(path.path, "/path");
        assert_eq!(path.parent, "/");

        let path = Path::new(&"/path/".into()).unwrap();
        assert_eq!(path.name, "path");
        assert_eq!(path.path, "/path");
        assert_eq!(path.parent, "/");
        let path = Path::new(&"/".into()).unwrap();

        assert_eq!(path.name, "");
        assert_eq!(path.path, "/");
        assert_eq!(path.parent, "/");

        let path = Path::new(&"/path/to/file".into()).unwrap();
        assert_eq!(path.name, "file");
        assert_eq!(path.path, "/path/to/file");
        assert_eq!(path.parent, "/path/to");

        let path = Path::new(&"/path/./to//nope/../file".into()).unwrap();
        assert_eq!(path.name, "file");
        assert_eq!(path.path, "/path/to/file");
        assert_eq!(path.parent, "/path/to");

        let path = Path::new(&"./path/to/file".into()).unwrap();
        assert_eq!(path.name, "file");
        assert_eq!(path::Path::new(&path.path), cwd.join("path/to/file"));
        assert_eq!(path::Path::new(&path.parent), cwd.join("path/to"));

        let path = Path::new(&"path/to/file".into()).unwrap();
        assert_eq!(path.name, "file");
        assert_eq!(path::Path::new(&path.path), cwd.join("path/to/file"));
        assert_eq!(path::Path::new(&path.parent), cwd.join("path/to"));
    }

    #[test]
    fn test_change_root() {
        let path = Path::new(&"/some/path".into()).unwrap();
        let new_path = path.with_root("/some", "/").unwrap();
        assert_eq!(new_path.name, "path");
        assert_eq!(new_path.path, "/path");
        assert_eq!(new_path.parent, "/");

        let path = Path::new(&"/some/path".into()).unwrap();
        let new_path = path.with_root("/some/", "/").unwrap();
        assert_eq!(new_path.name, "path");
        assert_eq!(new_path.path, "/path");
        assert_eq!(new_path.parent, "/");

        let path = Path::new(&"/some/longer/path".into()).unwrap();
        let new_path = path.with_root("/some", "/a").unwrap();
        assert_eq!(new_path.name, "path");
        assert_eq!(new_path.path, "/a/longer/path");
        assert_eq!(new_path.parent, "/a/longer");

        let path = Path::new(&"/some///////////longer/path".into()).unwrap();
        let new_path = path.with_root("/some", "/a/").unwrap();
        assert_eq!(new_path.name, "path");
        assert_eq!(new_path.path, "/a/longer/path");
        assert_eq!(new_path.parent, "/a/longer");

        let path = Path::new(&"/some/longer/path".into()).unwrap();
        let new_path = path.with_root("/some/", "/a/").unwrap();
        assert_eq!(new_path.name, "path");
        assert_eq!(new_path.path, "/a/longer/path");
        assert_eq!(new_path.parent, "/a/longer");

        let path = Path::new(&"/some/longer/path".into()).unwrap();
        let new_path = path.with_root("/som", "/a");
        assert!(new_path.is_none());
    }

    #[test]
    fn test_join_path() {
        let path = Path::new(&"/a".into()).unwrap();
        let new_path = path.join("b").unwrap();
        assert_eq!(new_path.name, "b");
        assert_eq!(new_path.path, "/a/b");
        assert_eq!(new_path.parent, "/a");
    }
}
