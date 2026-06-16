# Void

An encrypted file store with a filesystem-like structure.

Void lets you store files and folders inside an encrypted vault. You can add, extract, move, rename,
and organise entries just like a regular filesystem — except everything is encrypted at rest with
AES-256-GCM and a password-derived key (Argon2). Files also support custom metadata and tags, with
search and filtering built in.

<p align="center">
  <img src="https://acristoffers.me/screenshots/Void1.png" width="400" />
  <img src="https://acristoffers.me/screenshots/Void2.png" width="400" />
</p>
<p align="center">
  <img src="https://acristoffers.me/screenshots/Void3.png" width="400" />
  <img src="https://acristoffers.me/screenshots/Void4.png" width="400" />
</p>

## GUI

The graphical interface is a GNOME/Adwaita application that provides a full file manager experience
for your vault:

- **File grid** with adjustable icon sizes and thumbnail previews for images and videos.
- **Folder tree** sidebar for quick navigation.
- **Breadcrumb path bar** with direct path editing (<kbd>Ctrl</kbd>+<kbd>L</kbd>).
- **Filter** (<kbd>Ctrl</kbd>+<kbd>F</kbd>) and **Search** (<kbd>Ctrl</kbd>+<kbd>/</kbd>) with regex and `tag:value` support.
- **Drag & drop** — drag files in to import, drag files out to export.
- **Copy, cut, paste, rename, delete** with familiar keyboard shortcuts.
- **In-app viewers** — images open in a picture window, text files in an editor with save-back support.
- **Info pane** — view and edit file size, custom metadata, and tags.
- **Dark/light/system theme** support.

## CLI

The `void-cli` command-line tool exposes the full vault API for scripting and headless use:

```
void-cli create <store>
void-cli add -s <store> <internal_path> <files...>
void-cli get -s <store> <internal_path> <external_path>
void-cli ls -s <store> [-l] [-H] <path>
void-cli rm -s <store> <path>
void-cli mkdir -s <store> <path>
void-cli change-password -s <store>
void-cli gc -s <store>
```

Metadata and tags are first-class:

```
void-cli metadata-set -s <store> <path> <key> <value>
void-cli metadata-get -s <store> <path> <key>
void-cli metadata-list -s <store> <path>
void-cli tag-add -s <store> <path> <tags...>
void-cli tag-search -s <store> <tags...>
```

Set `VOID_STORE` and `VOID_PSWD` environment variables to avoid repeating `-s` and `-p` on every
call.

## Project Structure

| Crate       | Description                                      |
| ----------- | ------------------------------------------------ |
| `void`      | Core library — encryption, storage, and file API |
| `void-cli`  | Command-line interface                           |
| `void-gui`  | GTK 4 / Adwaita graphical interface              |
| `void-ffi`  | C-compatible FFI bindings for the core library   |

## Security

- **Encryption**: AES-256-GCM (authenticated encryption).
- **Key derivation**: Argon2 with a random salt.
- **Integrity**: BLAKE2 content-addressed chunks — tampering is detected on read.
- **At rest**: Every file chunk is individually encrypted. The store index is also encrypted.

## Installation

There are packages for Ubuntu and Fedora in my [personal repository](https://github.com/acristoffers/repository).

## License

[Mozilla Public License 2.0](LICENSE)
