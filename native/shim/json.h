#ifndef ULUA_JSON_H
#define ULUA_JSON_H
// Rust advances the bounded grammar cursor. Only this frame allocates VM values.
static int json_parse(lua_State* state) {
    auto* vm = static_cast<ulua_vm*>(lua_tolightuserdata(state, lua_upvalueindex(1)));
    size_t length = 0;
    const char* input = lua_tolstring(state, 1, &length);
    charge_steps(state, vm, NATIVE_CALL_STEPS + length / NATIVE_BYTES_PER_STEP);
    if (length > static_cast<size_t>(INT32_MAX) - 144) {
        record_stop(vm, SHIM_MEMORY);
        luaL_error(state, "JSON scratch exceeds memory limit");
    }
    // The fixed grammar stack and string scratch are charged VM allocations.
    auto* output = static_cast<unsigned char*>(lua_newbuffer(state, length + 144));
    std::memset(output, 0, 144);
    lua_createtable(state, 128, 0); // parent value tables
    const int parents = lua_gettop(state);
    lua_createtable(state, 128, 0); // pending object keys
    const int keys = lua_gettop(state);
    lua_pushnil(state);
    const int root = lua_gettop(state);
    double cursor = 0;
    unsigned depth = 0;
    while (true) {
        const int event = vm->number_dispatch(14, cursor, 0, 0,
            reinterpret_cast<const unsigned char*>(input), length, output, &cursor);
        if (event < 0 || event == 8) {
            lua_pushinteger(state, event < 0 ? 1 : output[130] ? 2 : 0);
            lua_pushvalue(state, root);
            return 2;
        }
        uint64_t bits = 0;
        for (unsigned i = 0; i < 8; ++i) bits |= uint64_t(output[136 + i]) << (8 * i);
        if (event == 6) {
            lua_pushlstring(state, reinterpret_cast<const char*>(output + 144), static_cast<size_t>(bits));
            lua_rawseti(state, keys, static_cast<int>(depth));
            continue;
        }
        if (event == 7) {
            lua_pushnil(state);
            lua_rawseti(state, parents, static_cast<int>(depth));
            lua_pushnil(state);
            lua_rawseti(state, keys, static_cast<int>(depth));
            --depth;
            continue;
        }
        lua_createtable(state, 0, 2);
        const int node = lua_gettop(state);
        lua_pushinteger(state, event);
        lua_setfield(state, node, "k");
        if (event == 1) {
            double value = 0;
            std::memcpy(&value, &bits, sizeof(value));
            lua_pushnumber(state, value);
        } else if (event == 2) {
            lua_pushlstring(state, reinterpret_cast<const char*>(output + 144), static_cast<size_t>(bits));
        } else if (event == 3) {
            lua_pushboolean(state, bits != 0);
        } else if (event == 4 || event == 5) {
            lua_createtable(state, 0, 0);
        } else {
            lua_pushnil(state);
        }
        lua_setfield(state, node, "v");
        if (depth == 0) {
            lua_pushvalue(state, node);
            lua_replace(state, root);
        } else {
            lua_rawgeti(state, parents, static_cast<int>(depth));
            const int parent = lua_gettop(state);
            lua_rawgeti(state, keys, static_cast<int>(depth));
            if (lua_isnil(state, -1)) {
                lua_pop(state, 1);
                lua_pushvalue(state, node);
                lua_rawseti(state, parent, lua_objlen(state, parent) + 1);
            } else {
                lua_pushvalue(state, node);
                lua_rawset(state, parent);
                lua_pushnil(state);
                lua_rawseti(state, keys, static_cast<int>(depth));
            }
            lua_pop(state, 1);
        }
        if (event == 4 || event == 5) {
            ++depth;
            lua_getfield(state, node, "v");
            lua_rawseti(state, parents, static_cast<int>(depth));
        }
        lua_pop(state, 1);
    }
}
#endif
