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
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for Error {}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct File {
    name: String,
    path: String,
    size: u64,
    key: [u8; 32],
    iv: [u8; 16],
    salt: [u8; 16],
    metadata: HashMap<String, String>,
    parts: Vec<String>,
}

trait FlexBufferSerializable {
    fn fb_serialize(&self) -> Result<Vec<u8>, Error>;
    fn fb_deserialize(bytes: &[u8]) -> Result<Box<Self>, Error>;
}

#[derive(Serialize, Deserialize)]
struct StoreFile {
    contents: Vec<u8>,
    hash: [u8; 32],
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

impl FlexBufferSerializable for Vec<File> {
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
    files: Box<Vec<File>>,
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
    fn save(&self) -> Result<(), Error> {
        let store_folder = Path::new(&self.path).ok_or(Error::CannotParseError)?;
        let store_journal = store_folder
            .join("Store.void")
            .ok_or(Error::CannotParseError)?;

        if !store_journal.exists() {
            return Err(Error::FileDoesNotExistError);
        }

        let file_bytes = match self.files.fb_serialize() {
            Ok(bytes) => bytes,
            Err(err) => return Err(err),
        };

        let key = &self.key;
        let iv = &self.iv;
        let contents = crypto::encrypt(file_bytes.as_slice(), key, iv);
        let contents_hash = crypto::hash(contents.as_slice(), &self.salt);
        let mut hash = [0u8; 32];

        hash.copy_from_slice(&contents_hash);

        let store_file = StoreFile {
            contents,
            hash,
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

        let store = Store {
            files: Box::new(vec![]),
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

        let contents = store_file.contents.as_slice();
        let contents =
            crypto::decrypt(contents, &key, &iv).map_err(|_| Error::CannotDecryptFileError)?;

        let files = Vec::fb_deserialize(contents.as_slice())?;

        let store = Store {
            files,
            iv: store_file.iv,
            key,
            path: store_folder.path,
            salt: store_file.salt,
        };

        Ok(store)
    }

    pub fn add<S: Into<String>>(&mut self, file_path: S, store_path: S) -> Result<(), Error> {
        let file_path: String = file_path.into();
        let store_path: String = store_path.into();

        let ends_in_slash = store_path.ends_with("/");

        let store_path = Path::new(&store_path).ok_or(Error::CannotParseError)?;

        let store_folder = Path::new(&self.path).ok_or(Error::CannotParseError)?;
        let store_journal = store_folder
            .join("Store.void")
            .ok_or(Error::CannotParseError)?;
        let file_path = Path::new(file_path).ok_or(Error::CannotParseError)?;

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
            let mut file_handle =
                fs::File::open(&file_path.path).map_err(|_| Error::CannotReadFileError)?;

            let salt = crypto::uuid();
            let iv = crypto::uuid();
            let pswd = hex::encode(crypto::uuid());
            let key = crypto::derive_key(&pswd, &salt, &iv);

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
                if self
                    .files
                    .iter()
                    .any(|f| f.name == file_name && f.path == store_path.path)
                {
                    return Err(Error::StoreFileAlreadyExistsError);
                }

                // Path contains a file?
                // like /folder/file.txt/folder/file.jpg
                if self
                    .files
                    .iter()
                    .map(|f| Path::new(&f.path)?.join(&f.name))
                    .filter_map(|p| p)
                    .any(|p| store_path.contains(&p))
                {
                    return Err(Error::StoreFileAlreadyExistsError);
                }

                (store_path, file_name)
            } else {
                // is /folder/folder a folder?
                if self
                    .files
                    .iter()
                    .map(|f| Path::new(&f.path))
                    .filter_map(|p| p)
                    .any(|p| p.contains(&store_path))
                {
                    // File already exists?
                    if self
                        .files
                        .iter()
                        .any(|f| f.name == file_name && f.path == store_path.path)
                    {
                        return Err(Error::StoreFileAlreadyExistsError);
                    }

                    (store_path, file_name)
                } else {
                    let file_name = store_path.name;
                    let store_path = Path::new(store_path.parent).ok_or(Error::CannotParseError)?;

                    (store_path, file_name)
                }
            };

            let mut file = File {
                name: file_name.into(),
                path: store_path.path,
                size: file_size,
                key,
                iv,
                salt,
                metadata,
                parts: vec![],
            };

            let mut bytes = vec![0u8; 52428800];
            loop {
                let bytes_read = match file_handle.read(bytes.as_mut_slice()) {
                    Ok(size) => size,
                    Err(_) => {
                        for part in file.parts {
                            let part_file =
                                store_folder.join(part).ok_or(Error::CannotParseError)?;
                            fs::remove_file(part_file.path).unwrap_or(());
                        }
                        return Err(Error::CannotReadFileError);
                    }
                };

                if bytes_read == 0 {
                    break;
                }

                let bytes_read = &bytes[..bytes_read];
                let file_path = file.path.clone() + "/" + file.name.as_str();
                let file_path = file_path.as_bytes();
                let part_hash = crypto::hash2(bytes_read, file_path, &file.salt);
                let part_hash = hex::encode(part_hash);
                let content = crypto::encrypt(bytes_read, &file.key, &file.iv);
                let part_file = store_folder
                    .join(&part_hash)
                    .ok_or(Error::CannotParseError)?;

                if let Err(_) = fs::write(part_file.path, content) {
                    for part in file.parts {
                        let part_file = store_folder.join(part).ok_or(Error::CannotParseError)?;
                        fs::remove_file(part_file.path).unwrap_or(());
                    }
                    return Err(Error::CannotWriteFileError);
                };

                file.parts.push(part_hash);
            }

            self.files.push(file.clone());
            if let Err(err) = self.save() {
                for part in file.parts {
                    let part_file = store_folder.join(part).ok_or(Error::CannotParseError)?;
                    fs::remove_file(part_file.path).unwrap_or(());
                }
                return Err(err);
            }
        }

        Ok(())
    }

