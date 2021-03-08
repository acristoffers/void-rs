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

use super::crypto;
use super::path::Path;
use flexbuffers::{FlexbufferSerializer, Reader};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Display;
use std::fs;
use std::io::{Read, Write};

#[derive(Debug, PartialEq)]
pub enum Error {
    CannotCreateDirectoryError,
    CannotCreateFileError,
    CannotDecryptFileError,
    CannotDeserializeError,
    CannotParseError,
    CannotReadFileError,
    CannotRemoveFilesError(Vec<String>),
    CannotSerializeError,
    CannotWriteFileError,
    FileAlreadyExistsError,
    FileDoesNotExistError,
    FolderDoesNotExistError,
    StoreFileAlreadyExistsError,
    InternalStructureError,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for Error {}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct INode {
    id: u64,
    parent: u64,
    name: String,
    size: u64,
    is_file: bool,
    metadata: HashMap<String, String>,
    data: Vec<u64>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct Data {
    id: u64,
    key: [u8; 32],
    iv: [u8; 16],
    salt: [u8; 16],
}

trait FlexBufferSerializable {
    fn fb_serialize(&self) -> Result<Vec<u8>, Error>;
    fn fb_deserialize(bytes: &[u8]) -> Result<Box<Self>, Error>;
}

#[derive(Serialize, Deserialize)]
struct StoreFile {
    inodes: Vec<u8>,
    inodes_hash: [u8; 32],
    data: Vec<u8>,
    data_hash: [u8; 32],
    iv: [u8; 16],
    salt: [u8; 16],
}

impl FlexBufferSerializable for StoreFile {
    fn fb_serialize(&self) -> Result<Vec<u8>, Error> {
        let mut bytes = FlexbufferSerializer::new();
        match self.serialize(&mut bytes) {
            Ok(_) => Ok(bytes.view().into()),
            Err(_) => Err(Error::CannotSerializeError),
        }
    }

    fn fb_deserialize(bytes: &[u8]) -> Result<Box<Self>, Error> {
        let reader = match Reader::get_root(bytes) {
            Ok(reader) => reader,
            Err(_) => return Err(Error::CannotDeserializeError),
        };

        match StoreFile::deserialize(reader) {
            Ok(store) => Ok(Box::new(store)),
            Err(_) => return Err(Error::CannotDeserializeError),
        }
    }
}

impl FlexBufferSerializable for Vec<INode> {
    fn fb_serialize(&self) -> Result<Vec<u8>, Error> {
        let mut bytes = FlexbufferSerializer::new();
        match self.serialize(&mut bytes) {
            Ok(_) => Ok(bytes.view().into()),
            Err(_) => Err(Error::CannotSerializeError),
        }
    }

    fn fb_deserialize(bytes: &[u8]) -> Result<Box<Self>, Error> {
        let reader = match Reader::get_root(bytes) {
            Ok(reader) => reader,
            Err(_) => return Err(Error::CannotDeserializeError),
        };

        match Vec::deserialize(reader) {
            Ok(vector) => Ok(Box::new(vector)),
            Err(_) => return Err(Error::CannotDeserializeError),
        }
    }
}

impl FlexBufferSerializable for Vec<Data> {
    fn fb_serialize(&self) -> Result<Vec<u8>, Error> {
        let mut bytes = FlexbufferSerializer::new();
        match self.serialize(&mut bytes) {
            Ok(_) => Ok(bytes.view().into()),
            Err(_) => Err(Error::CannotSerializeError),
        }
    }

    fn fb_deserialize(bytes: &[u8]) -> Result<Box<Self>, Error> {
        let reader = match Reader::get_root(bytes) {
            Ok(reader) => reader,
            Err(_) => return Err(Error::CannotDeserializeError),
        };

        match Vec::deserialize(reader) {
            Ok(vector) => Ok(Box::new(vector)),
            Err(_) => return Err(Error::CannotDeserializeError),
        }
    }
}

#[derive(Debug)]
pub struct Store {
    inodes: Box<Vec<INode>>,
    data: Box<Vec<Data>>,
    iv: [u8; 16],
    key: [u8; 32],
    path: String,
    salt: [u8; 16],
}

impl Store {
    /// Saves the content of store to file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path of the store.
    /// * `password` - Password that encrypts the store.
    fn save(&mut self) -> Result<(), Error> {
        let store_folder = Path::new(&self.path).ok_or(Error::CannotParseError)?;
        let store_journal = store_folder
            .join("Store.void")
            .ok_or(Error::CannotParseError)?;

        if !store_journal.exists() {
            return Err(Error::FileDoesNotExistError);
        }

        self.inodes.sort_by_key(|inode| inode.id);
        self.data.sort_by_key(|data| data.id);

        let inodes_bytes = match self.inodes.fb_serialize() {
            Ok(bytes) => bytes,
            Err(err) => return Err(err),
        };

        let data_bytes = match self.data.fb_serialize() {
            Ok(bytes) => bytes,
            Err(err) => return Err(err),
        };

        let key = &self.key;
        let iv = &self.iv;
        let inodes = crypto::encrypt(inodes_bytes.as_slice(), key, iv);
        let inodes_hash_vec = crypto::hash(inodes.as_slice(), &self.salt);
        let data = crypto::encrypt(data_bytes.as_slice(), key, iv);
        let data_hash_vec = crypto::hash(data.as_slice(), &self.salt);
        let mut inodes_hash = [0u8; 32];
        let mut data_hash = [0u8; 32];

        inodes_hash.copy_from_slice(&inodes_hash_vec);
        data_hash.copy_from_slice(&data_hash_vec);

        let store_file = StoreFile {
            inodes,
            inodes_hash,
            data,
            data_hash,
            iv: self.iv,
            salt: self.salt,
        };

        let serialized = match store_file.fb_serialize() {
            Ok(bytes) => bytes,
            Err(err) => return Err(err),
        };

        match fs::write(store_journal.path, serialized.as_slice()) {
            Ok(_) => Ok(()),
            Err(_) => Err(Error::CannotWriteFileError),
        }
    }

    /// Creates a new store and return a Store object.
    ///
    /// # Arguments
    ///
    /// * `path` - Path where the store should be created.
    /// * `password` - Password that encrypts the store.
    pub fn create<S: Into<String>>(path: S, password: S) -> Result<Store, Error> {
        let path: String = path.into();
        let password: String = password.into();

        let store_folder = Path::new(path).ok_or(Error::CannotParseError)?;
        let store_journal = store_folder
            .join("Store.void")
            .ok_or(Error::CannotParseError)?;

        if store_folder.exists() {
            return Err(Error::FileAlreadyExistsError);
        }

        if let Err(_) = fs::create_dir_all(&store_folder.path) {
            return Err(Error::CannotCreateDirectoryError);
        }

        if let Err(_) = fs::write(store_journal.path, "") {
            return Err(Error::CannotWriteFileError);
        }

        let salt = crypto::uuid();
        let iv = crypto::uuid();
        let key = crypto::derive_key(&password, &salt, &iv);

        let root = INode {
            id: 0,
            parent: 0,
            name: String::new(),
            size: 0,
            is_file: false,
            metadata: HashMap::new(),
            data: vec![],
        };

        let mut store = Store {
            inodes: Box::new(vec![root]),
            data: Box::new(vec![]),
            iv,
            key,
            path: store_folder.path,
            salt,
        };

        store.save().map(|_| store)
    }

    /// Opens an existing store and return a Store object.
    ///
    /// # Arguments
    ///
    /// * `path` - Path of the store to be opened.
    /// * `password` - Password that encrypts the store.
    pub fn open<S: Into<String>>(path: S, password: S) -> Result<Store, Error> {
        let path: String = path.into();
        let password: String = password.into();

        let store_folder = Path::new(path).ok_or(Error::CannotParseError)?;
        let store_journal = store_folder
            .join("Store.void")
            .ok_or(Error::CannotParseError)?;

        if !store_folder.exists() {
            return Err(Error::FolderDoesNotExistError);
        } else if !store_journal.exists() {
            return Err(Error::FileDoesNotExistError);
        }

        let bytes = fs::read(store_journal.path).map_err(|_| Error::CannotReadFileError)?;
        let store_file = StoreFile::fb_deserialize(bytes.as_slice())?;

        let salt = store_file.salt;
        let iv = store_file.iv;
        let key = crypto::derive_key(password.as_str(), &salt, &iv);

        let inodes = store_file.inodes.as_slice();
        let inodes = crypto::decrypt(inodes, &key, &iv);
        let inodes = inodes.map_err(|_| Error::CannotDecryptFileError)?;

        let data = store_file.data.as_slice();
        let data = crypto::decrypt(data, &key, &iv);
        let data = data.map_err(|_| Error::CannotDecryptFileError)?;

        let inodes = Vec::fb_deserialize(inodes.as_slice())?;
        let data = Vec::fb_deserialize(data.as_slice())?;

        let store = Store {
            inodes,
            data,
            iv,
            key,
            path: store_folder.path,
            salt,
        };

        Ok(store)
    }

    /// Encrypts a file and adds it to the store.
    /// The arguments work like the `cp` unix command when it comes to trailling
    /// slashes.
    ///
    /// # Arguments
    ///
    /// * `file_path` - File path in the disk.
    /// * `store_path` - Path in store where to save.
    pub fn add<S: Into<String>>(&mut self, file_path: S, store_path: S) -> Result<(), Error> {
        let file_path: String = file_path.into();
        let store_path: String = store_path.into();

        let ends_in_slash = store_path.ends_with("/");
        let file_path = Path::new(file_path).ok_or(Error::CannotParseError)?;
        let store_path = Path::new(&store_path).ok_or(Error::CannotParseError)?;
        let store_folder = Path::new(&self.path).ok_or(Error::CannotParseError)?;
        let store_journal = store_folder
            .join("Store.void")
            .ok_or(Error::CannotParseError)?;

        if !store_folder.exists() {
            return Err(Error::FolderDoesNotExistError);
        } else if !store_journal.exists() || !file_path.exists() {
            return Err(Error::FileDoesNotExistError);
        }

        if file_path.is_dir() {
            for entry in walkdir::WalkDir::new(&file_path.path)
                .follow_links(false)
                .into_iter()
                .filter_map(Result::ok)
            {
                if entry
                    .metadata()
                    .map_err(|_| Error::CannotReadFileError)?
                    .is_dir()
                {
                    continue;
                }

                let file_path = if ends_in_slash {
                    file_path.parent.clone()
                } else {
                    file_path.path.clone()
                };

                let entry_path: Path = entry.path().to_path_buf().into();
                let store_path = entry_path
                    .with_root(&file_path, &store_path.path)
                    .ok_or(Error::CannotParseError)?;

                self.add(entry_path.path, store_path.path)?;
            }
        } else {
            let file_handle = fs::File::open(&file_path.path);
            let mut file_handle = file_handle.map_err(|_| Error::CannotReadFileError)?;

            let file_std_path = std::path::Path::new(&file_path.path);
            let mut metadata: HashMap<String, String> = HashMap::new();
            metadata.insert("mimetype".into(), tree_magic::from_filepath(&file_std_path));

            let file_name = file_path.name;
            let file_size = file_std_path
                .metadata()
                .map_err(|_| Error::CannotReadFileError)?
                .len();

            // is path /folder/folder/ ?
            let (store_path, file_name) = if ends_in_slash {
                // verify if /folder/folder/file exists.
                // verify if /folder/folder does not contain file in path.

                // File already exists?
                let path = store_path.join(&file_name).ok_or(Error::CannotParseError)?;
                if self.inode_with_path(path.path).is_ok() {
                    return Err(Error::StoreFileAlreadyExistsError);
                }

                // Path contains a file?
                // like /folder/file.txt/folder/file.jpg
                let mut path = Path::new("/").ok_or(Error::CannotParseError)?;
                for component in store_path.components() {
                    if let Some(inode) = self.inode_with_path(&path.path).ok() {
                        if inode.is_file {
                            return Err(Error::StoreFileAlreadyExistsError);
                        }

                        path = path.join(component).ok_or(Error::CannotParseError)?;
                    } else {
                        break;
                    }
                }

                (store_path, file_name)
            } else {
                // is /folder/folder a folder?
                if let Some(node) = self.inode_with_path(&store_path.path).ok() {
                    if node.is_file {
                        return Err(Error::StoreFileAlreadyExistsError);
                    } else {
                        // File already exists?
                        let path = store_path.join(&file_name).ok_or(Error::CannotParseError)?;
                        if let Some(_) = self.inode_with_path(path.path).ok() {
                            return Err(Error::StoreFileAlreadyExistsError);
                        }
                    }

                    (store_path, file_name)
                } else {
                    let file_name = store_path.name;
                    let store_path = Path::new(store_path.parent).ok_or(Error::CannotParseError)?;

                    (store_path, file_name)
                }
            };

            self.inode_mkdirp(&store_path.path)?;
            let mut inode = INode {
                id: self.inodes.iter().map(|node| node.id).max().unwrap_or(0) + 1,
                parent: self.inode_with_path(&store_path.path)?.id,
                name: file_name.into(),
                size: file_size,
                is_file: true,
                metadata,
                data: vec![],
            };

            let mut bytes = vec![0u8; 52428800];
            loop {
                let bytes_read = match file_handle.read(bytes.as_mut_slice()) {
                    Ok(size) => size,
                    Err(_) => {
                        for part in inode.data {
                            let part_name = hex::encode(part.to_string());
                            let part_name = format!("{:0>32}", part_name);
                            let part_file = store_folder
                                .join(part_name)
                                .ok_or(Error::CannotParseError)?;
                            fs::remove_file(part_file.path).unwrap_or(());
                        }
                        return Err(Error::CannotReadFileError);
                    }
                };

                if bytes_read == 0 {
                    break;
                }

                let salt = crypto::uuid();
                let iv = crypto::uuid();
                let pswd = hex::encode(crypto::uuid());
                let key = crypto::derive_key(&pswd, &salt, &iv);

                let data = Data {
                    id: self
                        .data
                        .iter()
                        .map(|data| data.id)
                        .max()
                        .unwrap_or_else(|| 0)
                        + 1,
                    key,
                    iv,
                    salt,
                };

                let bytes_read = &bytes[..bytes_read];
                let content = crypto::encrypt(bytes_read, &key, &iv);
                let part_name = hex::encode(data.id.to_string());
                let part_name = format!("{:0>32}", part_name);
                let part_file = store_folder
                    .join(part_name)
                    .ok_or(Error::CannotParseError)?;

                if let Err(_) = fs::write(part_file.path, content) {
                    for part in inode.data {
                        let part_name = hex::encode(part.to_string());
                        let part_name = format!("{:0>32}", part_name);
                        let part_file = store_folder
                            .join(part_name)
                            .ok_or(Error::CannotParseError)?;
                        fs::remove_file(part_file.path).unwrap_or(());
                        let data = self
                            .data
                            .iter()
                            .filter(|data| data.id != part)
                            .map(|data| data.clone())
                            .collect();
                        self.data = Box::new(data);
                    }
                    return Err(Error::CannotWriteFileError);
                };

                self.data.push(data.clone());
                inode.data.push(data.id);
            }

            self.inodes.push(inode.clone());
            self.inode_add_child(inode.parent, inode.id)?;
            if let Err(err) = self.save() {
                for part in inode.data {
                    let part_name = hex::encode(part.to_string());
                    let part_name = format!("{:0>32}", part_name);
                    let part_file = store_folder
                        .join(part_name)
                        .ok_or(Error::CannotParseError)?;
                    fs::remove_file(part_file.path).unwrap_or(());
                    let data = self
                        .data
                        .iter()
                        .filter(|data| data.id != part)
                        .map(|data| data.clone())
                        .collect();
                    self.data = Box::new(data);
                }
                return Err(err);
            }
        }

        Ok(())
    }

    /// Decrypts a file from the store and saves it on disk.
    ///
    /// # Arguments
    ///
    /// * `store_path` - Path of folder/file in the store.
    /// * `file_path` - Path in the disk where to save.
    pub fn get<S: Into<String>>(&self, store_path: S, file_path: S) -> Result<(), Error> {
        let store_path = Path::new(store_path).ok_or(Error::CannotParseError)?;
        let file_path = Path::new(file_path).ok_or(Error::CannotParseError)?;

        if file_path.exists() {
            return Err(Error::FileAlreadyExistsError);
        }

        let inode = self.inode_with_path(&store_path.path)?.clone();
        let files: Vec<INode> = if !inode.is_file {
            self.inode_walk(inode)
                .into_iter()
                .filter(|node| node.is_file)
                .collect()
        } else {
            vec![inode]
        };

        for file in &files {
            let disk_path = Path::new(self.path_for_inode(file.clone()))
                .ok_or(Error::CannotParseError)?
                .with_root(&store_path.path, &file_path.path)
                .ok_or(Error::CannotParseError)?;

            if !Path::new(&disk_path.parent)
                .ok_or(Error::CannotParseError)?
                .exists()
            {
                fs::create_dir_all(disk_path.parent)
                    .map_err(|_| Error::CannotCreateDirectoryError)?;
            }

            let file_handle = fs::File::create(&disk_path.path);
            let mut file_handle = file_handle.map_err(|_| Error::CannotWriteFileError)?;

            let store_path = Path::new(&self.path).ok_or(Error::CannotParseError)?;
            for part_id in &file.data {
                let part_name = hex::encode(part_id.to_string());
                let part_name = format!("{:0>32}", part_name);
                let part_path = store_path.join(part_name).ok_or(Error::CannotParseError)?;
                let cipher = fs::read(part_path.path).map_err(|_| Error::CannotReadFileError)?;
                let data = self.data_with_id(*part_id)?;
                let content = crypto::decrypt(cipher.as_slice(), &data.key, &data.iv);
                let content = content.map_err(|_| Error::CannotDecryptFileError)?;

                file_handle
                    .write_all(content.as_slice())
                    .map_err(|_| Error::CannotWriteFileError)?;
            }
        }

        Ok(())
    }

    /// Removes a file or folder from the store.
    ///
    /// # Arguments
    ///
    /// * `path` - Path of folder/file in the store.
    pub fn remove<S: Into<String>>(&mut self, path: S) -> Result<(), Error> {
        let path = Path::new(path.into()).ok_or(Error::CannotParseError)?;
        let inode = self.inode_with_path(path.path)?.clone();
        let inodes = self.inode_walk(inode.clone());
        let store_path = Path::new(&self.path).ok_or(Error::CannotParseError)?;
        let mut data = inode.data.clone();

        self.inode_remove_child(inode.parent, inode.id)?;

        if inode.id != 0 {
            self.inodes.retain(|node| node.id != inode.id);
        }

        for inode in inodes {
            data.extend(inode.data.clone());
            self.inodes.retain(|node| node.id != inode.id);
        }

        for id in data {
            let part_name = hex::encode(id.to_string());
            let part_name = format!("{:0>32}", part_name);
            let part_path = store_path.join(part_name).ok_or(Error::CannotParseError)?;

            self.data.retain(|data| data.id != id);
            fs::remove_file(part_path.path).ok();
        }

        self.save()?;

        Ok(())
    }

    /// Lists files in the store.
    ///
    /// # Arguments
    ///
    /// * `path` - Path of folder/file in the store.
    pub fn list<S: Into<String>>(&self, path: S) -> Result<Vec<(String, u64, bool)>, Error> {
        let path = Path::new(path.into()).ok_or(Error::CannotParseError)?;
        let inode = self.inode_with_path(path.path)?;

        let files = if inode.is_file {
            vec![(inode.name.clone(), inode.size, !inode.is_file)]
        } else {
            inode
                .data
                .iter()
                .map(|&id| self.inode_with_id(id).ok())
                .filter_map(|node| node)
                .map(|node| (node.name.clone(), node.size, !node.is_file))
                .collect()
        };

        Ok(files)
    }

    pub fn metadata_set<S: Into<String>>(
        &mut self,
        path: S,
        key: S,
        value: S,
    ) -> Result<(), Error> {
        let path = Path::new(path.into()).ok_or(Error::CannotParseError)?;
        let mut inode = self.inode_with_path(path.path)?.clone();
        let key: String = key.into();
        let value: String = value.into();

        inode.metadata.insert(key, value);

        self.inodes.retain(|node| node.id != inode.id);
        self.inodes.push(inode);

        self.save()?;

        Ok(())
    }

    pub fn metadata_remove<S: Into<String>>(&mut self, path: S, key: S) -> Result<(), Error> {
        let path = Path::new(path.into()).ok_or(Error::CannotParseError)?;
        let mut inode = self.inode_with_path(path.path)?.clone();
        let key: String = key.into();

        inode.metadata.remove(&key);

        self.inodes.retain(|node| node.id != inode.id);
        self.inodes.push(inode);

        self.save()?;

        Ok(())
    }

    pub fn metadata_get<S: Into<String>>(&self, path: S, key: S) -> Result<&String, Error> {
        let path = Path::new(path.into()).ok_or(Error::CannotParseError)?;
        let inode = self.inode_with_path(path.path)?;
        let key: String = key.into();

        inode
            .metadata
            .get(&key)
            .ok_or(Error::InternalStructureError)
    }

    pub fn metadata_list<S: Into<String>>(
        &self,
        path: S,
    ) -> Result<HashMap<String, String>, Error> {
        let path = Path::new(path.into()).ok_or(Error::CannotParseError)?;
        let inode = self.inode_with_path(path.path)?;

        Ok(inode.metadata.clone())
    }

    fn inode_with_id(&self, id: u64) -> Result<&INode, Error> {
        let inode = self.inodes.iter().find(|inode| inode.id == id);
        inode.ok_or(Error::InternalStructureError)
    }

    fn data_with_id(&self, id: u64) -> Result<&Data, Error> {
        let data = self.data.iter().find(|data| data.id == id);
        data.ok_or(Error::InternalStructureError)
    }

    fn inode_with_path<S: Into<String>>(&self, path: S) -> Result<&INode, Error> {
        let path = Path::new(path.into()).ok_or(Error::CannotParseError)?;

        let mut node = self.inode_with_id(0)?;
        if path.path == "/" {
            return Ok(node);
        }

        for component in path.components() {
            if component == "/" {
                // Since path is normalized, it skips only the root component.
                continue;
            } else if let Some(next_node) = node
                .data
                .iter()
                .map(|&id| self.inode_with_id(id).ok())
                .filter_map(|r| r)
                .find(|node| node.name == component)
            {
                node = next_node
            } else {
                return Err(Error::InternalStructureError);
            }
        }

        if node.name == path.name {
            Ok(node)
        } else {
            Err(Error::InternalStructureError)
        }
    }

    fn path_for_inode(&self, inode: INode) -> String {
        let mut inode = inode;
        let mut components = vec![];
        while inode.id != 0 {
            components.insert(0, inode.name);
            inode = self.inode_with_id(inode.parent).unwrap().clone();
        }
        components.insert(0, "".into());
        components.join("/")
    }

    fn inode_walk(&self, inode: INode) -> Vec<INode> {
        let mut inodes = vec![];

        if !inode.is_file {
            for id in inode.data {
                let child = self.inode_with_id(id).unwrap();
                inodes.push(child.clone());
                if !child.is_file {
                    let mut children = self.inode_walk(child.clone());
                    inodes.append(&mut children);
                }
            }
        }

        inodes
    }

    fn inode_mkdirp<S: Into<String>>(&mut self, path: S) -> Result<(), Error> {
        let path = Path::new(path.into()).ok_or(Error::CannotParseError)?;

        let mut node = self.inode_with_id(0)?.clone();
        for component in path.components() {
            if component == "/" {
                continue;
            }

            if let Some(child) = node
                .data
                .iter()
                .map(|&id| self.inode_with_id(id).ok())
                .filter_map(|node| node)
                .find(|node| node.name == component)
            {
                node = child.clone();
            } else {
                let inode = INode {
                    id: self.inodes.iter().map(|node| node.id).max().unwrap_or(0) + 1,
                    parent: node.id,
                    name: component,
                    size: 0,
                    is_file: false,
                    metadata: HashMap::new(),
                    data: vec![],
                };
                self.inodes.push(inode.clone());
                node = self.inode_with_id(inode.id)?.clone();
                self.inode_add_child(node.parent, node.id)?;
            }
        }

        Ok(())
    }

    fn inode_add_child(&mut self, parent_id: u64, child_id: u64) -> Result<(), Error> {
        let mut parent = self.inode_with_id(parent_id)?.clone();
        let child = self.inode_with_id(child_id)?.clone();

        if parent.is_file {
            return Err(Error::InternalStructureError);
        }

        parent.data.push(child.id);
        parent.data.sort();
        parent.data.dedup();

        self.inodes.retain(|node| node.id != parent.id);
        self.inodes.push(parent);

        Ok(())
    }

    fn inode_remove_child(&mut self, parent_id: u64, child_id: u64) -> Result<(), Error> {
        let mut parent = self.inode_with_id(parent_id)?.clone();
        let child = self.inode_with_id(child_id)?.clone();

        if parent.is_file {
            return Err(Error::InternalStructureError);
        }

        parent.data.retain(|&id| id != child.id);
        self.inodes.retain(|node| node.id != parent.id);
        self.inodes.push(parent);

        Ok(())
    }
}
