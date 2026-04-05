/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! C FFI layer for the void encrypted-store library.
//!
//! # Memory ownership
//!
//! - Functions that return heap-allocated data (`*mut c_char`, `VoidFileArray`,
//!   etc.) transfer ownership to the caller.  The caller **must** free every
//!   returned value with the matching `void_*_free` function.
//! - `VoidStore *` handles are allocated by `void_store_create` /
//!   `void_store_open` and freed with `void_store_free`.
//! - Passing a null pointer for a required argument returns
//!   `VOID_ERR_CANNOT_PARSE`.

use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use void::{Error, File, Store};

// ---------------------------------------------------------------------------
// Error codes
// ---------------------------------------------------------------------------

pub const VOID_OK: i32 = 0;
pub const VOID_ERR_CANNOT_CREATE_DIRECTORY: i32 = 1;
pub const VOID_ERR_CANNOT_CREATE_FILE: i32 = 2;
pub const VOID_ERR_CANNOT_ENCRYPT_FILE: i32 = 3;
pub const VOID_ERR_CANNOT_DECRYPT_FILE: i32 = 4;
pub const VOID_ERR_CANNOT_DESERIALIZE: i32 = 5;
pub const VOID_ERR_CANNOT_PARSE: i32 = 6;
pub const VOID_ERR_CANNOT_READ_FILE: i32 = 7;
pub const VOID_ERR_CANNOT_REMOVE_FILES: i32 = 8;
pub const VOID_ERR_CANNOT_SERIALIZE: i32 = 9;
pub const VOID_ERR_CANNOT_WRITE_FILE: i32 = 10;
pub const VOID_ERR_FILE_ALREADY_EXISTS: i32 = 11;
pub const VOID_ERR_FILE_DOES_NOT_EXIST: i32 = 12;
pub const VOID_ERR_FOLDER_DOES_NOT_EXIST: i32 = 13;
pub const VOID_ERR_STORE_FILE_ALREADY_EXISTS: i32 = 14;
pub const VOID_ERR_NO_SUCH_METADATA_KEY: i32 = 15;
pub const VOID_ERR_INTERNAL_STRUCTURE: i32 = 16;
pub const VOID_ERR_KEY_DERIVATION: i32 = 17;
pub const VOID_ERR_UNSUPPORTED_VERSION: i32 = 18;
pub const VOID_ERR_NOT_A_FILE: i32 = 19;

fn map_err(e: Error) -> i32 {
    match e {
        Error::CannotCreateDirectoryError => VOID_ERR_CANNOT_CREATE_DIRECTORY,
        Error::CannotCreateFileError => VOID_ERR_CANNOT_CREATE_FILE,
        Error::CannotEncryptFileError => VOID_ERR_CANNOT_ENCRYPT_FILE,
        Error::CannotDecryptFileError => VOID_ERR_CANNOT_DECRYPT_FILE,
        Error::CannotDeserializeError => VOID_ERR_CANNOT_DESERIALIZE,
        Error::CannotParseError => VOID_ERR_CANNOT_PARSE,
        Error::CannotReadFileError => VOID_ERR_CANNOT_READ_FILE,
        Error::CannotRemoveFilesError(_) => VOID_ERR_CANNOT_REMOVE_FILES,
        Error::CannotSerializeError => VOID_ERR_CANNOT_SERIALIZE,
        Error::CannotWriteFileError => VOID_ERR_CANNOT_WRITE_FILE,
        Error::FileAlreadyExistsError => VOID_ERR_FILE_ALREADY_EXISTS,
        Error::FileDoesNotExistError => VOID_ERR_FILE_DOES_NOT_EXIST,
        Error::FolderDoesNotExistError => VOID_ERR_FOLDER_DOES_NOT_EXIST,
        Error::NotAFileError => VOID_ERR_NOT_A_FILE,
        Error::StoreFileAlreadyExistsError => VOID_ERR_STORE_FILE_ALREADY_EXISTS,
        Error::NoSuchMetadataKey => VOID_ERR_NO_SUCH_METADATA_KEY,
        Error::InternalStructureError => VOID_ERR_INTERNAL_STRUCTURE,
        Error::KeyDerivationError => VOID_ERR_KEY_DERIVATION,
        Error::UnsupportedVersionError => VOID_ERR_UNSUPPORTED_VERSION,
    }
}

