/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::filesystem::{Data, Filesystem};
pub use crate::filesystem::File;

use super::crypto;
pub use super::path::{EasyPath, RealPath, VirtualPath};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Display;
use std::fs;
use std::io::{Read, Write};

#[derive(Debug, PartialEq)]
pub enum Error {
    CannotCreateDirectoryError,
    CannotCreateFileError,
    CannotEncryptFileError,
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
    NoSuchMetadataKey,
    InternalStructureError,
    KeyDerivationError,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for Error {}

fn serialize<T: Serialize>(value: &T) -> Result<Vec<u8>, Error> {
    bincode::serialize(value).map_err(|_| Error::CannotSerializeError)
}

fn deserialize<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, Error> {
    bincode::deserialize(bytes).map_err(|_| Error::CannotDeserializeError)
}

#[derive(Serialize, Deserialize)]
struct StoreFile {
    fs: Vec<u8>,
    fs_hash: [u8; 32],
    iv: [u8; 16],
    salt: [u8; 16],
}

#[derive(Debug, Clone)]
pub struct Store {
    fs: Filesystem,
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
        let store_folder = RealPath::new(&self.path).ok_or(Error::CannotParseError)?;
        let store_journal = store_folder
            .join("Store.void")
            .ok_or(Error::CannotParseError)?;

        if !store_journal.exists() {
            return Err(Error::FileDoesNotExistError);
        }

        let fs_bytes = serialize(&self.fs)?;

        let iv = crypto::uuid(); // fresh nonce on every save
        let fs = crypto::encrypt(fs_bytes.as_slice(), &self.key, &iv)?;
        let fs_hash_vec = crypto::hash(fs.as_slice(), &self.salt);
        let mut fs_hash = [0u8; 32];

        fs_hash.copy_from_slice(&fs_hash_vec);

        let store_file = StoreFile {
            fs,
            fs_hash,
            iv,
            salt: self.salt,
        };

        let serialized = serialize(&store_file)?;

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

        let store_folder = RealPath::new(&path).ok_or(Error::CannotParseError)?;
        let store_journal = store_folder
            .join("Store.void")
            .ok_or(Error::CannotParseError)?;

        if store_folder.exists() {
            return Err(Error::FileAlreadyExistsError);
        }

        if fs::create_dir_all(&store_folder.path).is_err() {
            return Err(Error::CannotCreateDirectoryError);
        }

        if fs::write(store_journal.path, "").is_err() {
            return Err(Error::CannotWriteFileError);
        }

        let salt = crypto::uuid();
        let key = crypto::derive_key(&password, &salt)?;

        let mut store = Store {
            fs: Filesystem::new(),
            key,
            path: store_folder.path,
            salt,
        };

        store.save()?;
        Ok(store)
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

        let store_folder = RealPath::new(&path).ok_or(Error::CannotParseError)?;
        let store_journal = store_folder
            .join("Store.void")
            .ok_or(Error::CannotParseError)?;

        if !store_folder.exists() {
            return Err(Error::FolderDoesNotExistError);
        } else if !store_journal.exists() {
            return Err(Error::FileDoesNotExistError);
        }

        let bytes = fs::read(store_journal.path).map_err(|_| Error::CannotReadFileError)?;
        let store_file: StoreFile = deserialize(bytes.as_slice())?;

        let salt = store_file.salt;
        let iv = store_file.iv;

        let computed_hash = crypto::hash(store_file.fs.as_slice(), &salt);
        if computed_hash != store_file.fs_hash {
            return Err(Error::CannotDecryptFileError);
        }

        let key = crypto::derive_key(password.as_str(), &salt)?;

        let fs = store_file.fs.as_slice();
        let fs = crypto::decrypt(fs, &key, &iv);
        let fs = fs.map_err(|_| Error::CannotDecryptFileError)?;
        let fs: Filesystem = deserialize(fs.as_slice())?;

        let store = Store {
            fs,
            key,
            path: store_folder.path,
            salt,
        };

        Ok(store)
    }

