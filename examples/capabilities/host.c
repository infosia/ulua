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

static int32_t damage(void* userdata, const ulua_value* arguments,
                      size_t count, ulua_value* result) {
    (void)userdata;
    if (count != 1 || arguments[0].type != ULUA_TYPE_NUMBER) return 1;
    result->type = ULUA_TYPE_NUMBER;
    result->number = arguments[0].number * 2;
    return 0;
}

static int32_t log_text(void* userdata, const ulua_value* arguments,
                        size_t count, ulua_value* result) {
    (void)userdata;
    if (count != 1 || arguments[0].type != ULUA_TYPE_STRING) return 1;
    result->type = ULUA_TYPE_VOID;
    fputs("host log: ", stdout);
    return fwrite(arguments[0].data, 1, arguments[0].length, stdout) == arguments[0].length
        && putchar('\n') != EOF ? 0 : 1;
}

int main(int argc, char** argv) {
    int exit_code = 1;
    char* source = NULL;
    char* declaration_path = NULL;
    char* expected = NULL;
    size_t source_length = 0, expected_length = 0, declaration_length = 0;
    char declarations[256];
    ulua_context* context = NULL;
    ulua_module* module = NULL;
    ulua_instance* instance = NULL;
    ulua_capability* grants[2] = {NULL, NULL};
    ulua_diagnostic diagnostic = {.struct_size = sizeof(diagnostic)};
    ulua_host host = {.struct_size = sizeof(host)};
    uint32_t number_type = ULUA_TYPE_NUMBER, string_type = ULUA_TYPE_STRING;
    ulua_capability_desc damage_desc = {
        .struct_size = sizeof(damage_desc), .name = "damage", .name_length = 6,
        .parameter_types = &number_type, .parameter_count = 1,
        .result_type = ULUA_TYPE_NUMBER, .function = damage
    };
    ulua_capability_desc log_desc = {
        .struct_size = sizeof(log_desc), .name = "log", .name_length = 3,
        .parameter_types = &string_type, .parameter_count = 1,
        .result_type = ULUA_TYPE_VOID, .function = log_text
    };
    ulua_value argument = {.type = ULUA_TYPE_NUMBER, .number = 10};
    ulua_value result = {0};
    uint32_t hit_index = 0, heal_index = 0;
    ulua_status status;

    if (argc != 2) {
        fprintf(stderr, "usage: %s game.ts\n", argv[0]);
        goto cleanup;
    }
    source = read_file(argv[1], &source_length);
    REQUIRE(source != NULL);
    /* The declaration file is beside the script, with the suffix .d.ts. */
    size_t path_length = strlen(argv[1]);
    REQUIRE(path_length >= 3 && strcmp(argv[1] + path_length - 3, ".ts") == 0);
    declaration_path = malloc(path_length + 3);
    REQUIRE(declaration_path != NULL);
    memcpy(declaration_path, argv[1], path_length - 3);
    strcpy(declaration_path + path_length - 3, ".d.ts");
    expected = read_file(declaration_path, &expected_length);
    REQUIRE(expected != NULL);
    CHECK(ulua_context_new(&context));
    puts("create damage and log capabilities");
    CHECK(ulua_capability_new(context, &damage_desc, &grants[0]));
    CHECK(ulua_capability_new(context, &log_desc, &grants[1]));
    CHECK(ulua_declarations(grants, 2, declarations, sizeof(declarations), &declaration_length));
    puts("write declarations");
    REQUIRE(fwrite(declarations, 1, declaration_length, stdout) == declaration_length);
    REQUIRE(declaration_length == expected_length);
    REQUIRE(memcmp(declarations, expected, expected_length) == 0);
    puts("declarations match");
    puts("compile game with grants");
    CHECK(ulua_compile(context, source, source_length, grants, 2, NULL, &module, &diagnostic));
    puts("create instance");
    CHECK(ulua_instance_new(context, module, &host, NULL, &instance, &diagnostic));
    CHECK(ulua_module_find_export(module, "hit", 3, &hit_index));
    CHECK(ulua_module_find_export(module, "heal", 4, &heal_index));
    for (int i = 0; i < 2; ++i) {
        puts("call hit(10)");
        CHECK(ulua_call(instance, hit_index, &argument, 1, &result, &diagnostic));
        REQUIRE(result.type == ULUA_TYPE_NUMBER);
        printf("hit result: %.0f\n", result.number);
    }
    puts("call heal");
    CHECK(ulua_call(instance, heal_index, NULL, 0, &result, &diagnostic));
    REQUIRE(result.type == ULUA_TYPE_BOOLEAN);
    printf("heal result: %s\n", result.boolean ? "true" : "false");
    puts("revoke damage");
    CHECK(ulua_capability_revoke(grants[0]));
    puts("call hit(10)");
    status = ulua_call(instance, hit_index, &argument, 1, &result, &diagnostic);
    REQUIRE(status == ULUA_HOST_ERROR && strcmp(diagnostic.code, "host:revoked") == 0);
    printf("hit status: ULUA_HOST_ERROR, code: %s\n", diagnostic.code);
    puts("free resources");
    exit_code = 0;
cleanup:
    RELEASE(ulua_instance_free(instance));
    RELEASE(ulua_module_free(module));
    RELEASE(ulua_capability_free(grants[0]));
    RELEASE(ulua_capability_free(grants[1]));
    RELEASE(ulua_context_free(context));
    free(expected);
    free(declaration_path);
    free(source);
    return ferror(stdout) ? 1 : exit_code;
}
