/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 *
 * void_ffi.h — C interface for the void encrypted-store library.
 *
 * Keep this file in sync with void-ffi/src/lib.rs.
 *
 * Memory ownership
 * ----------------
 * - Every function that produces heap-allocated output (strings, arrays)
 *   transfers ownership to the caller.  Free with the matching void_*_free
 *   function listed in the "Memory management" section below.
 * - VoidStore * handles are allocated by void_store_create / void_store_open
 *   and must be freed with void_store_free.
 * - Passing NULL for a required pointer argument returns VOID_ERR_CANNOT_PARSE.
 */

#ifndef VOID_FFI_H
#define VOID_FFI_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* -------------------------------------------------------------------------
 * Error codes
 * ------------------------------------------------------------------------- */

#define VOID_OK                            0
#define VOID_ERR_CANNOT_CREATE_DIRECTORY   1
#define VOID_ERR_CANNOT_CREATE_FILE        2
#define VOID_ERR_CANNOT_ENCRYPT_FILE       3
#define VOID_ERR_CANNOT_DECRYPT_FILE       4
#define VOID_ERR_CANNOT_DESERIALIZE        5
#define VOID_ERR_CANNOT_PARSE              6
#define VOID_ERR_CANNOT_READ_FILE          7
#define VOID_ERR_CANNOT_REMOVE_FILES       8
#define VOID_ERR_CANNOT_SERIALIZE          9
#define VOID_ERR_CANNOT_WRITE_FILE         10
#define VOID_ERR_FILE_ALREADY_EXISTS       11
#define VOID_ERR_FILE_DOES_NOT_EXIST       12
#define VOID_ERR_FOLDER_DOES_NOT_EXIST     13
#define VOID_ERR_STORE_FILE_ALREADY_EXISTS 14
#define VOID_ERR_NO_SUCH_METADATA_KEY      15
#define VOID_ERR_INTERNAL_STRUCTURE        16
#define VOID_ERR_KEY_DERIVATION            17
#define VOID_ERR_UNSUPPORTED_VERSION       18
#define VOID_ERR_NOT_A_FILE                19

/* -------------------------------------------------------------------------
 * Opaque handle
 * ------------------------------------------------------------------------- */

/** Opaque handle to an open void store. Freed with void_store_free. */
typedef struct VoidStore VoidStore;

/* -------------------------------------------------------------------------
 * Data structures
 * ------------------------------------------------------------------------- */

/**
 * A file or directory entry returned by void_store_list /
 * void_store_tag_search.  The name field is owned; it is freed automatically
 * when the containing VoidFileArray is passed to void_file_array_free.
 */
typedef struct {
    uint64_t  id;
    char     *name;    /* heap-allocated, freed by void_file_array_free */
    uint64_t  size;
    bool      is_file;
} VoidFile;

/** Owned array of VoidFile. Free with void_file_array_free. */
typedef struct {
    VoidFile *items;   /* null when len == 0 */
    size_t    len;
} VoidFileArray;

/** Owned byte buffer. Free with void_byte_array_free. */
typedef struct {
    uint8_t *data;     /* null when len == 0 */
    size_t   len;
} VoidByteArray;

/** Owned array of NUL-terminated strings. Free with void_string_array_free. */
typedef struct {
    char  **items;     /* null when len == 0 */
    size_t  len;
} VoidStringArray;

/** A key/value metadata pair. Both strings are freed by void_kv_array_free. */
typedef struct {
    char *key;
    char *value;
} VoidKV;

/** Owned array of VoidKV pairs. Free with void_kv_array_free. */
typedef struct {
    VoidKV *items;     /* null when len == 0 */
    size_t  len;
} VoidKVArray;

/* -------------------------------------------------------------------------
 * Store lifecycle
 * ------------------------------------------------------------------------- */

/**
 * Creates a new encrypted store at path.
 * On success writes the handle to *out and returns VOID_OK.
 * On failure writes NULL to *out and returns an error code.
 */
int void_store_create(const char *path, const char *password, VoidStore **out);

/**
 * Opens an existing store at path.
 * On success writes the handle to *out and returns VOID_OK.
 * On failure writes NULL to *out and returns an error code.
 */
int void_store_open(const char *path, const char *password, VoidStore **out);

/** Frees a store handle. Safe to call with NULL. */
void void_store_free(VoidStore *store);

/* -------------------------------------------------------------------------
 * File operations
 * ------------------------------------------------------------------------- */

/**
 * Encrypts file_path and adds it to the store at store_path.
 * Trailing slash on file_path copies directory contents (rsync semantics).
 */