    /// Encrypts a file and adds it to the store.
    /// The arguments work like the `rsync` unix command when it comes to
    /// trailling slashes, so a trailling slash on the source, if it is a
    /// folder, makes the contents be copied into destination. Without the
    /// slash, the folder itself is copied.
    ///
    /// # Arguments
    ///
    /// * `file_path` - File path in the disk.
    /// * `store_path` - Path in store where to save.
    pub fn add(&mut self, file_path: &str, store_path: &str) -> Result<(), Error> {
        let source_contents = file_path.ends_with('/');

        let file_path: String = file_path.into();
        let store_path: String = store_path.into();

        let file_path = RealPath::new(&file_path).ok_or(Error::CannotParseError)?;
        let store_path = VirtualPath::new(&store_path).ok_or(Error::CannotParseError)?;
        let store_folder = RealPath::new(&self.path).ok_or(Error::CannotParseError)?;

        if file_path.is_dir() {
            let store_path = match self.fs.exists(&store_path.path)? {
                Some(id) => {
                    let node = self.fs.get(id)?;
                    if node.is_file {
                        return Err(Error::CannotCreateDirectoryError);
                    } else if source_contents || &store_path.path == "/" {
                        store_path
                    } else {
                        store_path
                            .join(&file_path.name)
                            .ok_or(Error::CannotParseError)?
                    }
                }
                None => store_path,
            };

            for entry in walkdir::WalkDir::new(&file_path.path)
                .follow_links(true)
                .into_iter()
                .filter_map(Result::ok)
            {
                let file_path = if source_contents {
                    &file_path.path
                } else {
                    &file_path.parent
                };

                let entry_path: RealPath = entry.path().to_path_buf().into();
                let store_path = entry_path
                    .reroot_as_virtual(file_path, &store_path.path)
                    .ok_or(Error::CannotParseError)?;

                if entry
                    .metadata()
                    .map_err(|_| Error::CannotReadFileError)?
                    .is_dir()
                {
                    self.fs.mkdirp(&store_path.path)?;
                } else {
                    self.add(&entry_path.path, &store_path.path)?;
                }
            }
        } else {
            let store_path = match self.fs.exists(&store_path.path)? {
                Some(id) => {
                    let node = self.fs.get(id)?;
                    if node.is_file {
                        return Err(Error::FileAlreadyExistsError);
                    } else {
                        let joined = store_path
                            .join(&file_path.name)
                            .ok_or(Error::CannotParseError)?;
                        if self.fs.exists(&joined.path)?.is_some() {
                            return Err(Error::FileAlreadyExistsError);
                        }
                        joined
                    }
                }
                None => store_path,
            };

            let file_handle = fs::File::open(&file_path.path);
            let mut file_handle = file_handle.map_err(|_| Error::CannotReadFileError)?;

            let file_std_path = std::path::Path::new(&file_path.path);
            let mimetype = tree_magic::from_filepath(file_std_path);

            let file_size = file_std_path
                .metadata()
                .map_err(|_| Error::CannotReadFileError)?
                .len();

            let node_id = self.fs.touch(&store_path.path)?;
            self.fs.set_size(node_id, file_size)?;
            self.fs.set_metadata(node_id, "mimetype", &mimetype)?;
            let mut bytes = vec![0u8; 52428800]; // 50MB

            loop {
                let bytes_read = match file_handle.read(bytes.as_mut_slice()) {
                    Ok(size) => size,
                    Err(_) => {
                        let data = self.fs.rm(node_id)?;
                        for d in data {
                            let part_name = hex::encode(d.id.to_be_bytes());
                            let part_name = format!("{part_name:0>32}");
                            let part_file = store_folder
                                .join(part_name)
                                .ok_or(Error::CannotParseError)?;
                            fs::remove_file(part_file.path).ok();
                        }
                        return Err(Error::CannotReadFileError);
                    }
                };

                if bytes_read == 0 {
                    break;
                }

                let iv = crypto::uuid();
                let key = crypto::random_key();

                let data = Data { id: 0, key, iv };

                let file = self.fs.append(node_id, &data)?;
                let data = file
                    .data
                    .iter()
                    .last()
                    .ok_or(Error::InternalStructureError)?;

                let bytes_read = &bytes[..bytes_read];
                let content = crypto::encrypt(bytes_read, &key, &iv)?;
                let part_name = hex::encode(data.id.to_be_bytes());
                let part_name = format!("{part_name:0>32}");
                let part_file = store_folder
                    .join(part_name)
                    .ok_or(Error::CannotParseError)?;

                if fs::write(part_file.path, content).is_err() {
                    let data = self.fs.rm(node_id)?;
                    for d in data {
                        let part_name = hex::encode(d.id.to_be_bytes());
                        let part_name = format!("{part_name:0>32}");
                        let part_file = store_folder
                            .join(part_name)
                            .ok_or(Error::CannotParseError)?;
                        fs::remove_file(part_file.path).ok();
                    }
                    return Err(Error::CannotWriteFileError);
                };
            }
        }

        self.save()?;
        Ok(())
    }

