// Standalone P5.4 probe. Link this file with the pinned compiler archive.
// Add the Common, Ast, Compiler, Bytecode include directories and Compiler/src.
// Replacement new counts STL containers and AST pages. The lcode.cpp hook counts the bytecode malloc.
// Counts use requested bytes, without allocator headers, input files, or process startup.
// The pinned compiler uses no other direct malloc, calloc, or realloc calls.
#include "luacode.h"
#include "Luau/Compiler.h"
#include <cstddef>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <fstream>
#include <iterator>
#include <new>
#include <string>

namespace
{
struct alignas(std::max_align_t) Header
{
    size_t size;
    bool counted;
};
bool active = false;
size_t live = 0;
size_t peak = 0;
size_t allocations = 0;

void* allocate(size_t size)
{
    auto* header = static_cast<Header*>(std::malloc(sizeof(Header) + size));
    if (!header)
        return nullptr;
    header->size = size;
    header->counted = active;
    if (active)
    {
        live += size;
        peak = peak > live ? peak : live;
        ++allocations;
    }
    return header + 1;
}
void release(void* pointer)
{
    if (!pointer)
        return;
    auto* header = static_cast<Header*>(pointer) - 1;
    if (header->counted)
        live -= header->size;
    std::free(header);
}
}

void* operator new(size_t size)
{
    if (void* pointer = allocate(size))
        return pointer;
    throw std::bad_alloc();
}
void* operator new[](size_t size) { return operator new(size); }
void operator delete(void* pointer) noexcept { release(pointer); }
void operator delete[](void* pointer) noexcept { release(pointer); }
void operator delete(void* pointer, size_t) noexcept { release(pointer); }
void operator delete[](void* pointer, size_t) noexcept { release(pointer); }

// Include the pinned C entry with only its result malloc replaced.
#define malloc allocate
#include "lcode.cpp"
#undef malloc

int main(int argc, char** argv)
{
    try
    {
        size_t largestPeak = 0;
        double largestRatio = 0;
        const char* peakShape = "";
        const char* ratioShape = "";
        for (int i = 1; i < argc; ++i)
        {
            std::ifstream file(argv[i], std::ios::binary);
            if (!file)
                return 1;
            std::string source((std::istreambuf_iterator<char>(file)), std::istreambuf_iterator<char>());
            live = peak = allocations = 0;
            active = true;
            size_t size = 0;
            char* bytecode = luau_compile(source.data(), source.size(), nullptr, &size);
            active = false;
            if (!bytecode)
                return 1;
            bool success = size != 0 && bytecode[0] != 0;
            double ratio = source.empty() ? 0 : double(peak) / double(source.size());
            if (peak > largestPeak)
            {
                largestPeak = peak;
                peakShape = argv[i];
            }
            if (ratio > largestRatio)
            {
                largestRatio = ratio;
                ratioShape = argv[i];
            }
            std::printf("%s output=%zu peak=%zu ratio=%.6f allocations=%zu bytecode=%zu success=%d\n",
                argv[i], source.size(), peak, ratio, allocations, size, success);
            release(bytecode);
            if (live != 0)
                return 2;
        }
        std::printf("P5.4 native largest-peak=%zu shape=%s largest-ratio=%.6f shape=%s\n",
            largestPeak, peakShape, largestRatio, ratioShape);
    }
    catch (const std::exception& error)
    {
        std::fprintf(stderr, "%s\n", error.what());
        return 1;
    }
    return 0;
}
