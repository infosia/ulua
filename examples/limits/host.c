/* Build with the same flags as the gate test.
   macOS:
   cc -std=c11 -Wall -Wextra -Wpedantic -Werror <host.c> -I <include-dir> <libulua.a> -o <host> -lc++ -liconv -framework Security -framework CoreFoundation
   Linux:
   cc -std=c11 -Wall -Wextra -Wpedantic -Werror <host.c> -I <include-dir> <libulua.a> -o <host> -lstdc++ -lpthread -ldl -lm
*/
#include "ulua.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* Each example stands alone. No private runtime header is required. */
#define CHECK(call) do { \
    ulua_status status_ = (call); \
    if (status_ != ULUA_OK) { \
        fprintf(stderr, "%s: status %d, %s at %u:%u: %s\n", #call, \
                (int)status_, diagnostic.code, (unsigned)diagnostic.line, \
                (unsigned)diagnostic.column, diagnostic.message); \
        goto cleanup; \
    } \
} while (0)
#define REQUIRE(condition) do { \
    if (!(condition)) { \
        fprintf(stderr, "check failed: %s\n", #condition); \
        goto cleanup; \
    } \
} while (0)
#define RELEASE(call) do { \
    if ((call) != ULUA_OK) exit_code = 1; \
} while (0)

static char* read_file(const char* path, size_t* length) {
    FILE* file = fopen(path, "rb");
    char* bytes = NULL;
    long size;
    if (file == NULL) {
        perror(path);
        return NULL;
    }
    if (fseek(file, 0, SEEK_END) != 0 || (size = ftell(file)) < 0 ||
        fseek(file, 0, SEEK_SET) != 0) goto done;
    if ((uintmax_t)size >= SIZE_MAX) goto done;
    bytes = malloc((size_t)size + 1);
    if (bytes == NULL) goto done;
    if (fread(bytes, 1, (size_t)size, file) != (size_t)size) {
        free(bytes);
        bytes = NULL;
        goto done;
    }
    bytes[size] = '\0';
    *length = (size_t)size;
done:
    if (fclose(file) != 0) {
        free(bytes);
        bytes = NULL;
    }
    if (bytes == NULL) fprintf(stderr, "cannot read %s\n", path);
    return bytes;
}

int main(int argc, char** argv) {
    int exit_code = 1;
    char* source = NULL;
    size_t source_length = 0;
    ulua_context* context = NULL;
    ulua_module* module = NULL;
    ulua_instance* instance = NULL;
    ulua_diagnostic diagnostic = {.struct_size = sizeof(diagnostic)};
    ulua_host host = {.struct_size = sizeof(host)};
    ulua_run_limits limits = {.struct_size = sizeof(limits)};
    ulua_value result = {0};
    const char* names[] = {"spin", "grow", "outOfRange"};
    const char* codes[] = {"limit:steps", "limit:memory", "trap:index-out-of-range"};
    uint32_t index = 0;

    if (argc != 2) {
        fprintf(stderr, "usage: %s limits.ts\n", argv[0]);
        goto cleanup;
    }
    source = read_file(argv[1], &source_length);
    REQUIRE(source != NULL);
    puts("compile limits");
    CHECK(ulua_context_new(&context));
    CHECK(ulua_compile(context, source, source_length, NULL, 0, NULL, &module, &diagnostic));
    CHECK(ulua_default_run_limits(&limits));
    limits.max_steps = 10000;
    limits.max_memory_bytes = 1024 * 1024;
    puts("create instance: max_steps=10000 max_memory_bytes=1048576");
    CHECK(ulua_instance_new(context, module, &host, &limits, &instance, &diagnostic));
    /* All calls use the same instance, including the call after each stop. */
    for (size_t i = 0; i < sizeof(names) / sizeof(names[0]); ++i) {
        CHECK(ulua_module_find_export(module, names[i], strlen(names[i]), &index));
        ulua_status status = ulua_call(instance, index, NULL, 0, &result, &diagnostic);
        REQUIRE(status == ULUA_FAILED && strcmp(diagnostic.code, codes[i]) == 0);
        printf("%s: status=%d (ULUA_FAILED), code=%s\n", names[i], (int)status, diagnostic.code);
    }
    CHECK(ulua_module_find_export(module, "ok", 2, &index));
    CHECK(ulua_call(instance, index, NULL, 0, &result, &diagnostic));
    REQUIRE(result.type == ULUA_TYPE_NUMBER && result.number == 3);
    puts("ok: status=0 (ULUA_OK), result=3");
    puts("free resources");
    exit_code = 0;
cleanup:
    RELEASE(ulua_instance_free(instance));
    RELEASE(ulua_module_free(module));
    RELEASE(ulua_context_free(context));
    free(source);
    return ferror(stdout) ? 1 : exit_code;
}
