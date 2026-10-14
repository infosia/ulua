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

static int32_t output(void* userdata, const char* text, size_t length) {
    (void)userdata;
    return fwrite(text, 1, length, stdout) == length && putchar('\n') != EOF ? 0 : 1;
}

int main(int argc, char** argv) {
    int exit_code = 1;
    char* source = NULL;
    size_t source_length = 0;
    ulua_context* context = NULL;
    ulua_module* module = NULL;
    ulua_instance* instance = NULL;
    ulua_diagnostic diagnostic = {.struct_size = sizeof(diagnostic)};
    ulua_host host = {.struct_size = sizeof(host), .output = output};
    ulua_value result = {0};
    ulua_value argument = {.type = ULUA_TYPE_STRING, .data = "world", .length = 5};
    uint32_t main_index = 0, greet_index = 0;

    if (argc != 2) {
        fprintf(stderr, "usage: %s hello.ts\n", argv[0]);
        goto cleanup;
    }
    source = read_file(argv[1], &source_length);
    REQUIRE(source != NULL);
    puts("compile hello");
    CHECK(ulua_context_new(&context));
    CHECK(ulua_compile(context, source, source_length, NULL, 0, NULL, &module, &diagnostic));
    puts("create instance with output callback");
    CHECK(ulua_instance_new(context, module, &host, NULL, &instance, &diagnostic));
    CHECK(ulua_module_find_export(module, "main", 4, &main_index));
    CHECK(ulua_module_find_export(module, "greet", 5, &greet_index));
    puts("call main");
    CHECK(ulua_call(instance, main_index, NULL, 0, &result, &diagnostic));
    REQUIRE(result.type == ULUA_TYPE_VOID);
    puts("call greet(world)");
    CHECK(ulua_call(instance, greet_index, &argument, 1, &result, &diagnostic));
    REQUIRE(result.type == ULUA_TYPE_STRING);
    /* Consume the borrowed string before the next API call on this context. */
    fputs("greet result: ", stdout);
    REQUIRE(fwrite(result.data, 1, result.length, stdout) == result.length);
    REQUIRE(putchar('\n') != EOF);
    puts("free resources");
    exit_code = 0;
cleanup:
    RELEASE(ulua_instance_free(instance));
    RELEASE(ulua_module_free(module));
    RELEASE(ulua_context_free(context));
    free(source);
    return ferror(stdout) ? 1 : exit_code;
}
