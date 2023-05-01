/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::collections::HashMap;

pub fn translations() -> HashMap<String, String> {
    let mut ts = HashMap::new();

    ts.insert("create".to_string(), "Create".to_string());
    ts.insert("open".to_string(), "Open".to_string());
    ts.insert("cancel".to_string(), "Cancel".to_string());
    ts.insert("password".to_string(), "Password".to_string());
    ts.insert(
        "create_void_store".to_string(),
        "Create Void Store".to_string(),
    );
    ts.insert("open_void_store".to_string(), "Open Void Store".to_string());
    ts.insert(
        "CannotCreateDirectoryError".to_string(),
        "Could not create directory.".to_string(),
    );
    ts.insert(
        "CannotCreateFileError".to_string(),
        "Could not create file.".to_string(),
    );
    ts.insert(
        "CannotDecryptFileError".to_string(),
        "Wrong password or corrupted file.".to_string(),
    );
    ts.insert(
        "CannotDeserializeError".to_string(),
        "Could not read store file, it may be corrupted.".to_string(),
    );
    ts.insert(
        "CannotParseError".to_string(),
        "Could not read store file, it may be corrupted.".to_string(),
    );
    ts.insert(
        "CannotReadFileError".to_string(),
        "Could not read file.".to_string(),
    );
    ts.insert(
        "CannotRemoveFilesError".to_string(),
        "Could not remove files.".to_string(),
    );
    ts.insert(
        "CannotSerializeError".to_string(),
        "Could not save file.".to_string(),
    );
    ts.insert(
        "CannotWriteFileError".to_string(),
        "Could not write to file.".to_string(),
    );
    ts.insert(
        "FileAlreadyExistsError".to_string(),
        "File already exists.".to_string(),
    );
    ts.insert(
        "FileDoesNotExistError".to_string(),
        "File does not exist.".to_string(),
    );
    ts.insert(
        "FolderDoesNotExistError".to_string(),
        "Folder does not exist.".to_string(),
    );
    ts.insert(
        "StoreFileAlreadyExistsError".to_string(),
        "Store file already exists.".to_string(),
    );
    ts.insert(
        "NoSuchMetadataKey".to_string(),
        "Metadata key not found.".to_string(),
    );
    ts.insert(
        "InternalStructureError".to_string(),
        "Internal unknown error.".to_string(),
    );

    ts.insert(
        "CannotEncryptFileError".to_string(),
        "File could not be encrypted.".to_string(),
    );

    ts
}
