/* Build with the same flags as the gate test.
   macOS:
   cc -std=c11 -Wall -Wextra -Wpedantic -Werror <host.c> -I <include-dir> <libulua.a> -o <host> -lc++ -liconv -framework Security -framework CoreFoundation
   Linux:
   cc -std=c11 -Wall -Wextra -Wpedantic -Werror <host.c> -I <include-dir> <libulua.a> -o <host> -lstdc++ -lpthread -ldl -lm
*/
#include "ulua.h"
#include <stdio.h>
#include <stdlib.h>

#define RUN(call) do { \
    status = (call); \
    if (status != ULUA_OK) goto stopped; \
} while (0)
#define RELEASE(call) do { \
    if ((call) != ULUA_OK && exit_code == 0) exit_code = 1; \
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
    uint32_t main_index = UINT32_MAX, settled = 0;
    uint64_t task = 0;
    ulua_status status;

    if (argc != 2) {
        fputs("usage: run <script.ts>\n", stderr);
        goto cleanup;
    }
    source = read_file(argv[1], &source_length);
    if (source == NULL) goto cleanup;
    RUN(ulua_context_new(&context));
    status = ulua_compile(context, source, source_length, NULL, 0, NULL, &module, &diagnostic);
    if (status != ULUA_OK) {
        /* The public compile API returns one diagnostic per rejection. */
        printf("error %s at %u:%u: %s\n", diagnostic.code,
               (unsigned)diagnostic.line, (unsigned)diagnostic.column, diagnostic.message);
        goto cleanup;
    }
    /* NULL selects the default run limits. Module initializers run here. */
    RUN(ulua_instance_new(context, module, &host, NULL, &instance, &diagnostic));
    status = ulua_module_find_export(module, "main", 4, &main_index);
    if (status == ULUA_INVALID_ARGUMENT && main_index == UINT32_MAX) {
        exit_code = 0;
        goto cleanup;
    }
    if (status != ULUA_OK) goto stopped;
    /* Call tasks support both sync and async exports. This runner grants no capabilities. */
    RUN(ulua_call_start(instance, main_index, NULL, 0, &task, &diagnostic));
    RUN(ulua_pump(instance, 0, &settled, &diagnostic));
    status = ulua_task_result(instance, task, &result, &diagnostic);
    if (status == ULUA_PENDING) {
        /* No host requests can complete a pending task because this runner has no grants. */
        puts("stopped async:pending: The main task is still pending");
        exit_code = 2;
        goto cleanup;
    }
    if (status != ULUA_OK) goto stopped;
    exit_code = 0;
    goto cleanup;
stopped:
    if (diagnostic.code[0] != '\0') {
        printf("stopped %s: %s\n", diagnostic.code, diagnostic.message);
    } else {
        printf("stopped abi:status: status %d\n", (int)status);
    }
    exit_code = 2;
cleanup:
    /* Instance free also releases all remaining requests and tasks. */
    RELEASE(ulua_instance_free(instance));
    RELEASE(ulua_module_free(module));
    RELEASE(ulua_context_free(context));
    free(source);
    return ferror(stdout) ? 1 : exit_code;
}
