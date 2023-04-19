use std::collections::HashMap;

pub fn translations() -> HashMap<String, String> {
    let mut ts = HashMap::new();

    ts.insert("create".to_string(), "Criar".to_string());
    ts.insert("open".to_string(), "Abrir".to_string());
    ts.insert("cancel".to_string(), "Cancelar".to_string());
    ts.insert("password".to_string(), "Senha".to_string());
    ts.insert(
        "create_void_store".to_string(),
        "Criar Void Store.".to_string(),
    );
    ts.insert(
        "open_void_store".to_string(),
        "Abrir Void Store.".to_string(),
    );
    ts.insert(
        "CannotCreateDirectoryError".to_string(),
        "Não pude criar um diretório.".to_string(),
    );
    ts.insert(
        "CannotCreateFileError".to_string(),
        "Não pude criar um arquivo.".to_string(),
    );
    ts.insert(
        "CannotDecryptFileError".to_string(),
        "Senha incorreta ou arquivo corrompido.".to_string(),
    );
    ts.insert(
        "CannotDeserializeError".to_string(),
        "Não pude abrir a Store, ela pode estar corrompida.".to_string(),
    );
    ts.insert(
        "CannotParseError".to_string(),
        "Não pude ler a Store, ela pode estar corrompida.".to_string(),
    );
    ts.insert(
        "CannotReadFileError".to_string(),
        "Não pude ler um arquivo.".to_string(),
    );
    ts.insert(
        "CannotRemoveFilesError".to_string(),
        "Não pude apagar arquivos.".to_string(),
    );
    ts.insert(
        "CannotSerializeError".to_string(),
        "Não pude salvar um arquivo.".to_string(),
    );
    ts.insert(
        "CannotWriteFileError".to_string(),
        "Não pude escrever em um arquivo.".to_string(),
    );
    ts.insert(
        "FileAlreadyExistsError".to_string(),
        "Arquivo já existe.".to_string(),
    );
    ts.insert(
        "FileDoesNotExistError".to_string(),
        "Arquivo não existe.".to_string(),
    );
    ts.insert(
        "FolderDoesNotExistError".to_string(),
        "Diretório não existe.".to_string(),
    );
    ts.insert(
        "StoreFileAlreadyExistsError".to_string(),
        "Store já existe.".to_string(),
    );
    ts.insert(
        "NoSuchMetadataKey".to_string(),
        "Chave não encontrada.".to_string(),
    );
    ts.insert(
        "InternalStructureError".to_string(),
        "Erro interno desconhecido.".to_string(),
    );

    ts
}
