/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use super::path::VirtualPath;
use super::store::Error;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Node {
    id: u64,
    name: String,
    size: u64,
    is_file: bool,
    metadata: HashMap<String, String>,
    data: Vec<u64>,
    tags: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Data {
    pub id: u64,
    pub key: [u8; 32],
    pub iv: [u8; 16],
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Filesystem {
    data: HashMap<u64, Data>,
    nodes: HashMap<u64, Node>,
    graph: HashMap<u64, Vec<u64>>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct File {
    pub id: u64,
    pub name: String,
    pub size: u64,
    pub is_file: bool,
    pub metadata: HashMap<String, String>,
    pub data: Vec<Data>,
    pub tags: Vec<String>,
}

impl Filesystem {
    /// Creates a new, empty filesystem
    pub fn new() -> Filesystem {
        Filesystem {
            data: HashMap::new(),
            nodes: HashMap::new(),
            graph: HashMap::new(),
        }
    }

    /// Returns a node id which is not in use on the filesystem.
    fn next_node_id(&self) -> u64 {
        let len = self.nodes.len() as u64;
        match self
            .nodes
            .keys()
            .copied()
            .sorted()
            .zip(1..=len)
            .find(|(id, index)| id != index)
        {
            Some((_, id)) => id,
            None => len + 1,
        }
    }

    /// Returns a data id which is not in use on the filesystem.
    fn next_data_id(&self) -> u64 {
        let len = self.data.len() as u64;
        match self
            .data
            .keys()
            .copied()
            .sorted()
            .zip(1..=len)
            .find(|(id, index)| id != index)
        {
            Some((_, id)) => id,
            None => len + 1,
        }
    }

    /// Returns the id of the node at `path`, or `None` if it does not exist.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to be checked.
    pub fn exists(&self, path: &str) -> Result<Option<u64>, Error> {
        let path: String = path.into();
        let path = VirtualPath::new(&path).ok_or(Error::CannotParseError)?;
        let mut node_id: u64 = 0;
        let default: Vec<u64> = vec![];
        for component in path.components() {
            if component == "/" {
                continue;
            }
            match self
                .graph
                .get(&node_id)
                .unwrap_or(&default)
                .iter()
                .filter_map(|&id| self.nodes.get(&id))
                .find(|node| node.name == component)
            {
                Some(node) => node_id = node.id,
                None => return Ok(None),
            }
        }
        Ok(Some(node_id))
    }

    /// Creates a folder or folder tree.
    /// Works like the mkdir -p command on Unix systems.
    ///
    /// # Arguments
    ///
    /// * `path` - Path of the folder to be created.
    ///
    /// # Returns
    ///
    /// * The id of the innermost folder in the path
    pub fn mkdirp(&mut self, path: &str) -> Result<u64, Error> {
        let path: String = path.into();
        let path = VirtualPath::new(&path).ok_or(Error::CannotParseError)?;
        let mut node_id: u64 = 0;
        for component in path.components() {
            if component == "/" {
                continue;
            }
            let children = self.graph.get(&node_id).cloned().unwrap_or_default();
            let found = children
                .iter()
                .filter_map(|&id| self.nodes.get(&id))
                .find(|node| node.name == component)
                .map(|node| (node.id, node.is_file));
            match found {
                Some((id, is_file)) => {
                    if is_file {
                        return Err(Error::CannotCreateDirectoryError);
                    }
                    node_id = id;
                }
                None => {
                    let new_id = self.next_node_id();
                    let mut new_entry = children;
                    new_entry.push(new_id);
                    new_entry.sort();
                    self.graph.insert(node_id, new_entry);
                    self.nodes.insert(
                        new_id,
                        Node {
                            id: new_id,
                            name: component,
                            size: 0,
                            is_file: false,
                            metadata: HashMap::new(),
                            data: vec![],
                            tags: vec![],
                        },
                    );
                    node_id = new_id;
                }
            }
        }
        Ok(node_id)
    }

    /// Creates a file.
    /// Creates the path if it does not exists, creates the file and returns its
    /// id. If the file already exists, returns its id.
    ///
    /// # Arguments
    ///
    /// * `path` - Path of the file.
    ///
    /// # Returns
    ///
    /// * The id of the file
    pub fn touch(&mut self, path: &str) -> Result<u64, Error> {
        if path == "/" {
            return Ok(0);
        }
        let path: String = path.into();
        let path = VirtualPath::new(&path).ok_or(Error::CannotParseError)?;
        let parent_id = self.mkdirp(&path.parent)?;
        let children = self.graph.get(&parent_id).cloned().unwrap_or_default();
        let found = children
            .iter()
            .filter_map(|&id| self.nodes.get(&id))
            .find(|node| node.name == path.name)
            .map(|node| node.id);
        match found {
            Some(id) => Ok(id),
            None => {
                let new_id = self.next_node_id();
                let mut new_children = vec![new_id];
                new_children.extend(&children);
                self.graph.insert(parent_id, new_children);
                self.nodes.insert(
                    new_id,
                    Node {
                        id: new_id,
                        name: path.name,
                        size: 0,
                        is_file: true,
                        metadata: HashMap::new(),
                        data: vec![],
                        tags: vec![],
                    },
                );
                Ok(new_id)
            }
        }
    }

    /// Returns a File object containing information about a node and associated data.
    ///
    /// # Arguments
    ///
    /// * `id` - Id of the node.
    ///
    /// # Returns
    ///
    /// * A File object with all pertaining information.
    pub fn get(&self, id: u64) -> Result<File, Error> {
        if id == 0 {
            return Ok(File {
                id: 0,
                name: "/".into(),
                size: 0,
                is_file: false,
                metadata: HashMap::new(),
                data: vec![],
                tags: vec![],
            });
        }
        let node = self.nodes.get(&id).ok_or(Error::FileDoesNotExistError)?;
        let file = File {
            id: node.id,
            name: node.name.clone(),
            size: node.size,
            is_file: node.is_file,
            metadata: node.metadata.clone(),
            tags: node.tags.clone(),
            data: node
                .data
                .iter()
                .filter_map(|id| self.data.get(id))
                .cloned()
                .collect(),
        };
        Ok(file)
    }

    /// Sets the size of a file.
    ///
    /// # Arguments
    ///
    /// * `id` - Id of the file to change size.
    /// * `size` - New size.
    pub fn set_size(&mut self, id: u64, size: u64) -> Result<(), Error> {
        let node = self
            .nodes
            .get_mut(&id)
            .ok_or(Error::InternalStructureError)?;
        if !node.is_file {
            return Err(Error::InternalStructureError);
        }
        node.size = size;
        Ok(())
    }

    /// Lists a folder's children
    ///
    /// # Arguments
    ///
    /// * `id` Folder id to list children.
    ///
    /// # Returns
    ///
    /// * List of children's File structures.
    pub fn ls(&self, id: u64) -> Result<Vec<File>, Error> {
        let children = self
            .graph
            .get(&id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
            .iter()
            .filter_map(|&id| self.get(id).ok())
            .collect();
        Ok(children)
    }

    /// Moves a node
    ///
    /// # Arguments
    ///
    /// * `id` - Id of the node to move;
    /// * `parent` - Id of the new parent;
    pub fn mv(&mut self, id: u64, parent: u64) -> Result<(), Error> {
        if !self.nodes.contains_key(&id) {
            return Err(Error::FileDoesNotExistError);
        }
        if parent != 0 && !self.nodes.contains_key(&parent) {
            return Err(Error::FolderDoesNotExistError);
        }
        let old_parent = self
            .graph
            .iter()
            .find(|(_, v)| v.contains(&id))
            .map(|(&k, _)| k)
            .ok_or(Error::InternalStructureError)?;

        let old_children: Vec<u64> = self
            .graph
            .get(&old_parent)
            .ok_or(Error::InternalStructureError)?
            .iter()
            .filter(|&&cid| cid != id)
            .copied()
            .collect();
        self.graph.insert(old_parent, old_children);

        let mut new_children: Vec<u64> = self.graph.get(&parent).cloned().unwrap_or_default();
        new_children.push(id);
        self.graph.insert(parent, new_children);
        Ok(())
    }

    /// Removes entry. If it is a folder, removes the tree.
    ///
    /// # Arguments
    ///
    /// * `id` - Id of the entry to be removed.
    ///
    /// # Returns
    ///
    /// * A vector of Data objects that were removed.
    pub fn rm(&mut self, id: u64) -> Result<Vec<Data>, Error> {
        if id == 0 {
            let data = self.data.values().cloned().collect();
            self.graph.clear();
            self.nodes.clear();
            self.data.clear();
            return Ok(data);
        }
        let parent = self
            .graph
            .iter()
            .find(|(_, v)| v.contains(&id))
            .map(|(&k, _)| k)
            .ok_or(Error::InternalStructureError)?;
        let default = vec![];
        let children: Vec<u64> = self
            .graph
            .get(&parent)
            .unwrap_or(&default)
            .iter()
            .filter(|&&cid| cid != id)
            .copied()
            .collect();
        self.graph.insert(parent, children);
        self.clean()
    }

    /// Removes unreferenced data and nodes. Does not change ids.
    pub fn clean(&mut self) -> Result<Vec<Data>, Error> {
        let mut size = 0;
        while self.graph.len() != size {
            size = self.graph.len();
            let keys: Vec<u64> = self.graph.keys().copied().collect();
            let vals: Vec<u64> = self.graph.values().flat_map(|v| v.clone()).collect();
            for key in keys {
                if !vals.contains(&key) && key != 0 {
                    self.graph.remove(&key);
                }
            }
        }
        let keep: Vec<u64> = self.graph.values().flatten().copied().collect();
        self.nodes.retain(|id, _| keep.contains(id));
        let data_keep: Vec<u64> = self
            .nodes
            .values()
            .flat_map(|node| node.data.clone())
            .collect();
        let removed_data: Vec<Data> = self
            .data
            .values()
            .filter(|data| !data_keep.contains(&data.id))
            .cloned()
            .collect();
        self.data.retain(|id, _| data_keep.contains(id));
        Ok(removed_data)
    }

    /// Appends Data to a file
    ///
    /// # Arguments
    ///
    /// * `id` - File id to append Data to.
    /// * `data` - Data object to be inserted. The id will be overwritten.
    ///
    /// # Returns
    ///
    /// * The File with the new Data added.
    pub fn append(&mut self, id: u64, data: &Data) -> Result<File, Error> {
        let next_id = self.next_data_id();
        {
            let node = self
                .nodes
                .get_mut(&id)
                .ok_or(Error::FileDoesNotExistError)?;
            if !node.is_file {
                return Err(Error::FileDoesNotExistError);
            }
            node.data.push(next_id);
        }
        let data = Data {
            id: next_id,
            ..*data
        };
        self.data.insert(data.id, data);
        self.get(id)
    }

    /// Truncates file.
    ///
    /// # Arguments
    ///
    /// * `id` - Id of the file to truncate
    pub fn truncate(&mut self, id: u64) -> Result<(), Error> {
        let data_ids: Vec<u64> = {
            let node = self
                .nodes
                .get_mut(&id)
                .ok_or(Error::FileDoesNotExistError)?;
            if !node.is_file {
                return Err(Error::FileDoesNotExistError);
            }
            node.data.drain(..).collect()
        };
        self.data.retain(|id, _| !data_ids.contains(id));
        Ok(())
    }

    /// Sets file/folder metadata
    ///
    /// # Arguments
    ///
    /// * `id` - id of the affected node;
    /// * `key` - metadata key;
    /// * `value` - metadata value;
    pub fn set_metadata(&mut self, id: u64, key: &str, value: &str) -> Result<(), Error> {
        let node = self
            .nodes
            .get_mut(&id)
            .ok_or(Error::FileDoesNotExistError)?;
        node.metadata.insert(key.into(), value.into());
        Ok(())
    }

    /// Gets file/folder metadata
    ///
    /// # Arguments
    ///
    /// * `id` - id of the affected node;
    /// * `key` - metadata key;
    ///
    /// # Returns
    ///
    /// * The value associated with such key.
    pub fn get_metadata(&self, id: u64, key: &str) -> Result<String, Error> {
        let node = self.nodes.get(&id).ok_or(Error::FileDoesNotExistError)?;
        let key: String = key.into();
        match node.metadata.get(&key) {
            Some(value) => Ok(value.clone()),
            None => Err(Error::NoSuchMetadataKey),
        }
    }

    /// Removes a key from the node's metadata
    ///
    /// # Arguments
    ///
    /// * `id` - Id of the affected node;
    /// * `key` - Metadata key;
    pub fn rm_metadata(&mut self, id: u64, key: &str) -> Result<(), Error> {
        let node = self
            .nodes
            .get_mut(&id)
            .ok_or(Error::FileDoesNotExistError)?;
        match node.metadata.remove(key) {
            Some(_) => Ok(()),
            None => Err(Error::NoSuchMetadataKey),
        }
    }

    /// Returns a node's path
    ///
    /// # Arguments
    ///
    /// * `id` - Id of the node.
    ///
    /// # Returns
    ///
    /// * The node's path
    pub fn path(&self, id: u64) -> Result<String, Error> {
        let mut node_id = id;
        let mut path = vec![id];
        while node_id != 0 {
            let (&parent_id, _) = self
                .graph
                .iter()
                .find(|(_, v)| v.contains(&node_id))
                .ok_or(Error::FileDoesNotExistError)?;
            node_id = parent_id;
            path.push(node_id);
        }
        let path: Vec<String> = path
            .iter()
            .rev()
            .filter_map(|&id| self.nodes.get(&id))
            .map(|node| node.name.clone())
            .collect();
        let path: String = "/".to_string() + &path.join("/");
        Ok(path)
    }

    /// Lists all nodes in the store.
    ///
    /// # Returns
    ///
    /// * A list of File objects for all nodes in the store. In this case, the
    ///   name is the full path of the element.
    pub fn ls_all(&self) -> Result<Vec<File>, Error> {
        let nodes = self
            .nodes
            .keys()
            .filter_map(|&id| self.get(id).ok())
            .map(|file| File {
                name: self.path(file.id).unwrap(),
                ..file
            })
            .collect();
        Ok(nodes)
    }

    /// Adds a tag to a file
    ///
    /// # Arguments
    ///
    /// * `id` - Id of the file to add the tag to.
    /// * `tag` - Name of the tag to add.
    pub fn add_tag(&mut self, id: u64, tag: &str) -> Result<(), Error> {
        let tag: String = tag.into();
        let node = self
            .nodes
            .get_mut(&id)
            .ok_or(Error::FileDoesNotExistError)?;
        if !node.tags.contains(&tag) {
            node.tags.push(tag);
        }
        Ok(())
    }

    /// Removes a tag from a node.
    ///
    /// # Arguments
    ///
    /// * `id` - Node's id.
    /// * `tag` - Tag to remove.
    pub fn rm_tag(&mut self, id: u64, tag: &str) -> Result<(), Error> {
        let tag: String = tag.into();
        let node = self
            .nodes
            .get_mut(&id)
            .ok_or(Error::FileDoesNotExistError)?;
        node.tags.retain(|t| t != &tag);
        Ok(())
    }

    /// Clears all tags from a node.
    ///
    /// # Arguments
    ///
    /// * `id` - Node's id.
    pub fn clear_tag(&mut self, id: u64) -> Result<(), Error> {
        let node = self
            .nodes
            .get_mut(&id)
            .ok_or(Error::FileDoesNotExistError)?;
        node.tags.clear();
        Ok(())
    }

    /// Returns the set of all data IDs currently referenced in the store.
    pub fn data_ids(&self) -> std::collections::HashSet<u64> {
        self.data.keys().copied().collect()
    }

    /// List all tags in the filesystem.
    ///
    /// # Returns
    ///
    /// * A list of all tags found in the filesystem.
    pub fn list_tag(&self) -> Vec<String> {
        self.nodes
            .values()
            .flat_map(|node| node.tags.clone())
            .unique()
            .collect()
    }

    /// Lists files that contains or not a certaing tag. Accepts a list of tags
    /// returns a list of File objects for all nodes matching.
    ///
    /// # Arguments
    ///
    /// * `tags` - List of tags to search for. If the tag starts with !, search
    ///   for files not containing that tag.
    ///
    /// # Returns
    ///
    /// * A list of files matching the given tags.
    pub fn search_tag(&self, tags: Vec<String>) -> Vec<File> {
        let (include, exclude): (Vec<String>, Vec<String>) =
            tags.iter().cloned().partition(|tag| !tag.starts_with('!'));
        let exclude: Vec<String> = exclude.iter().map(|tag| tag.replace('!', "")).collect();
        self.nodes
            .values()
            .filter(|node| {
                node.tags.iter().filter(|tag| include.contains(tag)).count() == include.len()
            })
            .filter(|node| node.tags.iter().filter(|tag| exclude.contains(tag)).count() == 0)
            .filter_map(|node| self.get(node.id).ok())
            .map(|file| File {
                name: self.path(file.id).unwrap(),
                ..file
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::super::crypto;
    use super::*;

    #[test]
    fn test_filesystem_next_node_id() {
        let mut fs = Filesystem::new();
        assert_eq!(fs.next_node_id(), 1);
        fs.nodes.insert(
            1,
            Node {
                id: 1,
                name: "".into(),
                size: 0,
                is_file: false,
                metadata: HashMap::new(),
                data: vec![],
                tags: vec![],
            },
        );
        assert_eq!(fs.next_node_id(), 2);
        fs.nodes.insert(
            2,
            Node {
                id: 2,
                name: "".into(),
                size: 0,
                is_file: false,
                metadata: HashMap::new(),
                data: vec![],
                tags: vec![],
            },
        );
        assert_eq!(fs.next_node_id(), 3);
        fs.nodes.insert(
            5,
            Node {
                id: 5,
                name: "".into(),
                size: 0,
                is_file: false,
                metadata: HashMap::new(),
                data: vec![],
                tags: vec![],
            },
        );
        assert_eq!(fs.next_node_id(), 3);
    }

    #[test]
    fn test_filesystem_next_data_id() {
        let rand = crypto::uuid();
        let key = crypto::random_key();
        let mut fs = Filesystem::new();
        assert_eq!(fs.next_data_id(), 1);
        fs.data.insert(
            1,
            Data {
                id: 1,
                key,
                iv: rand,
            },
        );
        assert_eq!(fs.next_data_id(), 2);
        fs.data.insert(
            2,
            Data {
                id: 2,
                key,
                iv: rand,
            },
        );
        assert_eq!(fs.next_data_id(), 3);
        fs.data.insert(
            5,
            Data {
                id: 5,
                key,
                iv: rand,
            },
        );
        assert_eq!(fs.next_data_id(), 3);
    }

    #[test]
    fn test_filesystem_exists() {
        let mut fs = Filesystem::new();
        fs.nodes.insert(
            1,
            Node {
                id: 1,
                name: "f1".into(),
                size: 0,
                is_file: false,
                metadata: HashMap::new(),
                data: vec![],
                tags: vec![],
            },
        );
        fs.nodes.insert(
            2,
            Node {
                id: 2,
                name: "f2".into(),
                size: 0,
                is_file: false,
                metadata: HashMap::new(),
                data: vec![],
                tags: vec![],
            },
        );
        fs.nodes.insert(
            3,
            Node {
                id: 3,
                name: "f3".into(),
                size: 0,
                is_file: false,
                metadata: HashMap::new(),
                data: vec![],
                tags: vec![],
            },
        );
        fs.graph.insert(0, vec![1]);
        fs.graph.insert(1, vec![2]);
        fs.graph.insert(2, vec![3]);
        assert_eq!(fs.exists("/f1/f2/f3").unwrap(), Some(3));
        assert_eq!(fs.exists("/f1/f3/f2").unwrap(), None);
    }

    #[test]
    fn test_filesystem_mkdirp() {
        let mut fs = Filesystem::new();
        fs.mkdirp("/f1/f2/f3").unwrap();
        assert!(fs.exists("/f1/f2/f3").unwrap().is_some());
        fs.mkdirp("/f1/f2/f3/f4").unwrap();
        assert!(fs.exists("/f1/f2/f3/f4").unwrap().is_some());
        fs.nodes.insert(
            10,
            Node {
                id: 10,
                name: "f5".into(),
                size: 0,
                is_file: true,
                metadata: HashMap::new(),
                data: vec![],
                tags: vec![],
            },
        );
        fs.graph.insert(0, vec![1, 10]);
        assert_eq!(fs.mkdirp("/f5/f6"), Err(Error::CannotCreateDirectoryError));
    }

    #[test]
    fn test_filesystem_touch() {
        let mut fs = Filesystem::new();
        let id = fs.touch("/a/b/c").unwrap();
        assert_eq!(id, 3);
        assert!(fs.exists("/a/b/c").unwrap().is_some());
        let id = fs.touch("/a/b/c").unwrap();
        assert_eq!(id, 3);
        assert_eq!(fs.touch("/a/b/c/d"), Err(Error::CannotCreateDirectoryError));
        let node = fs.nodes.get(&id).unwrap();
        assert!(node.is_file)
    }

    #[test]
    fn test_filesystem_get() {
        let mut fs = Filesystem::new();
        let id = fs.touch("/a/b/c").unwrap();
        let file = fs.get(id).unwrap();
        assert_eq!(file.id, id);
        assert!(file.is_file);
        let folder = fs.get(1).unwrap();
        assert_eq!(folder.id, 1);
        assert!(!folder.is_file);
    }

    #[test]
    fn test_filesystem_set_size() {
        let mut fs = Filesystem::new();
        let id = fs.touch("/a/b/c").unwrap();
        let file = fs.get(id).unwrap();
        assert_eq!(file.size, 0);
        fs.set_size(id, 50).unwrap();
        let file = fs.get(id).unwrap();
        assert_eq!(file.size, 50);
    }

    #[test]
    fn test_filesystem_list() {
        let mut fs = Filesystem::new();
        fs.touch("/a/a1").unwrap();
        fs.touch("/a/a2").unwrap();
        fs.touch("/a/a3").unwrap();
        fs.touch("/a/a4").unwrap();
        fs.touch("/a/a5").unwrap();
        let mut children = fs.ls(1).unwrap();
        children.sort_by_key(|file| file.id);
        assert_eq!(children.len(), 5);
        assert_eq!(children[0].name, "a1");
        assert_eq!(children[1].name, "a2");
        assert_eq!(children[2].name, "a3");
        assert_eq!(children[3].name, "a4");
        assert_eq!(children[4].name, "a5");
    }

    #[test]
    fn test_filesystem_mv() {
        let mut fs = Filesystem::new();
        let id = fs.touch("/a/b").unwrap();
        let parent = fs.mkdirp("/c").unwrap();
        fs.mv(id, parent).unwrap();
        let children = fs.ls(1).unwrap();
        assert_eq!(children.len(), 0);
        let children = fs.ls(parent).unwrap();
        assert_eq!(children.len(), 1);
    }

    #[test]
    fn test_filesystem_mv_preserves_existing_children() {
        // Moving into a folder that already has children must not lose them.
        let mut fs = Filesystem::new();
        let src = fs.touch("/a/b").unwrap();
        let dst_parent = fs.mkdirp("/c").unwrap();
        fs.touch("/c/existing").unwrap();

        fs.mv(src, dst_parent).unwrap();

        // /a should be empty (b was moved out)
        let a_children = fs.ls(1).unwrap(); // node 1 is /a
        assert_eq!(a_children.len(), 0);

        // /c should have both "existing" and the moved "b"
        let c_children = fs.ls(dst_parent).unwrap();
        assert_eq!(c_children.len(), 2);
    }

    #[test]
    fn test_filesystem_rm() {
        let mut fs = Filesystem::new();
        fs.touch("/a/b").unwrap();
        fs.touch("/a/c").unwrap();
        let id = fs.touch("/a/d").unwrap();
        let data = Data {
            id: 0,
            key: crypto::random_key(),
            iv: crypto::uuid(),
        };
        fs.append(id, &data).unwrap();
        fs.rm(1).unwrap();
        assert_eq!(fs.nodes.len(), 0);
        assert_eq!(fs.data.len(), 0);
    }

    #[test]
    fn test_filesystem_clean() {
        let mut fs = Filesystem::new();
        fs.touch("/a/b/c/d/e").unwrap();
        fs.touch("/b/c/d/e").unwrap();
        fs.touch("/c/d/e").unwrap();
        fs.touch("/d/e").unwrap();
        let id = fs.touch("/e").unwrap();
        let data = Data {
            id: 0,
            key: crypto::random_key(),
            iv: crypto::uuid(),
        };
        fs.append(id, &data).unwrap();
        fs.graph = HashMap::new();
        fs.graph.insert(0, vec![1]);
        fs.graph.insert(1, vec![2]);
        fs.clean().unwrap();
        assert_eq!(fs.nodes.len(), 2);
        assert_eq!(fs.data.len(), 0);
    }

    #[test]
    fn test_filesystem_append() {
        let mut fs = Filesystem::new();
        let id = fs.touch("/file").unwrap();
        let data = Data {
            id: 0,
            key: crypto::random_key(),
            iv: crypto::uuid(),
        };
        let file = fs.append(id, &data).unwrap();
        assert_eq!(file.data[0].id, 1);
        assert_eq!(file.data.len(), 1);
        fs.append(id, &data).unwrap();
        let file = fs.append(id, &data).unwrap();
        assert_eq!(file.data.len(), 3);
    }

    #[test]
    fn test_filesystem_truncate() {
        let mut fs = Filesystem::new();
        let id = fs.touch("/a").unwrap();
        let data = Data {
            id: 0,
            key: crypto::random_key(),
            iv: crypto::uuid(),
        };
        fs.append(id, &data).unwrap();
        fs.truncate(id).unwrap();
        let file = fs.get(id).unwrap();
        assert_eq!(file.data.len(), 0);
    }

    #[test]
    fn test_filesystem_metadata() {
        let mut fs = Filesystem::new();
        let id = fs.touch("/a/b/c/d").unwrap();
        fs.set_metadata(id, "a", "b").unwrap();
        let val = fs.get_metadata(id, "a").unwrap();
        assert_eq!(val, "b");
        fs.rm_metadata(id, "a").unwrap();
        let val = fs.rm_metadata(id, "a");
        assert_eq!(val, Err(Error::NoSuchMetadataKey));
    }

    #[test]
    fn test_filesystem_path() {
        let mut fs = Filesystem::new();
        let id = fs.touch("/a/b/c/d").unwrap();
        let path = fs.path(id).unwrap();
        assert_eq!(path, "/a/b/c/d");
    }

    #[test]
    fn test_filesystem_add_tag() {
        let mut fs = Filesystem::new();
        let id = fs.touch("/a").unwrap();
        fs.add_tag(id, "tag1").unwrap();
        fs.add_tag(id, "tag2").unwrap();
        fs.add_tag(id, "tag3").unwrap();
        let node = fs.get(id).unwrap();
        let mut tags = node.tags.clone();
        tags.sort();
        assert_eq!(tags[0], "tag1");
        assert_eq!(tags[1], "tag2");
        assert_eq!(tags[2], "tag3");
    }

    #[test]
    fn test_filesystem_rm_tag() {
        let mut fs = Filesystem::new();
        let id = fs.touch("/a").unwrap();
        fs.add_tag(id, "tag1").unwrap();
        fs.add_tag(id, "tag2").unwrap();
        fs.add_tag(id, "tag3").unwrap();
        let node = fs.get(id).unwrap();
        let mut tags = node.tags.clone();
        tags.sort();
        assert_eq!(tags[0], "tag1");
        assert_eq!(tags[1], "tag2");
        assert_eq!(tags[2], "tag3");
        fs.rm_tag(id, "tag2").unwrap();
        let node = fs.get(id).unwrap();
        let mut tags = node.tags.clone();
        tags.sort();
        assert_eq!(tags[0], "tag1");
        assert_eq!(tags[1], "tag3");
    }

    #[test]
    fn test_filesystem_list_tag() {
        let mut fs = Filesystem::new();
        let id = fs.touch("/a").unwrap();
        fs.add_tag(id, "tag1").unwrap();
        fs.add_tag(id, "tag2").unwrap();
        let id = fs.touch("/b").unwrap();
        fs.add_tag(id, "tag3").unwrap();
        let mut tags = fs.list_tag();
        tags.sort();
        assert_eq!(tags[0], "tag1");
        assert_eq!(tags[1], "tag2");
        assert_eq!(tags[2], "tag3");
    }

    #[test]
    fn test_filesystem_clear_tag() {
        let mut fs = Filesystem::new();
        let id = fs.touch("/a").unwrap();
        fs.add_tag(id, "tag1").unwrap();
        fs.add_tag(id, "tag2").unwrap();
        fs.add_tag(id, "tag3").unwrap();
        fs.clear_tag(id).unwrap();
        let tags = fs.list_tag();
        assert_eq!(tags.len(), 0);
    }
}