    /// Decrypts a file from the store and saves it on disk.
    ///
    /// # Arguments
    ///
    /// * `store_path` - Path of folder/file in the store.
    /// * `file_path` - Path in the disk where to save.
    pub fn get(&self, store_path: &str, file_path: &str) -> Result<(), Error> {
        let file_path: String = file_path.into();
        let store_path: String = store_path.into();

        let store_path = VirtualPath::new(&store_path).ok_or(Error::CannotParseError)?;
        let file_path = RealPath::new(&file_path).ok_or(Error::CannotParseError)?;

        if file_path.exists() {
            return Err(Error::FileAlreadyExistsError);
        }

        let id = self
            .fs
            .exists(&store_path.path)?
            .ok_or(Error::FileDoesNotExistError)?;
        let file = self.fs.get(id)?;

        if file.is_file {
            if !std::path::Path::new(&file_path.parent).exists() {
                fs::create_dir_all(&file_path.parent)
                    .map_err(|_| Error::CannotCreateDirectoryError)?;
            }

            let file_handle = fs::File::create(&file_path.path);
            let mut file_handle = file_handle.map_err(|_| Error::CannotWriteFileError)?;

            let store_disk = RealPath::new(&self.path).ok_or(Error::CannotParseError)?;
            for data in &file.data {
                let part_name = hex::encode(data.id.to_be_bytes());
                let part_name = format!("{part_name:0>32}");
                let part_path = store_disk.join(part_name).ok_or(Error::CannotParseError)?;
                let cipher = fs::read(part_path.path).map_err(|_| Error::CannotReadFileError)?;
                let content = crypto::decrypt(cipher.as_slice(), &data.key, &data.iv);
                let content = content.map_err(|_| Error::CannotDecryptFileError)?;

                file_handle
                    .write_all(content.as_slice())
                    .map_err(|_| Error::CannotWriteFileError)?;
            }
        } else {
            std::fs::create_dir_all(&file_path.path)
                .map_err(|_| Error::CannotCreateDirectoryError)?;
            let children = self.fs.ls(id)?;
            for child in children {
                let from = store_path
                    .join(&child.name)
                    .ok_or(Error::CannotParseError)?;
                let to = file_path.join(&child.name).ok_or(Error::CannotParseError)?;
                self.get(&from.path, &to.path)?;
            }
        }

        Ok(())
    }

    /// Removes a file or folder from the store.
    ///
    /// # Arguments
    ///
    /// * `path` - Path of folder/file in the store.
    pub fn remove(&mut self, path: &str) -> Result<(), Error> {
        let path: String = path.into();

        let path = VirtualPath::new(&path).ok_or(Error::CannotParseError)?;
        let store_folder = RealPath::new(&self.path).ok_or(Error::CannotParseError)?;

        let id = self
            .fs
            .exists(&path.path)?
            .ok_or(Error::FileDoesNotExistError)?;
        let data = self.fs.rm(id)?;

        for d in data {
            let part_name = hex::encode(d.id.to_be_bytes());
            let part_name = format!("{part_name:0>32}");
            let part_path = store_folder
                .join(part_name)
                .ok_or(Error::CannotParseError)?;
            fs::remove_file(part_path.path).ok();
        }

        self.save()
    }

