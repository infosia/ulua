/* Build with the same flags as the gate test.
   macOS:
   cc -std=c11 -Wall -Wextra -Wpedantic -Werror <host.c> -I <include-dir> <libulua.a> -o <host> -lc++ -liconv -framework Security -framework CoreFoundation
   Linux:
   cc -std=c11 -Wall -Wextra -Wpedantic -Werror <host.c> -I <include-dir> <libulua.a> -o <host> -lstdc++ -lpthread -ldl -lm
   Windows (MSVC, Developer Command Prompt):
   cl /nologo /W4 /WX /MD /D_CRT_SECURE_NO_WARNINGS /TC /std:c11 <host.c> /I <include-dir> /Fe:<host.exe> /link <ulua.lib> advapi32.lib bcrypt.lib ntdll.lib userenv.lib ws2_32.lib
*/
/* A general runner. It compiles any script with no grants, calls its export `main`,
   and prints each line that the script prints. The TypeScript tutorial uses it.
   The exit status gives the result. It is 0 for an end without a stop, also when the
   script has no `main`. It is 1 for a rejected script or a host failure. It is 2 for
   a stop of the script. */
#include "ulua.h"
#include <stdio.h>
#include <stdlib.h>

/* RUN goes to `stopped` at the first status other than ULUA_OK. That label prints the
   diagnostic of the stop. A failed free changes only a successful exit status, so a
   stop keeps the status 2. */
#define RUN(call) do { \
    status = (call); \
    if (status != ULUA_OK) goto stopped; \
} while (0)
#define RELEASE(call) do { \
    if ((call) != ULUA_OK && exit_code == 0) exit_code = 1; \
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
   failed, and the run stops with ULUA_HOST_ERROR. */
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
        /* The public compile API returns one diagnostic per rejection. For a rejected
           script, the status is ULUA_REJECTED, and the diagnostic gives the code, the
           line, and the column of the cause. reject.ts shows this output. */
        printf("error %s at %u:%u: %s\n", diagnostic.code,
               (unsigned)diagnostic.line, (unsigned)diagnostic.column, diagnostic.message);
        goto cleanup;
    }
    /* NULL selects the default run limits. Module initializers run here. A trap in them
       stops this call, and no instance exists after it. init-trap.ts shows this case. */
    RUN(ulua_instance_new(context, module, &host, NULL, &instance, &diagnostic));
    status = ulua_module_find_export(module, "main", 4, &main_index);
    /* An unknown name gives ULUA_INVALID_ARGUMENT and writes UINT32_MAX to the index.
       The runner checks both, so a module without `main` ends with status 0. */
    if (status == ULUA_INVALID_ARGUMENT && main_index == UINT32_MAX) {
        exit_code = 0;
        goto cleanup;
    }
    if (status != ULUA_OK) goto stopped;
    /* Call tasks support both sync and async exports. This runner grants no capabilities.
       The runner does not know whether `main` is async, so it starts `main` as a task.
       A sync export gives its result at the first query of the task. */
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
    /* Each stop comes here: a trap, a limit, or a host error. The diagnostic code names
       the cause, for example trap:index-out-of-range. trap.ts shows this output. */
stopped:
    if (diagnostic.code[0] != '\0') {
        printf("stopped %s: %s\n", diagnostic.code, diagnostic.message);
    } else {
        printf("stopped abi:status: status %d\n", (int)status);
    }
    exit_code = 2;
cleanup:
    /* Instance free also releases all remaining requests and tasks. Each free function
       accepts NULL, so this code is correct after a failure at any step. The context goes
       last: its free invalidates every object that it made. */
    RELEASE(ulua_instance_free(instance));
    RELEASE(ulua_module_free(module));
    RELEASE(ulua_context_free(context));
    free(source);
    return ferror(stdout) ? 1 : exit_code;
}
