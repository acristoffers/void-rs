use std::collections::HashMap;

pub fn translations() -> HashMap<String, String> {
    let mut ts = HashMap::new();

    ts.insert("create".to_string(), "Créer".to_string());
    ts.insert("open".to_string(), "Ouvrir".to_string());
    ts.insert("cancel".to_string(), "Annuler".to_string());
    ts.insert("password".to_string(), "Mot de passe".to_string());
    ts.insert(
        "CannotCreateDirectoryError".to_string(),
        "Impossible de créer le dossier.".to_string(),
    );
    ts.insert(
        "create_void_store".to_string(),
        "Créer Void Store.".to_string(),
    );
    ts.insert(
        "open_void_store".to_string(),
        "Ouvrir Void Store.".to_string(),
    );
    ts.insert(
        "CannotCreateFileError".to_string(),
        "Impossible de créer le fichier.".to_string(),
    );
    ts.insert(
        "CannotDecryptFileError".to_string(),
        "Mot de passe faux ou fichier corrompu.".to_string(),
    );
    ts.insert(
        "CannotDeserializeError".to_string(),
        "Impossible de lire le ficher de le Store, il est peut-être corrompu.".to_string(),
    );
    ts.insert(
        "CannotParseError".to_string(),
        "Impossible de lire le ficher de le Store, il est peut-être corrompu.".to_string(),
    );
    ts.insert(
        "CannotReadFileError".to_string(),
        "Impossible de lire le fichier.".to_string(),
    );
    ts.insert(
        "CannotRemoveFilesError".to_string(),
        "Impossible de effacer un fichier.".to_string(),
    );
    ts.insert(
        "CannotSerializeError".to_string(),
        "Impossible d'enregistrer le fichier.".to_string(),
    );
    ts.insert(
        "CannotWriteFileError".to_string(),
        "Impossible d'enregistrer le fichier.".to_string(),
    );
    ts.insert(
        "FileAlreadyExistsError".to_string(),
        "Le fichier existe déjà.".to_string(),
    );
    ts.insert(
        "FileDoesNotExistError".to_string(),
        "Le fichier n'existe pas.".to_string(),
    );
    ts.insert(
        "FolderDoesNotExistError".to_string(),
        "Le dossier n'existe pas.".to_string(),
    );
    ts.insert(
        "StoreFileAlreadyExistsError".to_string(),
        "Le Store existe déjà.".to_string(),
    );
    ts.insert(
        "NoSuchMetadataKey".to_string(),
        "La clé n'était pas trouvé.".to_string(),
    );
    ts.insert(
        "InternalStructureError".to_string(),
        "Erreur interne inconnu.".to_string(),
    );

    ts
}