int void_store_add(VoidStore *store, const char *file_path, const char *store_path);

/**
 * Decrypts the entry at store_path and writes it to file_path on disk.
 * For directories, the entire subtree is written recursively.
 */
int void_store_get(const VoidStore *store, const char *store_path, const char *file_path);

/**
 * Reads the decrypted contents of a file into memory.
 * On success writes a VoidByteArray to *out.  Free with void_byte_array_free.
 */
int void_store_get_bytes(const VoidStore *store, const char *store_path, VoidByteArray *out);

/** Removes a file or directory (recursively) from the store. */
int void_store_remove(VoidStore *store, const char *path);

/** Moves or renames an entry within the store. */
int void_store_mv(VoidStore *store, const char *src, const char *dst);

/** Creates a directory (and any missing parents) inside the store. */
int void_store_mkdir(VoidStore *store, const char *path);

/**
 * Lists entries at path.  Pass "*" to list every file in the store.
 * On success writes a VoidFileArray to *out.  Free with void_file_array_free.
 */
int void_store_list(const VoidStore *store, const char *path, VoidFileArray *out);

/** Removes all encrypted content from a file without deleting its entry. */
int void_store_truncate(VoidStore *store, const char *path);

/* -------------------------------------------------------------------------
 * Metadata
 * ------------------------------------------------------------------------- */

/** Sets a metadata key/value on the node at path. */
int void_store_metadata_set(VoidStore *store, const char *path,
                             const char *key, const char *value);

/**
 * Sets a metadata key/value without persisting to disk.
 * Call void_store_save() to flush accumulated changes.
 */
int void_store_metadata_set_nosave(VoidStore *store, const char *path,
                                    const char *key, const char *value);

/** Persists the store to disk. Use after one or more _nosave operations. */
int void_store_save(VoidStore *store);

/** Removes a metadata key from the node at path. */
int void_store_metadata_remove(VoidStore *store, const char *path, const char *key);

/**
 * Gets the metadata value for key on the node at path.
 * On success writes a heap-allocated string to *out.  Free with void_string_free.
 */
int void_store_metadata_get(const VoidStore *store, const char *path,
                             const char *key, char **out);

/**
 * Returns all metadata on the node at path.
 * On success writes a VoidKVArray to *out.  Free with void_kv_array_free.
 */
int void_store_metadata_list(const VoidStore *store, const char *path, VoidKVArray *out);

/* -------------------------------------------------------------------------
 * Tags
 * ------------------------------------------------------------------------- */

/** Adds a tag to the node at path. */
int void_store_tag_add(VoidStore *store, const char *path, const char *tag);

/** Removes a tag from the node at path. */
int void_store_tag_rm(VoidStore *store, const char *path, const char *tag);

/** Removes all tags from the node at path. */
int void_store_tag_clear(VoidStore *store, const char *path);

/**
 * Returns all distinct tags used anywhere in the store.
 * Free with void_string_array_free.
 */
int void_store_tag_list(const VoidStore *store, VoidStringArray *out);

/**
 * Returns all tags on the node at path.
 * Free with void_string_array_free.
 */
int void_store_tag_get(const VoidStore *store, const char *path, VoidStringArray *out);

/**
 * Returns all nodes whose tags match the given list.
 * Prefix a tag with '!' to match nodes NOT containing that tag.
 * tags is an array of count NUL-terminated strings.
 * Free the result with void_file_array_free.
 */
int void_store_tag_search(const VoidStore *store,
                          const char * const *tags, size_t count,
                          VoidFileArray *out);

/* -------------------------------------------------------------------------
 * GC
 * ------------------------------------------------------------------------- */

/**
 * Removes orphaned chunk files from the store directory.
 * Writes the number of removed files to *removed (may be NULL).
 */
int void_store_gc(const VoidStore *store, size_t *removed);

/* -------------------------------------------------------------------------
 * Memory management
 * ------------------------------------------------------------------------- */

/** Frees a string returned by the library. Safe to call with NULL. */
void void_string_free(char *s);

/** Frees a VoidByteArray. */
void void_byte_array_free(VoidByteArray arr);

/** Frees a VoidFileArray and all strings it contains. */
void void_file_array_free(VoidFileArray arr);

/** Frees a VoidStringArray and all strings it contains. */
void void_string_array_free(VoidStringArray arr);

/** Frees a VoidKVArray and all key/value strings it contains. */
void void_kv_array_free(VoidKVArray arr);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* VOID_FFI_H */