    // Moves a file or folder.
    //
    // # Arguments
    //
    // * `src` - Source path.
    // * `dst` - Destination path.
    pub fn mv(&mut self, src: &str, dst: &str) -> Result<(), Error> {
        let src: String = src.into();
        let dst: String = dst.into();
        let src = VirtualPath::new(&src).ok_or(Error::CannotParseError)?;
        let dst = VirtualPath::new(&dst).ok_or(Error::CannotParseError)?;

        let src_id = self
            .fs
            .exists(&src.path)?
            .ok_or(Error::FileDoesNotExistError)?;
        let dst_id = self.fs.mkdirp(&dst.parent)?;

        self.fs.mv(src_id, dst_id)
    }

    /// Lists files in the store.
    ///
    /// # Arguments
    ///
    /// * `path` - Path of folder/file in the store.
    ///
    /// # Returns
    ///
    /// * A list of File objects with this folder's direct children.
    pub fn list(&self, path: &str) -> Result<Vec<File>, Error> {
        if path == "*" {
            return self.fs.ls_all();
        }

        let path: String = path.into();
        let path = VirtualPath::new(&path).ok_or(Error::CannotParseError)?;

        if path.path != "/" {
            let id = self
                .fs
                .exists(&path.path)?
                .ok_or(Error::FolderDoesNotExistError)?;
            let file = self.fs.get(id)?;
            if file.is_file {
                Ok(vec![file])
            } else {
                self.fs.ls(id)
            }
        } else {
            self.fs.ls(0)
        }
    }

    /// Truncates a file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path of the file to be truncated.
    pub fn truncate(&mut self, path: &str) -> Result<(), Error> {
        let path: String = path.into();
        let path = VirtualPath::new(&path).ok_or(Error::CannotParseError)?;

        let id = self
            .fs
            .exists(&path.path)?
            .ok_or(Error::FileDoesNotExistError)?;
        self.fs.truncate(id)?;

        self.save()
    }

    /// Sets file/folder metadata
    ///
    /// # Arguments
    ///
    /// * `id` - id of the affected node;
    /// * `key` - metadata key;
    /// * `value` - metadata value;
    pub fn metadata_set(&mut self, path: &str, key: &str, value: &str) -> Result<(), Error> {
        let path: String = path.into();
        let key: String = key.into();
        let value: String = value.into();

        let path = VirtualPath::new(&path).ok_or(Error::CannotParseError)?;

        let id = self
            .fs
            .exists(&path.path)?
            .ok_or(Error::FileDoesNotExistError)?;
        self.fs.set_metadata(id, &key, &value)?;

        self.save()
    }