    pub fn get<S: Into<String>>(&self, store_path: S, file_path: S) -> Result<(), Error> {
        let store_path = Path::new(store_path).ok_or(Error::CannotParseError)?;
        let file_path = Path::new(file_path).ok_or(Error::CannotParseError)?;

        if file_path.exists() {
            return Err(Error::FileAlreadyExistsError);
        }

        let files: Vec<&File> = self
            .files
            .iter()
            .filter_map(|f| {
                let file_path = Path::new(&f.path)?.join(&f.name)?;
                if file_path.contains(&store_path) {
                    Some(f)
                } else {
                    None
                }
            })
            .collect();

        for file in &files {
            let disk_path = Path::new(&file.path)
                .ok_or(Error::CannotParseError)?
                .join(&file.name)
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

            let mut file_handle =
                fs::File::create(&disk_path.path).map_err(|_| Error::CannotWriteFileError)?;

            let store_path = Path::new(&self.path).ok_or(Error::CannotParseError)?;
            for part in &file.parts {
                let part_path = store_path.join(part).ok_or(Error::CannotParseError)?;
                let cipher = fs::read(part_path.path).map_err(|_| Error::CannotReadFileError)?;
                let content = crypto::decrypt(cipher.as_slice(), &file.key, &file.iv)
                    .map_err(|_| Error::CannotDecryptFileError)?;

                file_handle
                    .write_all(content.as_slice())
                    .map_err(|_| Error::CannotWriteFileError)?;
            }
        }

        Ok(())
    }

    pub fn remove<S: Into<String>>(&mut self, path: S) -> Result<(), Error> {
        let path: String = path.into();
        let path = Path::new(&path).ok_or(Error::CannotParseError)?;

        let files: Vec<String> = self
            .files
            .iter()
            .filter_map(|f| {
                let file_path = Path::new(&f.path)?.join(&f.name)?;
                if file_path.contains(&path) {
                    Some(f)
                } else {
                    None
                }
            })
            .flat_map(|f| f.parts.clone())
            .collect();

        let mut cannot_remove: Vec<String> = vec![];
        for file in files {
            // It should not panic. Store has been tested and is valid and file
            // is just a filename.
            let file_path = Path::new(&self.path).unwrap().join(&file).unwrap();

            if let Err(_) = fs::remove_file(file_path.path) {
                cannot_remove.push(file.clone());
            }
        }

        let files: Vec<File> = self
            .files
            .iter()
            .filter_map(|f| {
                let file_path = Path::new(&f.path)?.join(&f.name)?;
                if !file_path.contains(&path) {
                    Some(f.clone())
                } else {
                    None
                }
            })
            .collect();

        self.files = Box::new(files);
        self.save()?;

        if !cannot_remove.is_empty() {
            return Err(Error::CannotRemoveFilesError(cannot_remove));
        }

        Ok(())
    }

    pub fn list<S: Into<String>>(&self, path: S) -> Result<Vec<(String, u64)>, Error> {
        let path: String = path.into();
        let path = Path::new(&path).ok_or(Error::CannotParseError)?;

        let files = self
            .files
            .iter()
            .filter_map(|f| {
                let file_path = Path::new(&f.path)?.join(&f.name)?;
                if file_path.contains(&path) {
                    Some((file_path.path, f.size))
                } else {
                    None
                }
            })
            .collect();

        Ok(files)
    }
}