// ---------------------------------------------------------------------------
// Opaque handle
// ---------------------------------------------------------------------------

/// Opaque handle to an open void store. Freed with `void_store_free`.
pub struct VoidStore(Store);

// ---------------------------------------------------------------------------
// C-compatible data structures
// ---------------------------------------------------------------------------

/// A file or directory entry.  The `name` field is a heap-allocated
/// NUL-terminated string freed as part of `void_file_array_free`.
#[repr(C)]
pub struct VoidFile {
    pub id: u64,
    pub name: *mut c_char,
    pub size: u64,
    pub is_file: bool,
}

/// Owned array of `VoidFile`. Free with `void_file_array_free`.
#[repr(C)]
pub struct VoidFileArray {
    pub items: *mut VoidFile,
    pub len: usize,
}

/// Owned byte buffer. Free with `void_byte_array_free`.
#[repr(C)]
pub struct VoidByteArray {
    pub data: *mut u8,
    pub len: usize,
}

/// Owned array of NUL-terminated strings. Free with `void_string_array_free`.
#[repr(C)]
pub struct VoidStringArray {
    pub items: *mut *mut c_char,
    pub len: usize,
}

/// A key/value string pair. Both strings are heap-allocated and freed as
/// part of `void_kv_array_free`.
#[repr(C)]
pub struct VoidKV {
    pub key: *mut c_char,
    pub value: *mut c_char,
}