    /// Removes a key from the node's metadata
    ///
    /// # Arguments
    ///
    /// * `id` - id of the affected node;
    /// * `key` - metadata key;
    pub fn metadata_remove(&mut self, path: &str, key: &str) -> Result<(), Error> {
        let path: String = path.into();
        let key: String = key.into();

        let path = VirtualPath::new(&path).ok_or(Error::CannotParseError)?;

        let id = self
            .fs
            .exists(&path.path)?
            .ok_or(Error::FileDoesNotExistError)?;
        self.fs.rm_metadata(id, &key)?;

        self.save()
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
    pub fn metadata_get(&self, path: &str, key: &str) -> Result<String, Error> {
        let path: String = path.into();
        let key: String = key.into();

        let path = VirtualPath::new(&path).ok_or(Error::CannotParseError)?;

        let id = self
            .fs
            .exists(&path.path)?
            .ok_or(Error::FileDoesNotExistError)?;
        self.fs.get_metadata(id, &key)
    }

    /// Returns file/folder metadata
    ///
    /// # Arguments
    ///
    /// * `id` - id of the affected node;
    ///
    /// # Returns
    ///
    /// * The metadata HashMap
    pub fn metadata_list(&self, path: &str) -> Result<HashMap<String, String>, Error> {
        let path: String = path.into();
        let path = VirtualPath::new(&path).ok_or(Error::CannotParseError)?;

        let id = self
            .fs
            .exists(&path.path)?
            .ok_or(Error::FileDoesNotExistError)?;
        let file = self.fs.get(id)?;

        Ok(file.metadata)
    }

    /// Adds a tag to a file
    ///
    /// # Arguments
    ///
    /// * `id` - Id of the file to add the tag to.
    /// * `tag` - Name of the tag to add.
    pub fn tag_add(&mut self, path: &str, tag: &str) -> Result<(), Error> {
        let path: String = path.into();
        let path = VirtualPath::new(&path).ok_or(Error::CannotParseError)?;

        let id = self
            .fs
            .exists(&path.path)?
            .ok_or(Error::FileDoesNotExistError)?;
        self.fs.add_tag(id, tag)?;

        self.save()
    }

    /// Removes a tag from a node.
    ///
    /// # Arguments
    ///
    /// * `id` - Node's id.
    /// * `tag` - Tag to remove.
    pub fn tag_rm(&mut self, path: &str, tag: &str) -> Result<(), Error> {
        let path: String = path.into();
        let path = VirtualPath::new(&path).ok_or(Error::CannotParseError)?;

        let id = self
            .fs
            .exists(&path.path)?
            .ok_or(Error::FileDoesNotExistError)?;
        self.fs.rm_tag(id, tag)?;

        self.save()
    }

    /// Clears all tags from a node.
    ///
    /// # Arguments
    ///
    /// * `id` - Node's id.
    pub fn tag_clear(&mut self, path: &str) -> Result<(), Error> {
        let path: String = path.into();
        let path = VirtualPath::new(&path).ok_or(Error::CannotParseError)?;

        let id = self
            .fs
            .exists(&path.path)?
            .ok_or(Error::FileDoesNotExistError)?;
        self.fs.clear_tag(id)?;

        self.save()
    }

    /// List all tags in the filesystem.
    ///
    /// # Returns
    ///
    /// * A list of all tags found in the filesystem.
    pub fn tag_list(&self) -> Vec<String> {
        self.fs.list_tag()
    }

    /// List all tags in a node.
    ///
    /// # Arguments
    ///
    /// * `id` - Node's id.
    ///
    /// # Returns
    ///
    /// * A list of all tags found in the filesystem.
    pub fn tag_get(&self, path: &str) -> Result<Vec<String>, Error> {
        let path: String = path.into();
        let path = VirtualPath::new(&path).ok_or(Error::CannotParseError)?;

        let id = self
            .fs
            .exists(&path.path)?
            .ok_or(Error::FileDoesNotExistError)?;
        let node = self.fs.get(id)?;

        Ok(node.tags)
    }

    /// Lists files that does or does not contain a certaing tag. Accepts a list
    /// of tags returns a list of File objects for all nodes matching. The name
    /// of the files are their paths.
    ///
    /// # Arguments
    ///
    /// * `tags` - List of tags to search for. If the tag starts with !, search for files not
    ///   containing that tag.
    ///
    /// # Returns
    ///
    /// * A list of files matching the given tags.
    pub fn tag_search(&self, tags: Vec<String>) -> Vec<File> {
        self.fs.search_tag(tags)
    }

    /// Removes orphaned chunk files from the store directory.
    ///
    /// Orphaned chunks are encrypted data files that exist on disk but are no
    /// longer referenced by the store index. This can happen when a `save()`
    /// fails after chunks have already been written (e.g. disk full).
    ///
    /// # Returns
    ///
    /// * The number of orphaned files removed.
    pub fn gc(&self) -> Result<usize, Error> {
        let store_folder = RealPath::new(&self.path).ok_or(Error::CannotParseError)?;

        let referenced: std::collections::HashSet<String> = self
            .fs
            .data_ids()
            .iter()
            .map(|&id| format!("{:0>32}", hex::encode(id.to_be_bytes())))
            .collect();

        let mut removed = 0;
        let entries = fs::read_dir(&store_folder.path).map_err(|_| Error::CannotReadFileError)?;

        for entry in entries.filter_map(Result::ok) {
            let name = entry.file_name().to_string_lossy().to_string();
            if name == "Store.void" {
                continue;
            }
            if !referenced.contains(&name) && fs::remove_file(entry.path()).is_ok() {
                removed += 1;
            }
        }

        Ok(removed)
    }
}
