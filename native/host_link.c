/* C11 and C++17 use this source to check all public ABI functions. */
#include "ulua.h"
#include <assert.h>
#include <string.h>
static int32_t output(void* data, const char* text, size_t length) {
    (void)data;
    assert(length == 2 && memcmp(text, "ok", 2) == 0);
    return 0;
}
static int32_t host(void* data, const ulua_value* arguments, size_t count, ulua_value* result) {
    const uint32_t type = *(const uint32_t*)data;
    assert(count == 3 && arguments[0].number == 2.5 && arguments[1].boolean == 1);
    assert(arguments[2].length == 2 && memcmp(arguments[2].data, "ok", 2) == 0);
    assert(result->type == ULUA_TYPE_VOID && result->number == 0 && result->data == NULL);
    result->type = type;
    if (type == ULUA_TYPE_NUMBER) result->number = 7.5;
    if (type == ULUA_TYPE_BOOLEAN) result->boolean = 1;
    if (type == ULUA_TYPE_STRING) { result->data = "ok"; result->length = 2; }
    return 0;
}
int main(void) {
    ulua_context* context = NULL;
    ulua_module* module = NULL;
    ulua_instance* instance = NULL;
    ulua_capability* grants[4] = {NULL, NULL, NULL, NULL};
    ulua_compile_limits compile_limits;
    ulua_run_limits run_limits;
    ulua_diagnostic diagnostic;
    ulua_host outputs;
    ulua_value value;
    uint32_t types[4] = {0, 1, 2, 3};
    uint32_t parameters[3] = {1, 2, 3};
    const char* names[4] = {"hostVoid", "hostNumber", "hostBoolean", "hostString"};
    const char* source =
        "export function main():void{hostVoid(2.5,true,'ok');"
        "if(hostNumber(2.5,true,'ok')===7.5&&hostBoolean(2.5,true,'ok'))"
        "{print(hostString(2.5,true,'ok'));}}"
        "export function echo(s:string):string{return s;}";
    uint32_t count = 0, index = 0, signature[1] = {0}, result_type = 0;
    size_t length = 0, parameter_count = 0;
    const char* name = NULL;
    memset(&compile_limits, 0, sizeof(compile_limits));
    memset(&run_limits, 0, sizeof(run_limits));
    memset(&diagnostic, 0, sizeof(diagnostic));
    memset(&outputs, 0, sizeof(outputs));
    compile_limits.struct_size = sizeof(compile_limits);
    run_limits.struct_size = sizeof(run_limits);
    diagnostic.struct_size = sizeof(diagnostic);
    outputs.struct_size = sizeof(outputs);
    outputs.output = output;
    assert(ulua_abi_version() == 1);
    assert(ulua_default_compile_limits(&compile_limits) == ULUA_OK);
    assert(ulua_default_run_limits(&run_limits) == ULUA_OK);
    assert(ulua_context_new(&context) == ULUA_OK);
    for (size_t i = 0; i < 4; ++i) {
        ulua_capability_desc desc;
        memset(&desc, 0, sizeof(desc));
        desc.struct_size = sizeof(desc);
        desc.name = names[i]; desc.name_length = strlen(names[i]);
        desc.parameter_types = parameters; desc.parameter_count = 3;
        desc.result_type = types[i]; desc.function = host; desc.userdata = &types[i];
        assert(ulua_capability_new(context, &desc, &grants[i]) == ULUA_OK);
    }
    {
        char declarations[512]; size_t declaration_length = 0;
        assert(ulua_declarations(grants, 4, declarations, sizeof(declarations), &declaration_length) == ULUA_OK);
        assert(declaration_length > 0);
    }
    assert(ulua_compile(context, source, strlen(source), grants, 4, &compile_limits, &module, &diagnostic) == ULUA_OK);
    assert(ulua_module_export_count(module, &count) == ULUA_OK && count == 2);
    assert(ulua_module_export_name(module, 0, &name, &length) == ULUA_OK && length == 4 && memcmp(name,"main",4) == 0);
    assert(ulua_module_export_signature(module, 1, signature, 1, &parameter_count, &result_type) == ULUA_OK);
    assert(parameter_count == 1 && signature[0] == ULUA_TYPE_STRING && result_type == ULUA_TYPE_STRING);
    assert(ulua_module_find_export(module, "main", 4, &index) == ULUA_OK && index == 0);
    assert(ulua_instance_new(context, module, &outputs, &run_limits, &instance, &diagnostic) == ULUA_OK);
    memset(&value, 0, sizeof(value));
    assert(ulua_call(instance, index, NULL, 0, &value, &diagnostic) == ULUA_OK && value.type == ULUA_TYPE_VOID);
    value.type = ULUA_TYPE_STRING; value.data = "ok"; value.length = 2;
    assert(ulua_call(instance, 1, &value, 1, &value, &diagnostic) == ULUA_OK);
    assert(value.type == ULUA_TYPE_STRING && value.length == 2 && memcmp(value.data,"ok",2) == 0);
    assert(ulua_capability_revoke(grants[0]) == ULUA_OK);
    for (size_t i = 0; i < 4; ++i) assert(ulua_capability_free(grants[i]) == ULUA_OK);
    assert(ulua_call(instance, 0, NULL, 0, &value, &diagnostic) == ULUA_HOST_ERROR);
    assert(strcmp(diagnostic.code,"host:revoked") == 0 && strcmp(diagnostic.message,"hostVoid") == 0);
    assert(ulua_module_free(module) == ULUA_OK);
    assert(ulua_instance_free(instance) == ULUA_OK);
    assert(ulua_context_free(context) == ULUA_OK);
    return 0;
}
