// Inspect production bytecode without execution or emitter optimization metadata.
#include "../shim/ulua_shim.h"
#include "Luau/BytecodeUtils.h"
#include "lua.h"
#include "lualib.h"
#include "lstate.h"
#include "ldebug.h"
#include "lapi.h"
#include <cstring>

static void inspect(Proto* proto, unsigned* fastcalls, unsigned* calls, size_t lines) {
    for (int pc = 0; pc < proto->sizecode;) {
        const Instruction instruction = proto->code[pc];
        const auto op = LuauOpcode(LUAU_INSN_OP(instruction));
        const int line = luaG_getline(proto, pc);
        if (line > 0 && size_t(line) < lines) {
            if (Luau::isFastCall(op)) {
                const int builtin = LUAU_INSN_A(instruction);
                if (builtin == LBF_MATH_FMOD) fastcalls[line] |= 1;
                else if (builtin == LBF_TOSTRING) fastcalls[line] |= 2;
                else if (builtin == LBF_MATH_FLOOR) fastcalls[line] |= 4;
                else if (builtin == LBF_MATH_CEIL) fastcalls[line] |= 8;
                else fastcalls[line] |= 16;
            }
            if (op == LOP_CALL) ++calls[line];
        }
        pc += Luau::getOpLength(op);
    }
    for (int i = 0; i < proto->sizep; ++i) inspect(proto->p[i], fastcalls, calls, lines);
}

extern "C" int ulua_internal_probe_builtin_aliases(const char* source, size_t length,
    unsigned* fastcalls, unsigned* calls, size_t lines, char* error) {
    char* bytecode = nullptr;
    size_t size = 0;
    int status = ulua_internal_shim_compile(source, length, &bytecode, &size, error);
    if (status) return status;
    lua_State* state = luaL_newstate();
    if (!state) { ulua_internal_shim_bytecode_free(bytecode); return SHIM_MEMORY; }
    status = luau_load(state, "builtin-alias-probe", bytecode, size, 0);
    ulua_internal_shim_bytecode_free(bytecode);
    if (!status) inspect(clvalue(luaA_toobject(state, -1))->l.p, fastcalls, calls, lines);
    else {
        std::strncpy(error, lua_tostring(state, -1), 255);
        error[255] = '\0';
    }
    lua_close(state);
    return status;
}
