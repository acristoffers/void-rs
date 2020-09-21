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

fn gen_file(path: &str, size: usize) {
    let big_chunk: Vec<u8> = (0..size).map(|_| rand::random::<u8>()).collect();
    if let Err(err) = fs::write(path, big_chunk) {
        panic!("Error creating big file: {:?}", err);
    }
}

fn dir_ls_count(dir: &str) -> usize {
    fs::read_dir(dir)
        .unwrap()
        .map(|res| res.map(|e| e.path()))
        .collect::<Result<Vec<_>, std::io::Error>>()
        .unwrap()
        .len()
}

fn compare_files(file1: &str, file2: &str) {
    let c1 = fs::read(file1).unwrap();
    let c2 = fs::read(file2).unwrap();
    assert_eq!(c1, c2);
}

#[test]
fn test_store() -> Result<(), Error> {
    if Path::new("tmp").exists() {
        fs::remove_dir_all("tmp").unwrap();
    }

    println!("Prepares files used during tests");
    fs::create_dir_all("tmp/folder").unwrap();
    gen_file("tmp/folder/file1", 512);
    gen_file("tmp/folder/file2", 512);
    gen_file("tmp/folder/file3", 512);
    gen_file("tmp/folder/file4", 512);
    gen_file("tmp/folder/file5", 512);
    gen_file("tmp/file", 512);
    gen_file("tmp/big", 157286912);

    println!("Tests store creation");
    Store::create("tmp/store", "1234")?;

    println!("Journal file needs to exist");
    assert!(Path::new("tmp/store/Store.void").exists());

    println!("It is not possible to create a store where there is one");
    match Store::create("tmp/store", "1234") {
        Ok(_) => panic!("Store created when it should not have been possible."),
        Err(Error::FileAlreadyExistsError) => (),
        Err(err) => panic!("Wrong error!: {:?}", err),
    }

    println!("Tests opening non-existing store");
    match Store::open("tmp/no-store", "1234") {
        Ok(_) => panic!("Should not have opened."),
        Err(Error::FolderDoesNotExistError) => (),
        Err(err) => panic!("Wrong error: {:?}", err),
    }

    println!("Tests opening store");
    let mut store = Store::open("tmp/store", "1234")?;

    println!("Tests adding a file to the store");
    store.add("tmp/file", "/")?;
    store.add("tmp/file", "/ren")?;
    assert_eq!(3, dir_ls_count("tmp/store"));
    let list = store.list("/file")?;
    assert_eq!(list.first().ok_or(Error::CannotDeserializeError)?.0, "file");
    let list = store.list("/ren")?;
    assert_eq!(list.first().ok_or(Error::CannotDeserializeError)?.0, "ren");

    println!("Tests retrivieving");
    let list = store.list("/file")?;
    assert_eq!(list.first().ok_or(Error::CannotDeserializeError)?.0, "file");

    let list = store.list("/")?;
    assert_eq!(list.first().ok_or(Error::CannotDeserializeError)?.0, "file");
    assert_eq!(list.first().ok_or(Error::CannotDeserializeError)?.1, 512);

    store.get("/file", "tmp/got")?;
    compare_files("tmp/file", "tmp/got");

    println!("Tests removing file from store");
    store.remove("/file")?;
    assert_eq!(2, dir_ls_count("tmp/store"));
    store.remove("/ren")?;
    assert_eq!(1, dir_ls_count("tmp/store"));

    println!("Tests adding folder to slash terminated path");
    store.add("tmp/folder", "/subdir/")?;
    assert_eq!(6, dir_ls_count("tmp/store"));
    assert_eq!(1, store.list("/")?.len());
    assert_eq!(1, store.list("/subdir")?.len());
    assert_eq!(5, store.list("/subdir/folder")?.len());

    println!("Tests removing folder");
    store.remove("/subdir")?;
    assert_eq!(1, dir_ls_count("tmp/store"));
    assert_eq!(0, store.list("/")?.len());

    println!("Tests adding folder renaming");
    store.add("tmp/folder", "/subdir")?;
    assert_eq!(6, dir_ls_count("tmp/store"));
    assert_eq!(1, store.list("/")?.len());
    assert_eq!(5, store.list("/subdir")?.len());

    println!("Tests adding file to existing folder");
    store.add("tmp/file", "/subdir")?;
    assert_eq!(7, dir_ls_count("tmp/store"));
    assert_eq!(1, store.list("/")?.len());
    assert_eq!(6, store.list("/subdir")?.len());
    assert_eq!(1, store.list("/subdir/file")?.len());

    println!("Tests decrypting folder");
    store.get("/subdir", "tmp/subdir")?;
    compare_files("tmp/folder/file1", "tmp/subdir/file1");
    compare_files("tmp/folder/file2", "tmp/subdir/file2");
    compare_files("tmp/folder/file3", "tmp/subdir/file3");
    compare_files("tmp/folder/file4", "tmp/subdir/file4");
    compare_files("tmp/folder/file5", "tmp/subdir/file5");

    store.remove("/")?;
    assert_eq!(1, dir_ls_count("tmp/store"));

    println!("Tests adding big file");
    store.add("tmp/big", "/")?;
    assert_eq!(5, dir_ls_count("tmp/store"));
    store.get("/big", "tmp/big2")?;
    compare_files("tmp/big", "tmp/big2");
    store.remove("/")?;
    assert_eq!(1, dir_ls_count("tmp/store"));

    fs::remove_dir_all("tmp").unwrap();

    Ok(())
}
