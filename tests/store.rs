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

use std::fs;
use std::path::Path;
use void::{Error, Store};

#[test]
fn test_create() -> Result<(), Error> {
    if Path::new("test_create").exists() {
        fs::remove_dir_all("test_create").unwrap();
    }

    Store::create("test_create", "1234")?;

    assert!(Path::new("test_create/Store.void").exists());

    match Store::create("test_create", "1234") {
        Ok(_) => panic!("Store created when it should not have been possible."),
        Err(Error::FileAlreadyExistsError) => (),
        Err(err) => panic!("Wrong error!: {:?}", err),
    }

    fs::remove_dir_all("test_create").unwrap();

    Ok(())
}

#[test]
fn test_open() -> Result<(), Error> {
    Store::create("test_open", "1234")?;
    Store::open("test_open", "1234")?;

    fs::remove_dir_all("test_open").unwrap();

    match Store::open("test_open_not", "1234") {
        Ok(_) => panic!("Should not have opened."),
        Err(Error::FolderDoesNotExistError) => (),
        Err(err) => panic!("Wrong error: {:?}", err),
    }

    Ok(())
}

#[test]
fn test_add_file() -> Result<(), Error> {
    if Path::new("test_add").exists() {
        fs::remove_dir_all("test_add").unwrap();
    }

    let mut store = Store::create("test_add", "1234")?;
    store.add("Cargo.toml", "/")?;
    store.add("Cargo.toml", "/src/cargo")?;
    store.add("Cargo.toml", "/src")?;
    store.add("Cargo.toml", "/somewhere/")?;

    match store.add("Cargo.toml", "/src") {
        Ok(_) => panic!("Should not have inserted"),
        Err(Error::StoreFileAlreadyExistsError) => {}
        Err(err) => panic!("Wrong error: {:?}", err),
    }

    fs::remove_dir_all("test_add").unwrap();

    Ok(())
}

#[test]
fn test_add_folder() -> Result<(), Error> {
    if Path::new("test_add_folder").exists() {
        fs::remove_dir_all("test_add_folder").unwrap();
    }

    let mut store = Store::create("test_add_folder", "1234")?;
    store.add("src", "/")?;

    fs::remove_dir_all("test_add_folder").unwrap();

    Ok(())
}

// #[test]
// fn test_add_big_file() -> Result<(), Error> {
//     if Path::new("test_add_big").exists() {
//         fs::remove_dir_all("test_add_big").unwrap();
//     }

//     if Path::new("test_big_file").exists() {
//         fs::remove_file("test_big_file").unwrap();
//     }

//     let mut store = Store::create("test_add_big", "1234")?;

//     let big_chunk: Vec<u8> = (0..157286912).map(|_| rand::random::<u8>()).collect();
//     match fs::write("test_big_file", big_chunk) {
//         Ok(_) => (),
//         Err(err) => panic!("Error creating big file: {:?}", err),
//     }

//     store.add("test_big_file", "/")?;

//     let files = fs::read_dir("test_add_big")
//         .unwrap()
//         .map(|res| res.map(|e| e.path()))
//         .collect::<Result<Vec<_>, std::io::Error>>()
//         .unwrap();
//     assert_eq!(5, files.len());

//     fs::remove_file("test_big_file").unwrap();
//     fs::remove_dir_all("test_add_big").unwrap();

//     Ok(())
// }

#[test]
fn test_get_file() -> Result<(), Error> {
    if Path::new("test_get_file").exists() {
        fs::remove_file("test_get_file").unwrap();
    }

    if Path::new("test_get_dir").exists() {
        fs::remove_file("test_get_dir").unwrap();
    }

    if Path::new("test_get_file.toml").exists() {
        fs::remove_file("test_get_file.toml").unwrap();
    }

    let mut store = Store::create("test_get_file", "1234")?;
    store.add("Cargo.toml", "/dir/")?;
    store.add("src/gui/main.rs", "/dir")?;
    store.add("src/lib/crypto.rs", "/dir")?;

    store.get("/dir/Cargo.toml", "test_get_file.toml")?;
    store.get("/dir", "test_get_dir")?;

    fs::remove_dir_all("test_get_file").unwrap();
    fs::remove_dir_all("test_get_dir").unwrap();
    fs::remove_file("test_get_file.toml").unwrap();

    Ok(())
}

#[test]
fn test_remove_file() -> Result<(), Error> {
    if Path::new("test_remove_file").exists() {
        fs::remove_dir_all("test_remove_file").unwrap();
    }

    let mut store = Store::create("test_remove_file", "1234")?;
    store.add("Cargo.toml", "/dir/")?;
    store.remove("/dir/Cargo.toml")?;

    let files = fs::read_dir("test_remove_file")
        .unwrap()
        .map(|res| res.map(|e| e.path()))
        .collect::<Result<Vec<_>, std::io::Error>>()
        .unwrap();
    assert_eq!(1, files.len());

    fs::remove_dir_all("test_remove_file").unwrap();

    Ok(())
}