/// Owned array of `VoidKV` pairs. Free with `void_kv_array_free`.
#[repr(C)]
pub struct VoidKVArray {
    pub items: *mut VoidKV,
    pub len: usize,
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Converts a raw C string pointer to a Rust `&str`.
/// Returns `None` if the pointer is null or contains invalid UTF-8.
unsafe fn to_str<'a>(ptr: *const c_char) -> Option<&'a str> {
    if ptr.is_null() {
        None
    } else {
        CStr::from_ptr(ptr).to_str().ok()
    }
}

/// Clones a Rust `&str` into a heap-allocated C string.
/// Returns null if the string contains an interior NUL byte.
fn to_c_string(s: &str) -> *mut c_char {
    match CString::new(s) {
        Ok(cs) => cs.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

fn files_to_c(files: Vec<File>) -> VoidFileArray {
    if files.is_empty() {
        return VoidFileArray {
            items: std::ptr::null_mut(),
            len: 0,
        };
    }
    let items: Vec<VoidFile> = files
        .into_iter()
        .map(|f| VoidFile {
            id: f.id,
            name: to_c_string(&f.name),
            size: f.size,
            is_file: f.is_file,
        })
        .collect();
    let boxed = items.into_boxed_slice();
    let len = boxed.len();
    let ptr = Box::leak(boxed).as_mut_ptr();
    VoidFileArray { items: ptr, len }
}

fn strings_to_c(strings: Vec<String>) -> VoidStringArray {
    if strings.is_empty() {
        return VoidStringArray {
            items: std::ptr::null_mut(),
            len: 0,
        };
    }
    let items: Vec<*mut c_char> = strings.iter().map(|s| to_c_string(s)).collect();
    let boxed = items.into_boxed_slice();
    let len = boxed.len();
    let ptr = Box::leak(boxed).as_mut_ptr();
    VoidStringArray { items: ptr, len }
}

fn kv_to_c(map: HashMap<String, String>) -> VoidKVArray {
    if map.is_empty() {
        return VoidKVArray {
            items: std::ptr::null_mut(),
            len: 0,
        };
    }
    let items: Vec<VoidKV> = map
        .into_iter()
        .map(|(k, v)| VoidKV {
            key: to_c_string(&k),
            value: to_c_string(&v),
        })
        .collect();
    let boxed = items.into_boxed_slice();
    let len = boxed.len();
    let ptr = Box::leak(boxed).as_mut_ptr();
    VoidKVArray { items: ptr, len }
}

// ---------------------------------------------------------------------------
// Store lifecycle
// ---------------------------------------------------------------------------

/// Creates a new encrypted store at `path`.
///
/// On success writes the store handle to `*out` and returns `VOID_OK`.
/// On failure writes null to `*out` and returns an error code.
#[no_mangle]
pub unsafe extern "C" fn void_store_create(
    path: *const c_char,
    password: *const c_char,
    out: *mut *mut VoidStore,
) -> i32 {
    if out.is_null() {
        return VOID_ERR_CANNOT_PARSE;
    }
    *out = std::ptr::null_mut();
    let path = match to_str(path) {
        Some(s) => s,
        None => return VOID_ERR_CANNOT_PARSE,
    };
    let password = match to_str(password) {
        Some(s) => s,
        None => return VOID_ERR_CANNOT_PARSE,
    };
    match Store::create(path, password) {
        Ok(store) => {
            *out = Box::into_raw(Box::new(VoidStore(store)));
            VOID_OK
        }
        Err(e) => map_err(e),
    }
}

/// Opens an existing store at `path`.
///
/// On success writes the store handle to `*out` and returns `VOID_OK`.
/// On failure writes null to `*out` and returns an error code.
#[no_mangle]
pub unsafe extern "C" fn void_store_open(
    path: *const c_char,
    password: *const c_char,
    out: *mut *mut VoidStore,
) -> i32 {
    if out.is_null() {
        return VOID_ERR_CANNOT_PARSE;
    }
    *out = std::ptr::null_mut();
    let path = match to_str(path) {
        Some(s) => s,
        None => return VOID_ERR_CANNOT_PARSE,
    };
    let password = match to_str(password) {
        Some(s) => s,
        None => return VOID_ERR_CANNOT_PARSE,
    };
    match Store::open(path, password) {
        Ok(store) => {
            *out = Box::into_raw(Box::new(VoidStore(store)));
            VOID_OK
        }
        Err(e) => map_err(e),
    }
}

/// Frees a store handle. Safe to call with null.
#[no_mangle]
pub unsafe extern "C" fn void_store_free(store: *mut VoidStore) {
    if !store.is_null() {
        drop(Box::from_raw(store));
    }
}

// ---------------------------------------------------------------------------
// File operations
// ---------------------------------------------------------------------------

/// Encrypts a file at `file_path` and stores it at `store_path`.
///
/// Follows rsync trailing-slash semantics: a trailing `/` on `file_path`
/// copies the directory contents rather than the directory itself.
#[no_mangle]
pub unsafe extern "C" fn void_store_add(
    store: *mut VoidStore,
    file_path: *const c_char,
    store_path: *const c_char,
) -> i32 {
    let store = match store.as_mut() {
        Some(s) => &mut s.0,
        None => return VOID_ERR_CANNOT_PARSE,
    };
    let file_path = match to_str(file_path) {
        Some(s) => s,
        None => return VOID_ERR_CANNOT_PARSE,
    };
    let store_path = match to_str(store_path) {
        Some(s) => s,
        None => return VOID_ERR_CANNOT_PARSE,
    };
    store
        .add(file_path, store_path)
        .map_or_else(map_err, |_| VOID_OK)
}

/// Like [`void_store_add`] but writes the total number of bytes processed to
/// `*bytes_done` when the operation completes.
///
/// For **real-time** progress the caller should run this function on a worker
/// thread and periodically read `*bytes_done` from the main thread (the value
/// is updated atomically during the operation).
///
/// `bytes_done` may be null, in which case progress is silently discarded.
///
/// # Safety
///
/// `bytes_done`, if not null, must point to a valid `uint64_t` that remains
/// alive for the entire duration of the call.
#[no_mangle]
pub unsafe extern "C" fn void_store_add_with_progress(
    store: *mut VoidStore,
    file_path: *const c_char,
    store_path: *const c_char,
    bytes_done: *mut u64,
) -> i32 {
    let store = match store.as_mut() {
        Some(s) => &mut s.0,
        None => return VOID_ERR_CANNOT_PARSE,
    };
    let file_path = match to_str(file_path) {
        Some(s) => s,
        None => return VOID_ERR_CANNOT_PARSE,
    };
    let store_path = match to_str(store_path) {
        Some(s) => s,
        None => return VOID_ERR_CANNOT_PARSE,
    };
    let counter = Arc::new(AtomicU64::new(0));
    let result = store
        .add_with_progress(file_path, store_path, counter.clone())
        .map_or_else(map_err, |_| VOID_OK);
    if !bytes_done.is_null() {
        *bytes_done = counter.load(Ordering::Relaxed);
    }
    result
}

/// Decrypts the entry at `store_path` and writes it to `file_path` on disk.
#[no_mangle]
pub unsafe extern "C" fn void_store_get(
    store: *const VoidStore,
    store_path: *const c_char,
    file_path: *const c_char,
) -> i32 {
    let store = match store.as_ref() {
        Some(s) => &s.0,
        None => return VOID_ERR_CANNOT_PARSE,
    };
    let store_path = match to_str(store_path) {
        Some(s) => s,
        None => return VOID_ERR_CANNOT_PARSE,
    };
    let file_path = match to_str(file_path) {
        Some(s) => s,
        None => return VOID_ERR_CANNOT_PARSE,
    };
    store
        .get(store_path, file_path)
        .map_or_else(map_err, |_| VOID_OK)
}

/// Reads the decrypted contents of a file into memory.
///
/// On success writes a `VoidByteArray` to `*out` and returns `VOID_OK`.
/// Free the array with `void_byte_array_free`.
#[no_mangle]
pub unsafe extern "C" fn void_store_get_bytes(
    store: *const VoidStore,
    store_path: *const c_char,
    out: *mut VoidByteArray,
) -> i32 {
    if out.is_null() {
        return VOID_ERR_CANNOT_PARSE;
    }
    let store = match store.as_ref() {
        Some(s) => &s.0,
        None => return VOID_ERR_CANNOT_PARSE,
    };
    let store_path = match to_str(store_path) {
        Some(s) => s,
        None => return VOID_ERR_CANNOT_PARSE,
    };
    match store.get_bytes(store_path) {
        Ok(bytes) => {
            let mut boxed = bytes.into_boxed_slice();
            *out = VoidByteArray {
                len: boxed.len(),
                data: boxed.as_mut_ptr(),
            };
            std::mem::forget(boxed);
            VOID_OK
        }
        Err(e) => map_err(e),
    }
}

/// Removes a file or directory (recursively) from the store.
#[no_mangle]
pub unsafe extern "C" fn void_store_remove(store: *mut VoidStore, path: *const c_char) -> i32 {
    let store = match store.as_mut() {
        Some(s) => &mut s.0,
        None => return VOID_ERR_CANNOT_PARSE,
    };
    let path = match to_str(path) {
        Some(s) => s,
        None => return VOID_ERR_CANNOT_PARSE,
    };
    store.remove(path).map_or_else(map_err, |_| VOID_OK)
}

/// Moves or renames an entry within the store.
#[no_mangle]
pub unsafe extern "C" fn void_store_mv(
    store: *mut VoidStore,
    src: *const c_char,
    dst: *const c_char,
) -> i32 {
    let store = match store.as_mut() {
        Some(s) => &mut s.0,
        None => return VOID_ERR_CANNOT_PARSE,
    };
    let src = match to_str(src) {
        Some(s) => s,
        None => return VOID_ERR_CANNOT_PARSE,
    };
    let dst = match to_str(dst) {
        Some(s) => s,
        None => return VOID_ERR_CANNOT_PARSE,
    };
    store.mv(src, dst).map_or_else(map_err, |_| VOID_OK)
}

/// Creates a directory (and any missing parents) inside the store.
#[no_mangle]
pub unsafe extern "C" fn void_store_mkdir(store: *mut VoidStore, path: *const c_char) -> i32 {
    let store = match store.as_mut() {
        Some(s) => &mut s.0,
        None => return VOID_ERR_CANNOT_PARSE,
    };
    let path = match to_str(path) {
        Some(s) => s,
        None => return VOID_ERR_CANNOT_PARSE,
    };
    store.mkdir(path).map_or_else(map_err, |_| VOID_OK)
}

/// Lists files at `path`. Pass `"*"` to list all files in the store.
///
/// On success writes a `VoidFileArray` to `*out` and returns `VOID_OK`.
/// Free the array with `void_file_array_free`.
#[no_mangle]
pub unsafe extern "C" fn void_store_list(
    store: *const VoidStore,
    path: *const c_char,
    out: *mut VoidFileArray,
) -> i32 {
    if out.is_null() {
        return VOID_ERR_CANNOT_PARSE;
    }
    let store = match store.as_ref() {
        Some(s) => &s.0,
        None => return VOID_ERR_CANNOT_PARSE,
    };
    let path = match to_str(path) {
        Some(s) => s,
        None => return VOID_ERR_CANNOT_PARSE,
    };
    match store.list(path) {
        Ok(files) => {
            *out = files_to_c(files);
            VOID_OK
        }
        Err(e) => map_err(e),
    }
}

/// Removes all encrypted content from a file without deleting the entry.
#[no_mangle]
pub unsafe extern "C" fn void_store_truncate(store: *mut VoidStore, path: *const c_char) -> i32 {
    let store = match store.as_mut() {
        Some(s) => &mut s.0,
        None => return VOID_ERR_CANNOT_PARSE,
    };
    let path = match to_str(path) {
        Some(s) => s,
        None => return VOID_ERR_CANNOT_PARSE,
    };
    store.truncate(path).map_or_else(map_err, |_| VOID_OK)
}

// ---------------------------------------------------------------------------
// Metadata
// ---------------------------------------------------------------------------

/// Sets a metadata key/value pair on the node at `path`.
#[no_mangle]
pub unsafe extern "C" fn void_store_metadata_set(
    store: *mut VoidStore,
    path: *const c_char,
    key: *const c_char,
    value: *const c_char,
) -> i32 {
    let store = match store.as_mut() {
        Some(s) => &mut s.0,
        None => return VOID_ERR_CANNOT_PARSE,
    };
    let (path, key, value) = match (to_str(path), to_str(key), to_str(value)) {
        (Some(p), Some(k), Some(v)) => (p, k, v),
        _ => return VOID_ERR_CANNOT_PARSE,
    };
    store
        .metadata_set(path, key, value)
        .map_or_else(map_err, |_| VOID_OK)
}

/// Sets a metadata key/value without persisting to disk.
/// Call `void_store_save` to flush accumulated changes.
#[no_mangle]
pub unsafe extern "C" fn void_store_metadata_set_nosave(
    store: *mut VoidStore,
    path: *const c_char,
    key: *const c_char,
    value: *const c_char,
) -> i32 {
    let store = match store.as_mut() {
        Some(s) => &mut s.0,
        None => return VOID_ERR_CANNOT_PARSE,
    };
    let (path, key, value) = match (to_str(path), to_str(key), to_str(value)) {
        (Some(p), Some(k), Some(v)) => (p, k, v),
        _ => return VOID_ERR_CANNOT_PARSE,
    };
    store
        .metadata_set_nosave(path, key, value)
        .map_or_else(map_err, |_| VOID_OK)
}

/// Persists the store to disk. Use after one or more `_nosave` operations.
#[no_mangle]
pub unsafe extern "C" fn void_store_save(store: *mut VoidStore) -> i32 {
    let store = match store.as_mut() {
        Some(s) => &mut s.0,
        None => return VOID_ERR_CANNOT_PARSE,
    };
    store.save().map_or_else(map_err, |_| VOID_OK)
}

/// Removes a metadata key from the node at `path`.
#[no_mangle]
pub unsafe extern "C" fn void_store_metadata_remove(
    store: *mut VoidStore,
    path: *const c_char,
    key: *const c_char,
) -> i32 {
    let store = match store.as_mut() {
        Some(s) => &mut s.0,
        None => return VOID_ERR_CANNOT_PARSE,
    };
    let (path, key) = match (to_str(path), to_str(key)) {
        (Some(p), Some(k)) => (p, k),
        _ => return VOID_ERR_CANNOT_PARSE,
    };
    store
        .metadata_remove(path, key)
        .map_or_else(map_err, |_| VOID_OK)
}

/// Gets the metadata value for `key` on the node at `path`.
///
/// On success writes a heap-allocated string to `*out` and returns `VOID_OK`.
/// Free the string with `void_string_free`.
#[no_mangle]
pub unsafe extern "C" fn void_store_metadata_get(
    store: *const VoidStore,
    path: *const c_char,
    key: *const c_char,
    out: *mut *mut c_char,
) -> i32 {
    if out.is_null() {
        return VOID_ERR_CANNOT_PARSE;
    }
    *out = std::ptr::null_mut();
    let store = match store.as_ref() {
        Some(s) => &s.0,
        None => return VOID_ERR_CANNOT_PARSE,
    };
    let (path, key) = match (to_str(path), to_str(key)) {
        (Some(p), Some(k)) => (p, k),
        _ => return VOID_ERR_CANNOT_PARSE,
    };
    match store.metadata_get(path, key) {
        Ok(value) => {
            *out = to_c_string(&value);
            VOID_OK
        }
        Err(e) => map_err(e),
    }
}

/// Returns all metadata on the node at `path` as a key/value array.
///
/// Free with `void_kv_array_free`.
#[no_mangle]
pub unsafe extern "C" fn void_store_metadata_list(
    store: *const VoidStore,
    path: *const c_char,
    out: *mut VoidKVArray,
) -> i32 {
    if out.is_null() {
        return VOID_ERR_CANNOT_PARSE;
    }
    let store = match store.as_ref() {
        Some(s) => &s.0,
        None => return VOID_ERR_CANNOT_PARSE,
    };
    let path = match to_str(path) {
        Some(s) => s,
        None => return VOID_ERR_CANNOT_PARSE,
    };
    match store.metadata_list(path) {
        Ok(map) => {
            *out = kv_to_c(map);
            VOID_OK
        }
        Err(e) => map_err(e),
    }
}

// ---------------------------------------------------------------------------
// Tags
// ---------------------------------------------------------------------------

/// Adds a tag to the node at `path`.
#[no_mangle]
pub unsafe extern "C" fn void_store_tag_add(
    store: *mut VoidStore,
    path: *const c_char,
    tag: *const c_char,
) -> i32 {
    let store = match store.as_mut() {
        Some(s) => &mut s.0,
        None => return VOID_ERR_CANNOT_PARSE,
    };
    let (path, tag) = match (to_str(path), to_str(tag)) {
        (Some(p), Some(t)) => (p, t),
        _ => return VOID_ERR_CANNOT_PARSE,
    };
    store.tag_add(path, tag).map_or_else(map_err, |_| VOID_OK)
}

/// Removes a tag from the node at `path`.
#[no_mangle]
pub unsafe extern "C" fn void_store_tag_rm(
    store: *mut VoidStore,
    path: *const c_char,
    tag: *const c_char,
) -> i32 {
    let store = match store.as_mut() {
        Some(s) => &mut s.0,
        None => return VOID_ERR_CANNOT_PARSE,
    };
    let (path, tag) = match (to_str(path), to_str(tag)) {
        (Some(p), Some(t)) => (p, t),
        _ => return VOID_ERR_CANNOT_PARSE,
    };
    store.tag_rm(path, tag).map_or_else(map_err, |_| VOID_OK)
}

/// Removes all tags from the node at `path`.
#[no_mangle]
pub unsafe extern "C" fn void_store_tag_clear(store: *mut VoidStore, path: *const c_char) -> i32 {
    let store = match store.as_mut() {
        Some(s) => &mut s.0,
        None => return VOID_ERR_CANNOT_PARSE,
    };
    let path = match to_str(path) {
        Some(s) => s,
        None => return VOID_ERR_CANNOT_PARSE,
    };
    store.tag_clear(path).map_or_else(map_err, |_| VOID_OK)
}

/// Returns all distinct tags used anywhere in the store.
///
/// Free with `void_string_array_free`.
#[no_mangle]
pub unsafe extern "C" fn void_store_tag_list(
    store: *const VoidStore,
    out: *mut VoidStringArray,
) -> i32 {
    if out.is_null() {
        return VOID_ERR_CANNOT_PARSE;
    }
    let store = match store.as_ref() {
        Some(s) => &s.0,
        None => return VOID_ERR_CANNOT_PARSE,
    };
    *out = strings_to_c(store.tag_list());
    VOID_OK
}

/// Returns all tags on the node at `path`.
///
/// Free with `void_string_array_free`.
#[no_mangle]
pub unsafe extern "C" fn void_store_tag_get(
    store: *const VoidStore,
    path: *const c_char,
    out: *mut VoidStringArray,
) -> i32 {
    if out.is_null() {
        return VOID_ERR_CANNOT_PARSE;
    }
    let store = match store.as_ref() {
        Some(s) => &s.0,
        None => return VOID_ERR_CANNOT_PARSE,
    };
    let path = match to_str(path) {
        Some(s) => s,
        None => return VOID_ERR_CANNOT_PARSE,
    };
    match store.tag_get(path) {
        Ok(tags) => {
            *out = strings_to_c(tags);
            VOID_OK
        }
        Err(e) => map_err(e),
    }
}

/// Returns all nodes whose tags match the given list.
///
/// Prefix a tag with `!` to search for nodes NOT containing that tag.
/// `tags` is an array of `count` NUL-terminated strings.
///
/// Free the result with `void_file_array_free`.
#[no_mangle]
pub unsafe extern "C" fn void_store_tag_search(
    store: *const VoidStore,
    tags: *const *const c_char,
    count: usize,
    out: *mut VoidFileArray,
) -> i32 {
    if out.is_null() {
        return VOID_ERR_CANNOT_PARSE;
    }
    let store = match store.as_ref() {
        Some(s) => &s.0,
        None => return VOID_ERR_CANNOT_PARSE,
    };
    let tag_vec: Vec<String> = if count == 0 {
        vec![]
    } else if tags.is_null() {
        return VOID_ERR_CANNOT_PARSE;
    } else {
        (0..count)
            .filter_map(|i| to_str(*tags.add(i)).map(String::from))
            .collect()
    };
    *out = files_to_c(store.tag_search(tag_vec));
    VOID_OK
}

// ---------------------------------------------------------------------------
// GC
// ---------------------------------------------------------------------------

/// Removes orphaned chunk files from the store directory.
///
/// Orphaned chunks can accumulate when a `save()` fails after chunks have
/// already been written (e.g. disk full mid-add). Writes the number of
/// removed files to `*removed` (may be null if the count is not needed).
#[no_mangle]
pub unsafe extern "C" fn void_store_gc(store: *const VoidStore, removed: *mut usize) -> i32 {
    let store = match store.as_ref() {
        Some(s) => &s.0,
        None => return VOID_ERR_CANNOT_PARSE,
    };
    match store.gc() {
        Ok(n) => {
            if !removed.is_null() {
                *removed = n;
            }
            VOID_OK
        }
        Err(e) => map_err(e),
    }
}

/// Re-encrypts the store index with a new password.
/// Returns `VOID_OK` on success.
#[no_mangle]
pub unsafe extern "C" fn void_store_change_password(
    store: *mut VoidStore,
    new_password: *const c_char,
) -> i32 {
    let store = match store.as_mut() {
        Some(s) => &mut s.0,
        None => return VOID_ERR_CANNOT_PARSE,
    };
    let new_password = match new_password.as_ref() {
        Some(_) => match CStr::from_ptr(new_password).to_str() {
            Ok(s) => s,
            Err(_) => return VOID_ERR_CANNOT_PARSE,
        },
        None => return VOID_ERR_CANNOT_PARSE,
    };
    match store.change_password(new_password) {
        Ok(()) => VOID_OK,
        Err(e) => map_err(e),
    }
}

// ---------------------------------------------------------------------------
// Memory management
// ---------------------------------------------------------------------------

/// Frees a string returned by the library. Safe to call with null.
#[no_mangle]
pub unsafe extern "C" fn void_string_free(s: *mut c_char) {
    if !s.is_null() {
        drop(CString::from_raw(s));
    }
}

/// Frees a `VoidByteArray` returned by the library.
#[no_mangle]
pub unsafe extern "C" fn void_byte_array_free(arr: VoidByteArray) {
    if arr.data.is_null() || arr.len == 0 {
        return;
    }
    drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
        arr.data, arr.len,
    )));
}

