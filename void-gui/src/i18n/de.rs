use std::collections::HashMap;

pub fn translations() -> HashMap<String, String> {
    let mut ts = HashMap::new();

    ts.insert("create".to_string(), "Neu Store".to_string());
    ts.insert("open".to_string(), "Öffnen".to_string());
    ts.insert("cancel".to_string(), "Abbrechen".to_string());
    ts.insert("password".to_string(), "Passwort".to_string());
    ts.insert(
        "create_void_store".to_string(),
        "Void Store Erstellen.".to_string(),
    );
    ts.insert(
        "open_void_store".to_string(),
        "Void Store Öffnen.".to_string(),
    );
    ts.insert(
        "CannotCreateDirectoryError".to_string(),
        "Ordner konnte nicht erstellt werden.".to_string(),
    );
    ts.insert(
        "CannotCreateFileError".to_string(),
        "Datei konnte nicht erstellt werden.".to_string(),
    );
    ts.insert(
        "CannotDecryptFileError".to_string(),
        "Falsches Passwort oder beschädigte Datei.".to_string(),
    );
    ts.insert(
        "CannotDeserializeError".to_string(),
        "Datei konnte nicht gelesen werden, es kann beschädigte sein.".to_string(),
    );
    ts.insert(
        "CannotParseError".to_string(),
        "Datei konnte nicht gelesen werden, es kann beschädigte sein.".to_string(),
    );
    ts.insert(
        "CannotReadFileError".to_string(),
        "Datei konnte nicht gelesen werden.".to_string(),
    );
    ts.insert(
        "CannotRemoveFilesError".to_string(),
        "Datei konnte nicht erlöschen werden.".to_string(),
    );
    ts.insert(
        "CannotSerializeError".to_string(),
        "Datei konnte nicht erstellen werden.".to_string(),
    );
    ts.insert(
        "CannotWriteFileError".to_string(),
        "Datei konnte nicht erstellen werden.".to_string(),
    );
    ts.insert(
        "FileAlreadyExistsError".to_string(),
        "Datei existiert bereits.".to_string(),
    );
    ts.insert(
        "FileDoesNotExistError".to_string(),
        "Detei existiert nicht.".to_string(),
    );
    ts.insert(
        "FolderDoesNotExistError".to_string(),
        "Ordner existiert nicht.".to_string(),
    );
    ts.insert(
        "StoreFileAlreadyExistsError".to_string(),
        "Store existiert bereits.".to_string(),
    );
    ts.insert(
        "NoSuchMetadataKey".to_string(),
        "Schlüßel existiert nicht.".to_string(),
    );
    ts.insert(
        "InternalStructureError".to_string(),
        "Unbekannter interner Fehler.".to_string(),
    );

    ts.insert(
        "CannotEncryptFileError".to_string(),
        "Datei konnte nicht geschlüßelt sein.".to_string(),
    );

    ts
}
