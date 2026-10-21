/* Build with the same flags as the gate test.
   macOS:
   cc -std=c11 -Wall -Wextra -Wpedantic -Werror <host.c> -I <include-dir> <libulua.a> -o <host> -lc++ -liconv -framework Security -framework CoreFoundation
   Linux:
   cc -std=c11 -Wall -Wextra -Wpedantic -Werror <host.c> -I <include-dir> <libulua.a> -o <host> -lstdc++ -lpthread -ldl -lm
   Windows (MSVC, Developer Command Prompt):
   cl /nologo /W4 /WX /MD /D_CRT_SECURE_NO_WARNINGS /TC /std:c11 <host.c> /I <include-dir> /Fe:<host.exe> /link <ulua.lib> advapi32.lib bcrypt.lib ntdll.lib userenv.lib ws2_32.lib
*/
/* The smallest complete Ulua host. It compiles hello.ts, makes a running instance of it,
   and calls the two exports `main` and `greet` by name. Each phase of `main` below has
   a comment that explains what the step does and why the host needs it. */
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

/* The script calls `print`, and the library calls this function with the text of one
   line. The text has no line ending, so the host adds one. The bytes are valid only
   until this function returns. A non-zero return tells the library that the output
   failed, and the run stops with ULUA_HOST_ERROR. `userdata` is the pointer that the
   host puts in `ulua_host`; this host does not use it. */
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
    /* Each struct that the host gives to the library starts with `struct_size`. The
       library compares it with the size that it expects, and a different value gives
       ULUA_INVALID_ARGUMENT. The other fields start at zero, as the reserved fields
       must. */
    ulua_diagnostic diagnostic = {.struct_size = sizeof(diagnostic)};
    /* `host` connects the instance to the host: here, `print` goes to `output`. */
    ulua_host host = {.struct_size = sizeof(host), .output = output};
    ulua_value result = {0};
    /* A string crosses the C ABI as UTF-8 bytes and a length, with no terminator.
       The library reads exactly `length` bytes, so "world" has the length 5. */
    ulua_value argument = {.type = ULUA_TYPE_STRING, .data = "world", .length = 5};
    uint32_t main_index = 0, greet_index = 0;

    if (argc != 2) {
        fprintf(stderr, "usage: %s hello.ts\n", argv[0]);
        goto cleanup;
    }
    source = read_file(argv[1], &source_length);
    REQUIRE(source != NULL);
    /* Compile. A context owns every other Ulua object, and one thread at a time uses it.
       ulua_compile checks the script and turns it into a module. This script calls no
       host function, so the list of grants is empty. NULL selects the default compile
       limits. A script that the compiler rejects gives ULUA_REJECTED, and the diagnostic
       names the cause, the line, and the column. */
    puts("compile hello");
    CHECK(ulua_context_new(&context));
    CHECK(ulua_compile(context, source, source_length, NULL, 0, NULL, &module, &diagnostic));
    /* Make an instance. A module is the compiled script, and an instance is one running
       copy of it, with its own heap and its own limits. ulua_instance_new runs the
       module-level statements of the script. NULL selects the default run limits. */
    puts("create instance with output callback");
    CHECK(ulua_instance_new(context, module, &host, NULL, &instance, &diagnostic));
    /* Find the exports. The host calls a script function by its index in the module.
       The host looks up each name once and keeps the index for every later call. */
    CHECK(ulua_module_find_export(module, "main", 4, &main_index));
    CHECK(ulua_module_find_export(module, "greet", 5, &greet_index));
    /* Call an export. The host gives the arguments as an array of `ulua_value` and gets
       the result in one more `ulua_value`. `main` takes no arguments and returns void.
       Each line that `main` prints reaches `output` during this call. */
    puts("call main");
    CHECK(ulua_call(instance, main_index, NULL, 0, &result, &diagnostic));
    REQUIRE(result.type == ULUA_TYPE_VOID);
    /* `greet` takes one string and returns one string. The `type` field of the result
       tells the host which other field holds the value. */
    puts("call greet(world)");
    CHECK(ulua_call(instance, greet_index, &argument, 1, &result, &diagnostic));
    REQUIRE(result.type == ULUA_TYPE_STRING);
    /* The result string belongs to the instance, not to the host. It stays valid until
       the next call on this context, so the host writes it out first. It has no
       terminator, so the host writes it with its length. */
    fputs("greet result: ", stdout);
    REQUIRE(fwrite(result.data, 1, result.length, stdout) == result.length);
    REQUIRE(putchar('\n') != EOF);
    puts("free resources");
    exit_code = 0;
    /* Clean up. Each free function accepts NULL, so this code is correct after a failure
       at any step. The context goes last: its free invalidates every object that it
       made, so a later free of the instance or the module gets a dead pointer. */
cleanup:
    RELEASE(ulua_instance_free(instance));
    RELEASE(ulua_module_free(module));
    RELEASE(ulua_context_free(context));
    free(source);
    return ferror(stdout) ? 1 : exit_code;
}