/// Frees a `VoidFileArray` returned by the library.
#[no_mangle]
pub unsafe extern "C" fn void_file_array_free(arr: VoidFileArray) {
    if arr.items.is_null() || arr.len == 0 {
        return;
    }
    let slice = std::slice::from_raw_parts(arr.items, arr.len);
    for item in slice {
        if !item.name.is_null() {
            drop(CString::from_raw(item.name));
        }
    }
    drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
        arr.items, arr.len,
    )));
}

/// Frees a `VoidStringArray` returned by the library.
#[no_mangle]
pub unsafe extern "C" fn void_string_array_free(arr: VoidStringArray) {
    if arr.items.is_null() || arr.len == 0 {
        return;
    }
    let slice = std::slice::from_raw_parts(arr.items, arr.len);
    for &item in slice {
        if !item.is_null() {
            drop(CString::from_raw(item));
        }
    }
    drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
        arr.items, arr.len,
    )));
}

/// Frees a `VoidKVArray` returned by the library.
#[no_mangle]
pub unsafe extern "C" fn void_kv_array_free(arr: VoidKVArray) {
    if arr.items.is_null() || arr.len == 0 {
        return;
    }
    let slice = std::slice::from_raw_parts(arr.items, arr.len);
    for item in slice {
        if !item.key.is_null() {
            drop(CString::from_raw(item.key));
        }
        if !item.value.is_null() {
            drop(CString::from_raw(item.value));
        }
    }
    drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
        arr.items, arr.len,
    )));
}
