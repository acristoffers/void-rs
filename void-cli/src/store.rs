/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use prettytable::{cell, row, Cell, Row, Table};
use std::cmp::Ordering;
use std::collections::HashMap;
use void::{Error::*, Store};

fn open_store(path: String, password: String) -> Option<Store> {
    Store::open(path, password)
        .map_err(|error| {
            let msg = match &error {
                FolderDoesNotExistError => "The specified Store does not exist.".into(),
                FileDoesNotExistError => "The specified Store does not exist.".into(),
                CannotReadFileError => "Cannot read the store file.".into(),
                CannotDeserializeError => {
                    "Could not deserialize the store file. Is it corrupt? Or wrong password?".into()
                }
                err => format!("Unexpected error ocurred: {err:?}"),
            };
            eprint!("{msg}");
            error
        })
        .ok()
}

pub fn create_store(path: String, password: String) -> Option<()> {
    Store::create(path, password)
        .map_err(|error| {
            let msg = match &error {
                CannotCreateDirectoryError => "Could not create folder.".into(),
                CannotSerializeError => "Could not serialize store.".into(),
                CannotWriteFileError => "Could not write to store file.".into(),
                CannotCreateFileError => "Could not create store file.".into(),
                FileAlreadyExistsError => "Store already exists.".into(),
                err => format!("Unknown error occurred: {err:?}"),
            };
            eprint!("{msg}");
            error
        })
        .ok()?;

    println!("Store created.");
    Some(())
}

pub fn add(
    store_path: String,
    internal_path: String,
    files: Vec<String>,
    password: String,
) -> Option<()> {
    let mut store = open_store(store_path, password)?;

    for file in files {
        println!("Adding {} into {}", file, &internal_path);
        store
            .add(&file, &internal_path)
            .map_err(|error| {
                let msg = match &error {
                    CannotReadFileError => format!("Cannot read file {file}."),
                    CannotWriteFileError => format!("Cannot write file {file} into store."),
                    CannotCreateFileError => format!("Cannot write file {file} into store."),
                    FileDoesNotExistError => format!("File {file} does not exist."),
                    CannotSerializeError => "Error saving: could not serialize.".into(),
                    FileAlreadyExistsError => "Hash collision ocurred?".into(),
                    StoreFileAlreadyExistsError => {
                        "A file with same name in same path already exists.".into()
                    }
                    err => format!("An error occurred: {err:?}"),
                };
                eprint!("{msg}");
                error
            })
            .ok()?;
    }

    Some(())
}

pub fn get(
    store_path: String,
    internal_path: String,
    external_path: String,
    password: String,
) -> Option<()> {
    open_store(store_path, password)?
        .get(&internal_path, &external_path)
        .map_err(|error| {
            let msg = match &error {
                CannotWriteFileError => format!("Cannot write file {external_path}."),
                CannotCreateFileError => format!("Cannot write file {external_path}."),
                FileAlreadyExistsError => format!("File {external_path} already exists"),
                err => format!("An error occurred: {err:?}"),
            };
            eprint!("{msg}");
            error
        })
        .ok()
}

pub fn remove(store_path: String, path: String, password: String) -> Option<()> {
    open_store(store_path, password)?
        .remove(&path)
        .map_err(|error| {
            let err = &error;
            let msg = format!("An error occurred: {err:?}");
            eprint!("{msg}");
            error
        })
        .ok()
}

pub fn list(
    store_path: String,
    path: String,
    password: String,
    human: bool,
    list: bool,
) -> Option<()> {
    let mut store = open_store(store_path, password)?;

    let mut files = store
        .list(&path)
        .map_err(|error| {
            let err = &error;
            let msg = format!("An error occurred: {err:?}");
            eprint!("{msg}");
            error
        })
        .ok()?;

    files.sort_by(|a, b| {
        if !a.is_file && b.is_file {
            Ordering::Less
        } else if a.is_file && !b.is_file {
            Ordering::Greater
        } else {
            a.name.cmp(&b.name)
        }
    });

    let files: Vec<(String, String)> = files
        .iter()
        .map(|file| {
            let name: String = if !file.is_file {
                file.name.clone() + "/"
            } else {
                file.name.clone()
            };
            let size: String = if human {
                bytesize::ByteSize(file.size).to_string()
            } else {
                file.size.to_string()
            };
            (name, size)
        })
        .collect();

    if files.is_empty() {
        return Some(());
    }

    let mut table = Table::new();
    table.set_format(*prettytable::format::consts::FORMAT_CLEAN);

    if (human || list) && path != "*" {
        for (name, size) in files {
            table.add_row(row![name, size]);
        }
    } else {
        let cells: Vec<Cell> = files.iter().map(|file| cell![file.0]).collect();
        let max_width = files.iter().map(|file| file.0.len()).max()?;
        let term_width = term_size::dimensions()?.0;
        let cells_per_row: usize = if term_width >= max_width {
            term_width / max_width
        } else {
            1
        };
        for cells in cells.chunks(cells_per_row) {
            let row = Row::new(cells.to_vec());
            table.add_row(row);
        }
    }

    table.printstd();

    Some(())
}

pub fn metadata_set(
    store_path: String,
    path: String,
    password: String,
    key: String,
    value: String,
) -> Option<()> {
    let mut store = open_store(store_path, password)?;

    store
        .metadata_set(&path, &key, &value)
        .map_err(|error| {
            let msg = match &error {
                CannotSerializeError => "Error saving: could not serialize.".into(),
                FileAlreadyExistsError => "Hash collision ocurred?".into(),
                StoreFileAlreadyExistsError => {
                    "A file with same name in same path already exists.".into()
                }
                err => format!("An error occurred: {err:?}"),
            };
            eprint!("{msg}");
            error
        })
        .ok()?;

    Some(())
}

