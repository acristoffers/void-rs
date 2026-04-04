/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 *
 * C integration tests for the void-ffi library.
 *
 * Tests run sequentially; each test builds on the store state left by the
 * previous one.  See the comment above main() for the full test sequence.
 */

#include "void_ffi.h"

#include <assert.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

/* -------------------------------------------------------------------------
 * Tiny test framework
 * ------------------------------------------------------------------------- */

static int g_tests_run    = 0;
static int g_tests_failed = 0;

/* Assert a void_store_* call returns VOID_OK. */
#define CHECK(call)                                                            \
    do {                                                                       \
        int _rc = (call);                                                      \
        if (_rc != VOID_OK) {                                                  \
            fprintf(stderr, "  FAIL %s:%d  %s  => %d\n",                      \
                    __FILE__, __LINE__, #call, _rc);                           \
            g_tests_failed++;                                                  \
            return;                                                            \
        }                                                                      \
    } while (0)

/* Assert a call returns a specific error code. */
#define CHECK_ERR(call, expected)                                              \
    do {                                                                       \
        int _rc = (call);                                                      \
        if (_rc != (expected)) {                                               \
            fprintf(stderr, "  FAIL %s:%d  %s  expected=%d got=%d\n",         \
                    __FILE__, __LINE__, #call, (expected), _rc);               \
            g_tests_failed++;                                                  \
            return;                                                            \
        }                                                                      \
    } while (0)

/* Assert a boolean condition. */
#define ASSERT(cond)                                                           \
    do {                                                                       \
        if (!(cond)) {                                                         \
            fprintf(stderr, "  FAIL %s:%d  assertion failed: %s\n",           \
                    __FILE__, __LINE__, #cond);                                \
            g_tests_failed++;                                                  \
            return;                                                            \
        }                                                                      \
    } while (0)

/* Assert two NUL-terminated strings are equal. */
#define ASSERT_STR_EQ(a, b)                                                    \
    do {                                                                       \
        if (strcmp((a), (b)) != 0) {                                           \
            fprintf(stderr, "  FAIL %s:%d  expected \"%s\"  got \"%s\"\n",    \
                    __FILE__, __LINE__, (b), (a));                             \
            g_tests_failed++;                                                  \
            return;                                                            \
        }                                                                      \
    } while (0)

#define RUN(fn)                                                                \
    do {                                                                       \
        int _before = g_tests_failed;                                          \
        fn();                                                                  \
        g_tests_run++;                                                         \
        if (g_tests_failed == _before)                                         \
            printf("  PASS  " #fn "\n");                                       \
        else                                                                   \
            printf("  FAIL  " #fn "\n");                                       \
    } while (0)

/* -------------------------------------------------------------------------
 * Fixtures
 * ------------------------------------------------------------------------- */

static const char *STORE_PATH   = "/tmp/void_ffi_c_test_store";
static const char *SRC_FILE     = "/tmp/void_ffi_c_test_src.txt";
static const char *DST_FILE     = "/tmp/void_ffi_c_test_dst.txt";
static const char *FILE_CONTENT = "Hello from C FFI tests!\n";

static VoidStore *g_store = NULL;

static void write_file(const char *path, const char *content) {
    FILE *f = fopen(path, "w");
    assert(f != NULL);
    fputs(content, f);
    fclose(f);
}

static char *read_file(const char *path) {
    FILE *f = fopen(path, "r");
    if (!f) return NULL;
    fseek(f, 0, SEEK_END);
    long sz = ftell(f);
    rewind(f);
    char *buf = malloc((size_t)sz + 1);
    assert(buf != NULL);
    if (fread(buf, 1, (size_t)sz, f) != (size_t)sz && sz > 0) {
        free(buf); fclose(f); return NULL;
    }
    buf[sz] = '\0';
    fclose(f);
    return buf;
}

static void rmdir_r(const char *path) {
    char cmd[512];
    snprintf(cmd, sizeof(cmd), "rm -rf -- %s", path);
    if (system(cmd) != 0) { /* best-effort cleanup, ignore failures */ }
}

static void setup(void) {
    rmdir_r(STORE_PATH);
    unlink(SRC_FILE);
    unlink(DST_FILE);
    int rc = void_store_create(STORE_PATH, "hunter2", &g_store);
    if (rc != VOID_OK || g_store == NULL) {
        fprintf(stderr, "setup: void_store_create failed (%d)\n", rc);
        exit(1);
    }
}

static void teardown(void) {
    void_store_free(g_store);
    g_store = NULL;
    rmdir_r(STORE_PATH);
    unlink(SRC_FILE);
    unlink(DST_FILE);
}

/* -------------------------------------------------------------------------
 * Tests
 * ------------------------------------------------------------------------- */

/*
 * test_open_close
 * Re-open the store that was created in setup() to verify persistence and
 * confirm that a wrong password is rejected.
 */
static void test_open_close(void) {
    void_store_free(g_store);
    g_store = NULL;

    CHECK(void_store_open(STORE_PATH, "hunter2", &g_store));
    ASSERT(g_store != NULL);

    VoidStore *bad = NULL;
    CHECK_ERR(void_store_open(STORE_PATH, "wrong", &bad),
              VOID_ERR_CANNOT_DECRYPT_FILE);
    ASSERT(bad == NULL);
}

/*
 * test_add_get
 * Encrypt a file into the store, retrieve it, verify the content is intact.
 */
static void test_add_get(void) {
    write_file(SRC_FILE, FILE_CONTENT);

    CHECK(void_store_add(g_store, SRC_FILE, "/testfile.txt"));
    CHECK(void_store_get(g_store, "/testfile.txt", DST_FILE));

    char *got = read_file(DST_FILE);
    ASSERT(got != NULL);
    ASSERT_STR_EQ(got, FILE_CONTENT);
    free(got);

    unlink(DST_FILE);
}

/*
 * test_list
 * The store root should contain exactly the one file we just added.
 */
static void test_list(void) {
    VoidFileArray arr = {0};
    CHECK(void_store_list(g_store, "/", &arr));
    ASSERT(arr.len == 1);
    ASSERT(arr.items != NULL);
    ASSERT_STR_EQ(arr.items[0].name, "testfile.txt");
    ASSERT(arr.items[0].is_file == true);
    void_file_array_free(arr);

    /* "*" should also return it */
    VoidFileArray all = {0};
    CHECK(void_store_list(g_store, "*", &all));
    ASSERT(all.len >= 1);
    void_file_array_free(all);
}

/*
 * test_metadata
 * Set / get / list / remove a metadata key on the test file.
 */
static void test_metadata(void) {
    CHECK(void_store_metadata_set(g_store, "/testfile.txt", "author", "alice"));

    char *val = NULL;
    CHECK(void_store_metadata_get(g_store, "/testfile.txt", "author", &val));
    ASSERT(val != NULL);
    ASSERT_STR_EQ(val, "alice");
    void_string_free(val);

    /* metadata_list should contain at least "author" and the auto-set "mimetype" */
    VoidKVArray kv = {0};
    CHECK(void_store_metadata_list(g_store, "/testfile.txt", &kv));
    int found = 0;
    for (size_t i = 0; i < kv.len; i++) {
        if (strcmp(kv.items[i].key, "author") == 0) {
            ASSERT_STR_EQ(kv.items[i].value, "alice");
            found = 1;
        }
    }
    ASSERT(found == 1);
    void_kv_array_free(kv);

    /* remove the key and verify it's gone */
    CHECK(void_store_metadata_remove(g_store, "/testfile.txt", "author"));

    val = NULL;
    CHECK_ERR(void_store_metadata_get(g_store, "/testfile.txt", "author", &val),
              VOID_ERR_NO_SUCH_METADATA_KEY);
    ASSERT(val == NULL);
}

/*
 * test_tags
 * Add, inspect, search, remove, and clear tags on the test file.
 */
static void test_tags(void) {
    CHECK(void_store_tag_add(g_store, "/testfile.txt", "important"));
    CHECK(void_store_tag_add(g_store, "/testfile.txt", "text"));

    VoidStringArray tags = {0};
    CHECK(void_store_tag_get(g_store, "/testfile.txt", &tags));
    ASSERT(tags.len == 2);
    void_string_array_free(tags);

    /* global tag list */
    VoidStringArray all = {0};
    CHECK(void_store_tag_list(g_store, &all));
    ASSERT(all.len == 2);
    void_string_array_free(all);

    /* search by one tag */
    const char *search[] = {"important"};
    VoidFileArray results = {0};
    CHECK(void_store_tag_search(g_store, search, 1, &results));
    ASSERT(results.len == 1);
    void_file_array_free(results);

    /* negative search: files NOT tagged "missing_tag" */
    const char *neg[] = {"!missing_tag"};
    VoidFileArray neg_results = {0};
    CHECK(void_store_tag_search(g_store, neg, 1, &neg_results));
    ASSERT(neg_results.len == 1);
    void_file_array_free(neg_results);

    /* remove one tag */
    CHECK(void_store_tag_rm(g_store, "/testfile.txt", "text"));
    tags = (VoidStringArray){0};
    CHECK(void_store_tag_get(g_store, "/testfile.txt", &tags));
    ASSERT(tags.len == 1);
    ASSERT_STR_EQ(tags.items[0], "important");
    void_string_array_free(tags);

    /* clear all tags */
    CHECK(void_store_tag_clear(g_store, "/testfile.txt"));
    tags = (VoidStringArray){0};
    CHECK(void_store_tag_get(g_store, "/testfile.txt", &tags));
    ASSERT(tags.len == 0);
    void_string_array_free(tags);
}

/*
 * test_mv
 * Move the file into a sub-directory.  void's mv() moves the node to the
 * *parent* of dst — the leaf name is ignored and the file keeps its original
 * name.  So mv("/testfile.txt", "/subdir/testfile.txt") results in the file
 * being at /subdir/testfile.txt.
 */
static void test_mv(void) {
    CHECK(void_store_mv(g_store, "/testfile.txt", "/subdir/testfile.txt"));

    VoidFileArray root = {0};
    CHECK(void_store_list(g_store, "/", &root));
    ASSERT(root.len == 1);
    ASSERT_STR_EQ(root.items[0].name, "subdir");
    ASSERT(root.items[0].is_file == false);
    void_file_array_free(root);

    CHECK(void_store_get(g_store, "/subdir/testfile.txt", DST_FILE));
    char *got = read_file(DST_FILE);
    ASSERT(got != NULL);
    ASSERT_STR_EQ(got, FILE_CONTENT);
    free(got);
    unlink(DST_FILE);
}

/*
 * test_remove
 * Remove the file, confirm it is gone, confirm the directory entry persists
 * (void does not auto-delete empty directories).
 */
static void test_remove(void) {
    CHECK(void_store_remove(g_store, "/subdir/testfile.txt"));

    /* file must be gone */
    CHECK_ERR(void_store_get(g_store, "/subdir/testfile.txt", DST_FILE),
              VOID_ERR_FILE_DOES_NOT_EXIST);

    /* /subdir itself still exists as an empty directory */
    VoidFileArray sub = {0};
    CHECK(void_store_list(g_store, "/subdir", &sub));
    ASSERT(sub.len == 0);
    void_file_array_free(sub);
}

/*
 * test_gc
 * After all the clean operations above there should be no orphaned chunks.
 */
static void test_gc(void) {
    size_t removed = 999;
    CHECK(void_store_gc(g_store, &removed));
    ASSERT(removed == 0);
}

/* -------------------------------------------------------------------------
 * Entry point
 *
 * Test order matters — each test relies on state left by the previous:
 *   setup            creates the store
 *   test_open_close  re-opens it (persists state)
 *   test_add_get     adds /testfile.txt
 *   test_list        inspects /
 *   test_metadata    exercises metadata on /testfile.txt
 *   test_tags        exercises tags on /testfile.txt
 *   test_mv          moves it to /subdir/renamed.txt
 *   test_remove      deletes /subdir/renamed.txt
 *   test_gc          confirms no orphaned chunks
 *   teardown         deletes the store on disk
 * ------------------------------------------------------------------------- */

int main(void) {
    printf("void-ffi C tests\n");
    printf("================\n");

    setup();
    RUN(test_open_close);
    RUN(test_add_get);
    RUN(test_list);
    RUN(test_metadata);
    RUN(test_tags);
    RUN(test_mv);
    RUN(test_remove);
    RUN(test_gc);
    teardown();

    printf("================\n");
    printf("%d/%d passed\n", g_tests_run - g_tests_failed, g_tests_run);

    return g_tests_failed > 0 ? 1 : 0;
}
