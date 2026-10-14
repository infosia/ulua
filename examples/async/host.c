/* Build with the same flags as the gate test.
   macOS:
   cc -std=c11 -Wall -Wextra -Wpedantic -Werror <host.c> -I <include-dir> <libulua.a> -o <host> -lc++ -liconv -framework Security -framework CoreFoundation
   Linux:
   cc -std=c11 -Wall -Wextra -Wpedantic -Werror <host.c> -I <include-dir> <libulua.a> -o <host> -lstdc++ -lpthread -ldl -lm
*/
#include "ulua.h"
#include <inttypes.h>
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

typedef struct pending_request {
    uint64_t request;
    double id;
} pending_request;

typedef struct request_queue {
    pending_request requests[3];
    size_t count;
} request_queue;

static int32_t fetch_score(void* userdata, const ulua_value* arguments,
                           size_t count, uint64_t request) {
    request_queue* queue = userdata;
    if (count != 1 || arguments[0].type != ULUA_TYPE_NUMBER || queue->count == 3) return 1;
    queue->requests[queue->count++] = (pending_request){request, arguments[0].number};
    printf("start fetchScore(%.0f): request %" PRIu64 "\n", arguments[0].number, request);
    return ferror(stdout) ? 1 : 0;
}

int main(int argc, char** argv) {
    int exit_code = 1;
    char* source = NULL;
    size_t source_length = 0;
    ulua_context* context = NULL;
    ulua_module* module = NULL;
    ulua_instance* instance = NULL;
    ulua_capability* capability = NULL;
    ulua_diagnostic diagnostic = {.struct_size = sizeof(diagnostic)};
    ulua_host host = {.struct_size = sizeof(host), .output = output};
    request_queue queue = {0};
    uint32_t number_type = ULUA_TYPE_NUMBER;
    ulua_capability_desc desc = {
        .struct_size = sizeof(desc), .name = "fetchScore", .name_length = 10,
        .parameter_types = &number_type, .parameter_count = 1,
        .result_type = ULUA_TYPE_NUMBER, .start = fetch_score, .userdata = &queue
    };
    ulua_value result = {0};
    uint32_t report_index = 0, open = 0, recorded = 0, settled = 0;
    uint64_t task = 0;
    ulua_status status;

    if (argc != 2) {
        fprintf(stderr, "usage: %s scores.ts\n", argv[0]);
        goto cleanup;
    }
    source = read_file(argv[1], &source_length);
    REQUIRE(source != NULL);
    CHECK(ulua_context_new(&context));
    puts("create async fetchScore capability");
    CHECK(ulua_capability_new(context, &desc, &capability));
    puts("compile scores with grant");
    CHECK(ulua_compile(context, source, source_length, &capability, 1, NULL, &module, &diagnostic));
    puts("create instance with output callback");
    CHECK(ulua_instance_new(context, module, &host, NULL, &instance, &diagnostic));
    CHECK(ulua_module_find_export(module, "report", 6, &report_index));
    CHECK(ulua_instance_requests(instance, &open, &recorded));
    REQUIRE(open == 0 && recorded == 0);
    printf("requests before call: open=%u recorded=%u\n", (unsigned)open, (unsigned)recorded);
    puts("call_start report");
    CHECK(ulua_call_start(instance, report_index, NULL, 0, &task, &diagnostic));
    printf("task: %" PRIu64 "\n", task);
    status = ulua_task_result(instance, task, &result, &diagnostic);
    REQUIRE(status == ULUA_PENDING);
    puts("task_result: ULUA_PENDING");
    CHECK(ulua_instance_requests(instance, &open, &recorded));
    REQUIRE(queue.count == 3 && open == 3 && recorded == 0);
    printf("requests after call: open=%u recorded=%u\n", (unsigned)open, (unsigned)recorded);
    /* The host completes requests outside the start callback. No script runs here. */
    for (size_t i = 0; i < queue.count; ++i) {
        pending_request pending = queue.requests[i];
        ulua_value score = {.type = ULUA_TYPE_NUMBER, .number = pending.id * 10};
        CHECK(ulua_request_complete(instance, pending.request, &score));
        printf("complete request %" PRIu64 ": score=%.0f\n", pending.request, score.number);
    }
    CHECK(ulua_instance_requests(instance, &open, &recorded));
    REQUIRE(open == 3 && recorded == 3);
    printf("requests before pump: open=%u recorded=%u\n", (unsigned)open, (unsigned)recorded);
    puts("pump");
    CHECK(ulua_pump(instance, 0, &settled, &diagnostic));
    REQUIRE(settled == 3);
    printf("pump settled: %u\n", (unsigned)settled);
    CHECK(ulua_instance_requests(instance, &open, &recorded));
    REQUIRE(open == 0 && recorded == 0);
    printf("requests after pump: open=%u recorded=%u\n", (unsigned)open, (unsigned)recorded);
    CHECK(ulua_task_result(instance, task, &result, &diagnostic));
    REQUIRE(result.type == ULUA_TYPE_NUMBER);
    printf("task_result: ULUA_OK, total=%.0f\n", result.number);
    puts("free resources");
    exit_code = 0;
cleanup:
    RELEASE(ulua_instance_free(instance));
    RELEASE(ulua_module_free(module));
    RELEASE(ulua_capability_free(capability));
    RELEASE(ulua_context_free(context));
    free(source);
    return ferror(stdout) ? 1 : exit_code;
}
