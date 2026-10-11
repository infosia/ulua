#ifndef ULUA_SHIM_H
#define ULUA_SHIM_H
#include "ulua.h"
/* Private shim statuses: memory, steps, stack, host, internal. */
enum { SHIM_OK=0, SHIM_MEMORY=1, SHIM_STEPS=2, SHIM_STACK=3, SHIM_HOST=4, SHIM_INTERNAL=5 };
struct ulua_vm;
#ifdef __cplusplus
extern "C" {
#define ULUA_NOEXCEPT noexcept
#else
#define ULUA_NOEXCEPT
#endif
int ulua_shim_compile(const char*, size_t, char**, size_t*, char*) ULUA_NOEXCEPT;
void ulua_shim_bytecode_free(char*) ULUA_NOEXCEPT;
int ulua_shim_new(const char*, size_t, const ulua_host*, const ulua_run_limits*, struct ulua_vm**, char*) ULUA_NOEXCEPT;
int ulua_shim_call(struct ulua_vm*, const char*, const ulua_value*, size_t, uint32_t, ulua_value*, char*) ULUA_NOEXCEPT;
void ulua_shim_free(struct ulua_vm*) ULUA_NOEXCEPT;
#ifdef __cplusplus
}
#endif
#endif
