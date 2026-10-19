/* Build with the same flags as the gate test.
   macOS:
   cc -std=c11 -Wall -Wextra -Wpedantic -Werror <host.c> -I <include-dir> <libulua.a> -o <host> -lc++ -liconv -framework Security -framework CoreFoundation
   Linux:
   cc -std=c11 -Wall -Wextra -Wpedantic -Werror <host.c> -I <include-dir> <libulua.a> -o <host> -lstdc++ -lpthread -ldl -lm
*/
/* A host that sets run limits. Ulua treats each script as hostile, so the host bounds
   the steps and the memory of an instance. Three exports of limits.ts stop in three
   ways, and the instance stays usable after each stop. */
#include "ulua.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* Each example stands alone. No private runtime header is required.
   Every function of ulua.h returns a status, and ULUA_OK means success. CHECK stops at
   the first other status and prints the diagnostic that the library wrote. REQUIRE
   checks a fact of this program. RELEASE turns a failed free into exit code 1. */
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

/* The library takes the source as bytes and a length, not as a C string. The host
   reads the whole file into memory. The terminator is for C code only. */
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
    /* Each struct that the host gives to the library starts with `struct_size`. The
       library compares it with the size that it expects, and a different value gives
       ULUA_INVALID_ARGUMENT. The other fields start at zero, as the reserved fields
       must. */
    ulua_diagnostic diagnostic = {.struct_size = sizeof(diagnostic)};
    /* This script does not print, so the host gives no output function. */
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
    /* Compile with no grants and the default compile limits, as in the hello example. */
    puts("compile limits");
    CHECK(ulua_context_new(&context));
    CHECK(ulua_compile(context, source, source_length, NULL, 0, NULL, &module, &diagnostic));
    /* Set the run limits. ulua_default_run_limits fills every field with its default,
       so the host changes only the limits that it needs. `max_steps` bounds each run
       that the host starts, and each loop charges steps. `max_memory_bytes`
       bounds the heap of the instance. */
    CHECK(ulua_default_run_limits(&limits));
    limits.max_steps = 10000;
    limits.max_memory_bytes = 1024 * 1024;
    puts("create instance: max_steps=10000 max_memory_bytes=1048576");
    /* The limits belong to the instance. Each call on this instance runs under them. */
    CHECK(ulua_instance_new(context, module, &host, &limits, &instance, &diagnostic));
    /* All calls use the same instance, including the call after each stop.
       A stop does not end the host. The call gives ULUA_FAILED, and the diagnostic code
       names the cause: a limit or a trap. The instance keeps the state that the run
       reached, and the next call on it is valid. No `catch` and no `finally` of the
       script runs at a stop. */
    for (size_t i = 0; i < sizeof(names) / sizeof(names[0]); ++i) {
        CHECK(ulua_module_find_export(module, names[i], strlen(names[i]), &index));
        ulua_status status = ulua_call(instance, index, NULL, 0, &result, &diagnostic);
        REQUIRE(status == ULUA_FAILED && strcmp(diagnostic.code, codes[i]) == 0);
        printf("%s: status=%d (ULUA_FAILED), code=%s\n", names[i], (int)status, diagnostic.code);
    }
    /* After the three stops, `ok` still runs and returns the length of the array. */
    CHECK(ulua_module_find_export(module, "ok", 2, &index));
    CHECK(ulua_call(instance, index, NULL, 0, &result, &diagnostic));
    REQUIRE(result.type == ULUA_TYPE_NUMBER && result.number == 3);
    puts("ok: status=0 (ULUA_OK), result=3");
    puts("free resources");
    exit_code = 0;
    /* Clean up. Each free function accepts NULL, so this code is correct after a failure
       at any step. The context goes last: its free invalidates every object that it
       made. */
cleanup:
    RELEASE(ulua_instance_free(instance));
    RELEASE(ulua_module_free(module));
    RELEASE(ulua_context_free(context));
    free(source);
    return ferror(stdout) ? 1 : exit_code;
}