pub fn metadata_get(store_path: String, path: String, password: String, key: String) -> Option<()> {
    let mut store = open_store(store_path, password)?;

    let value = store
        .metadata_get(&path, &key)
        .map_err(|error| {
            let msg = match &error {
                CannotSerializeError => "Error saving: could not serialize.".into(),
                FileAlreadyExistsError => "Hash collision ocurred?".into(),
                StoreFileAlreadyExistsError => {
                    "A file with same name in same path already exists.".into()
                }
                err => format!("An error occurred: {err:?}"),
            };
            eprint!("{msg}");
            error
        })
        .ok()?;

    println!("{key}: {value}");

    Some(())
}

pub fn metadata_list(store_path: String, path: String, password: String) -> Option<()> {
    let mut store = open_store(store_path, password)?;

    let map: HashMap<String, String> = store
        .metadata_list(&path)
        .map_err(|error| {
            let msg = match &error {
                CannotSerializeError => "Error saving: could not serialize.".into(),
                FileAlreadyExistsError => "Hash collision ocurred?".into(),
                StoreFileAlreadyExistsError => {
                    "A file with same name in same path already exists.".into()
                }
                err => format!("An error occurred: {err:?}"),
            };
            eprint!("{msg}");
            error
        })
        .ok()?;

    let mut table = Table::new();
    table.set_format(*prettytable::format::consts::FORMAT_CLEAN);

    let mut list: Vec<(&String, &String)> = map.iter().collect();
    list.sort_by_key(|(key, _)| *key);
    for (key, value) in list {
        table.add_row(row![key, value]);
    }

    table.printstd();

    Some(())
}

pub fn metadata_remove(
    store_path: String,
    path: String,
    password: String,
    key: String,
) -> Option<()> {
    let mut store = open_store(store_path, password)?;

    store
        .metadata_remove(&path, &key)
        .map_err(|error| {
            let msg = match &error {
                CannotSerializeError => "Error saving: could not serialize.".into(),
                FileAlreadyExistsError => "Hash collision ocurred?".into(),
                StoreFileAlreadyExistsError => {
                    "A file with same name in same path already exists.".into()
                }
                err => format!("An error occurred: {err:?}"),
            };
            eprint!("{msg}");
            error
        })
        .ok()?;

    Some(())
}

pub fn tag_add(store_path: String, path: String, password: String, tag: String) -> Option<()> {
    let mut store = open_store(store_path, password)?;

    store
        .tag_add(&path, &tag)
        .map_err(|error| {
            let msg = match &error {
                CannotSerializeError => "Error saving: could not serialize.".into(),
                FileAlreadyExistsError => "Hash collision ocurred?".into(),
                StoreFileAlreadyExistsError => {
                    "A file with same name in same path already exists.".into()
                }
                err => format!("An error occurred: {err:?}"),
            };
            eprint!("{msg}");
            error
        })
        .ok()?;

    Some(())
}

pub fn tag_remove(store_path: String, path: String, password: String, tag: String) -> Option<()> {
    let mut store = open_store(store_path, password)?;

    store
        .tag_rm(&path, &tag)
        .map_err(|error| {
            let msg = match &error {
                CannotSerializeError => "Error saving: could not serialize.".into(),
                FileAlreadyExistsError => "Hash collision ocurred?".into(),
                StoreFileAlreadyExistsError => {
                    "A file with same name in same path already exists.".into()
                }
                err => format!("An error occurred: {err:?}"),
            };
            eprint!("{msg}");
            error
        })
        .ok()?;

    Some(())
}

pub fn tag_clear(store_path: String, path: String, password: String) -> Option<()> {
    let mut store = open_store(store_path, password)?;

    store
        .tag_clear(&path)
        .map_err(|error| {
            let msg = match &error {
                CannotSerializeError => "Error saving: could not serialize.".into(),
                FileAlreadyExistsError => "Hash collision ocurred?".into(),
                StoreFileAlreadyExistsError => {
                    "A file with same name in same path already exists.".into()
                }
                err => format!("An error occurred: {err:?}"),
            };
            eprint!("{msg}");
            error
        })
        .ok()?;

    Some(())
}

pub fn tag_list(store_path: String, password: String) -> Option<()> {
    let store = open_store(store_path, password)?;

    let mut tags = store.tag_list();
    tags.sort();

    let mut table = Table::new();
    table.set_format(*prettytable::format::consts::FORMAT_CLEAN);

    for tag in tags {
        table.add_row(row![tag]);
    }

    table.printstd();

    Some(())
}

pub fn tag_get(store_path: String, path: String, password: String) -> Option<()> {
    let mut store = open_store(store_path, password)?;

    let mut tags = store
        .tag_get(&path)
        .map_err(|error| {
            let msg = match &error {
                CannotSerializeError => "Error saving: could not serialize.".into(),
                FileAlreadyExistsError => "Hash collision ocurred?".into(),
                StoreFileAlreadyExistsError => {
                    "A file with same name in same path already exists.".into()
                }
                err => format!("An error occurred: {err:?}"),
            };
            eprint!("{msg}");
            error
        })
        .ok()?;
    tags.sort();

    let mut table = Table::new();
    table.set_format(*prettytable::format::consts::FORMAT_CLEAN);

    for tag in tags {
        table.add_row(row![tag]);
    }

    table.printstd();

    Some(())
}

pub fn tag_search(store_path: String, tags: Vec<String>, password: String) -> Option<()> {
    let store = open_store(store_path, password)?;

    let files = store.tag_search(tags);

    let mut table = Table::new();
    table.set_format(*prettytable::format::consts::FORMAT_CLEAN);

    for file in files {
        table.add_row(row![file.name]);
    }

    table.printstd();

    Some(())
}
