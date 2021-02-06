/*
 * The MIT License (MIT)
 *
 * Copyright (c) 2020 Álan Crístoffer e Sousa
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
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

use prettytable::{cell, row, Cell, Row, Table};
use std::cmp::Ordering;
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
                err => format!("Unexpected error ocurred: {:?}", err),
            };
            eprint!("{}", msg);
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
                err => format!("Unknown error occurred: {:?}", err),
            };
            eprint!("{}", msg);
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
                    CannotReadFileError => format!("Cannot read file {}.", file),
                    CannotWriteFileError => format!("Cannot write file {} into store.", file),
                    CannotCreateFileError => format!("Cannot write file {} into store.", file),
                    FileDoesNotExistError => format!("File {} does not exist.", file),
                    CannotSerializeError => "Error saving: could not serialize.".into(),
                    FileAlreadyExistsError => "Hash collision ocurred?".into(),
                    StoreFileAlreadyExistsError => {
                        "A file with same name in same path already exists.".into()
                    }
                    err => format!("An error occurred: {:?}", err),
                };
                eprint!("{}", msg);
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
                CannotWriteFileError => format!("Cannot write file {}.", external_path),
                CannotCreateFileError => format!("Cannot write file {}.", external_path),
                FileAlreadyExistsError => format!("File {} already exists", external_path),
                err => format!("An error occurred: {:?}", err),
            };
            eprint!("{}", msg);
            error
        })
        .ok()
}

pub fn remove(store_path: String, path: String, password: String) -> Option<()> {
    open_store(store_path, password)?
        .remove(path)
        .map_err(|error| {
            let msg = match &error {
                err => format!("An error occurred: {:?}", err),
            };
            eprint!("{}", msg);
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
    let store = open_store(store_path, password)?;

    let mut files = store
        .list(path)
        .map_err(|error| {
            let msg = match &error {
                err => format!("An error occurred: {:?}", err),
            };
            eprint!("{}", msg);
            error
        })
        .ok()?;

    files.sort_unstable_by(|a, b| {
        if a.2 && !b.2 {
            Ordering::Less
        } else if !a.2 && b.2 {
            Ordering::Greater
        } else {
            a.0.cmp(&b.0)
        }
    });

    let files: Vec<(String, String)> = files
        .iter()
        .map(|file| {
            let name: String = if file.2 {
                file.0.clone() + "/"
            } else {
                file.0.clone()
            };
            let size: String = if human {
                bytesize::ByteSize(file.1).to_string()
            } else {
                file.1.to_string()
            };

            (name, size)
        })
        .collect();

    if files.is_empty() {
        return Some(());
    }

    let mut table = Table::new();
    table.set_format(*prettytable::format::consts::FORMAT_CLEAN);

    if human || list {
        for (name, size) in files {
            table.add_row(row![name, size]);
        }
    } else {
        let cells: Vec<Cell> = files.iter().map(|file| cell![file.0]).collect();
        let max_width = files.iter().map(|file| file.0.len()).max()?;
        let term_width = term_size::dimensions()?.0;
        let cells_per_row: usize = term_width / max_width;

        for cells in cells.chunks(cells_per_row) {
            let row = Row::new(cells.to_vec());
            table.add_row(row);
        }
    }

    table.printstd();

    Some(())
}
