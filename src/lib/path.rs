/*
 * The MIT License (MIT)
 *
 * Copyright (c) 2020 Álan Crístoffer e Sousa
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, lish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in
 * all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.  IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */

use path_absolutize::*;
use std::convert::From;
use std::path;

trait EasyPath {
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
    pub fn new<S: Into<String>>(path: S) -> Option<Self> {
        let regex = regex::Regex::new("[/]+").ok()?;
        let path: String = path.into();
        let path = regex.replace_all(&path, "/");
        let fs_path = path::Path::new(path.as_ref());

        let path: String = match fs_path.abs() {
            Some(path) => {
                if path != "/" && path.ends_with("/") {
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

    pub fn with_root<S: Into<String>>(&self, remove: S, new_root: S) -> Option<Self> {
        let remove: String = remove.into();
        let new_root: String = new_root.into();

        let remove: String = if remove.ends_with("/") {
            remove[..remove.len() - 1].into()
        } else {
            remove
        };

        let new_root: String = if new_root.ends_with("/") {
            new_root[..new_root.len() - 1].into()
        } else {
            new_root
        };

        if !self.path.contains(&remove) {
            return None;
        }

        let removed = self.path.replacen(&remove, "", 1);
        if !removed.starts_with("/") && !removed.is_empty() {
            return None;
        }

        let new_path = self.path.replacen(&remove, &new_root, 1);

        Path::new(new_path)
    }

    pub fn join<S: Into<String>>(&self, node: S) -> Option<Self> {
        let node: String = node.into();
        let path = path::Path::new(&self.path).join(node);
        let path = path.abs()?;
        Path::new(path)
    }

    pub fn exists(&self) -> bool {
        path::Path::new(&self.path).exists()
    }

    pub fn contains(&self, other: &Self) -> bool {
        self.path.starts_with(&other.path)
    }

    pub fn is_dir(&self) -> bool {
        path::Path::new(&self.path).is_dir()
    }

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

        let path = Path::new("/path").unwrap();
        assert_eq!(path.name, "path");
        assert_eq!(path.path, "/path");
        assert_eq!(path.parent, "/");

        let path = Path::new("/path/").unwrap();
        assert_eq!(path.name, "path");
        assert_eq!(path.path, "/path");
        assert_eq!(path.parent, "/");
        let path = Path::new("/").unwrap();

        assert_eq!(path.name, "");
        assert_eq!(path.path, "/");
        assert_eq!(path.parent, "/");

        let path = Path::new("/path/to/file").unwrap();
        assert_eq!(path.name, "file");
        assert_eq!(path.path, "/path/to/file");
        assert_eq!(path.parent, "/path/to");

        let path = Path::new("/path/./to//nope/../file").unwrap();
        assert_eq!(path.name, "file");
        assert_eq!(path.path, "/path/to/file");
        assert_eq!(path.parent, "/path/to");

        let path = Path::new("./path/to/file").unwrap();
        assert_eq!(path.name, "file");
        assert_eq!(path::Path::new(&path.path), cwd.join("path/to/file"));
        assert_eq!(path::Path::new(&path.parent), cwd.join("path/to"));

        let path = Path::new("path/to/file").unwrap();
        assert_eq!(path.name, "file");
        assert_eq!(path::Path::new(&path.path), cwd.join("path/to/file"));
        assert_eq!(path::Path::new(&path.parent), cwd.join("path/to"));
    }

    #[test]
    fn test_change_root() {
        let path = Path::new("/some/path").unwrap();
        let new_path = path.with_root("/some", "/").unwrap();
        assert_eq!(new_path.name, "path");
        assert_eq!(new_path.path, "/path");
        assert_eq!(new_path.parent, "/");

        let path = Path::new("/some/path").unwrap();
        let new_path = path.with_root("/some/", "/").unwrap();
        assert_eq!(new_path.name, "path");
        assert_eq!(new_path.path, "/path");
        assert_eq!(new_path.parent, "/");

        let path = Path::new("/some/longer/path").unwrap();
        let new_path = path.with_root("/some", "/a").unwrap();
        assert_eq!(new_path.name, "path");
        assert_eq!(new_path.path, "/a/longer/path");
        assert_eq!(new_path.parent, "/a/longer");

        let path = Path::new("/some///////////longer/path").unwrap();
        let new_path = path.with_root("/some", "/a/").unwrap();
        assert_eq!(new_path.name, "path");
        assert_eq!(new_path.path, "/a/longer/path");
        assert_eq!(new_path.parent, "/a/longer");

        let path = Path::new("/some/longer/path").unwrap();
        let new_path = path.with_root("/some/", "/a/").unwrap();
        assert_eq!(new_path.name, "path");
        assert_eq!(new_path.path, "/a/longer/path");
        assert_eq!(new_path.parent, "/a/longer");

        let path = Path::new("/some/longer/path").unwrap();
        let new_path = path.with_root("/som", "/a");
        assert!(new_path.is_none());
    }

    #[test]
    fn test_join_path() {
        let path = Path::new("/a").unwrap();
        let new_path = path.join("b").unwrap();
        assert_eq!(new_path.name, "b");
        assert_eq!(new_path.path, "/a/b");
        assert_eq!(new_path.parent, "/a");
    }

    #[test]
    fn test_contains_path() {
        let path1 = Path::new("/a/b/c").unwrap();
        let path2 = Path::new("/a").unwrap();
        let contains = path1.contains(&path2);
        assert_eq!(contains, true);
    }
}
