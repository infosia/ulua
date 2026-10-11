#ifndef ULUA_RUNTIME_ERRORS_H
#define ULUA_RUNTIME_ERRORS_H
#include "ulua_shim.h"
#include "lua.h"
#include <cstring>

// The chunk name is fixed by load(). Strip only the VM location syntax.
// VM/src/ldebug.cpp:360; VM/src/laux.cpp:77. A missing line can appear as -1.
static const char* run_message(const char* text) {
    if (std::strncmp(text, "script:", 7) != 0) return text;
    const char* line = text + 7;
    if (*line == '-') ++line;
    const char* digits = line;
    while (*line >= '0' && *line <= '9') ++line;
    return line != digits && line[0] == ':' && line[1] == ' ' ? line + 2 : text;
}

struct RunLimitError {
    const char* text;
    int code;
};
// Resource limits from the pinned VM. Compare complete messages after location removal.
// P1 reaches allocation failure, concatenation overflow, buffer block size, and Lua call depth.
// Other paths need unexposed library inputs, dynamic tables, metamethods, or recursive protected C calls.
static const RunLimitError run_limit_errors[] = {
    // VM/src/lmem.cpp:284,551,591,635,661,798,812: allocator failure, LUA_ERRMEM.
    // VM/src/ldo.cpp:198: MAX_STACK_SIZE slots (a size limit), also LUA_ERRMEM.
    // VM/src/ldo.cpp:414: the fixed error object for LUA_ERRMEM.
    {"not enough memory", SHIM_MEMORY},
    // VM/src/lvmutils.cpp:411: MAXSSIZE concatenation; VM/src/lbuflib.cpp:228: buffer read size.
    {"string length overflow", SHIM_MEMORY},
    // VM/src/lmem.cpp:239: luaM_toobig; callers include VM/src/lstring.cpp:74,100 and VM/src/lbuffer.cpp:14.
    {"memory allocation error: block too big", SHIM_MEMORY},
    // VM/src/ltable.cpp:396,417,452: MAXBITS/MAXSIZE table allocation.
    {"table overflow", SHIM_MEMORY},
    // VM/src/laux.cpp:436: SIZE_MAX buffer growth.
    {"buffer too large", SHIM_MEMORY},
    // VM/src/lstrlib.cpp:96: MAXSSIZE string repetition.
    {"resulting string too large", SHIM_MEMORY},
    // VM/src/lstrlib.cpp:143; VM/src/lutf8lib.cpp:104: result count integer overflow.
    {"string slice too long", SHIM_MEMORY},
    // VM/src/lstrlib.cpp:398: LUA_MAXCAPTURES array capacity.
    {"too many captures", SHIM_MEMORY},
    // VM/src/lstrlib.cpp:1219: MAXSSIZE/INT_MAX format size.
    {"size specifier is too large", SHIM_MEMORY},
    // VM/src/lstrlib.cpp:1517 -> VM/src/laux.cpp:40,42: MAXSSIZE packsize via argument error.
    {"invalid argument #1 to 'packsize' (format result too large)", SHIM_MEMORY},
    {"invalid argument #1 (format result too large)", SHIM_MEMORY},
    // VM/src/ltablib.cpp:283 -> VM/src/laux.cpp:40,42: INT_MAX move element count.
    {"invalid argument #3 to 'move' (too many elements to move)", SHIM_MEMORY},
    {"invalid argument #3 (too many elements to move)", SHIM_MEMORY},
    // VM/src/ldo.cpp:239: LUAI_MAXCALLS; VM/src/lapi.cpp:61: ensure_stack_impl.
    {"stack overflow", SHIM_STACK},
    // VM/src/ldo.cpp:250,742: LUAI_MAXCCALLS.
    {"C stack overflow", SHIM_STACK},
    // VM/src/laux.cpp:161: lua_checkstack failure with each pinned caller description.
    // VM/src/lstrlib.cpp:144; VM/src/lutf8lib.cpp:106.
    {"stack overflow (string slice too long)", SHIM_STACK},
    // VM/src/lstrlib.cpp:629.
    {"stack overflow (too many captures)", SHIM_STACK},
    // VM/src/lstrlib.cpp:1581.
    {"stack overflow (too many results)", SHIM_STACK},
    // VM/src/lapi.cpp:1205: protected C call stack reservation.
    {"stack limit", SHIM_STACK},
    // VM/src/ldo.cpp:452: finalizer stack reservation.
    {"too many results to invoke finalizer", SHIM_STACK},
    // VM/src/ltablib.cpp:376: INT_MAX result count or lua_checkstack failure.
    {"too many results to unpack", SHIM_STACK},
    // VM/src/lcorolib.cpp:63,163,188: coroutine result stack reservation.
    {"too many results to resume", SHIM_STACK},
    // VM/src/lcorolib.cpp:136,143: coroutine argument stack reservation.
    {"too many arguments to resume", SHIM_STACK},
    // VM/src/lstrlib.cpp:431: LUAI_MAXCCALLS recursive pattern depth.
    {"pattern too complex", SHIM_STACK},
    // VM/src/lvmutils.cpp:176,236: MAXTAGLOOP metamethod chain depth.
    {"'__index' chain too long; possible loop", SHIM_STACK},
    {"'__newindex' chain too long; possible loop", SHIM_STACK},
};

static int run_error_code(int status, const char* text) {
    if (status == LUA_ERRMEM) return SHIM_MEMORY;
    // VM/src/ldo.cpp:233,252: hard call-depth limits throw LUA_ERRERR without a distinct message.
    // Our lua_cpcall has no error handler. P1 cannot call xpcall or coroutine error handlers.
    // Therefore these are its only LUA_ERRERR origins. Never classify the generic message alone.
    if (status == LUA_ERRERR) return SHIM_STACK;
    for (const RunLimitError& error : run_limit_errors) {
        if (std::strcmp(text, error.text) == 0)
            return error.code;
    }
    // Numeric range, malformed format, bounds, and type errors remain internal.
    // Examples: VM/src/lstrlib.cpp:931,945,1232,1556; VM/src/lintlib.cpp:108,123.
    return SHIM_INTERNAL;
}
#endif
