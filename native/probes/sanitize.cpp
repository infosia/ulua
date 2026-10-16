// Standalone sanitizer detector probe. Never link this file into a library.
#include <climits>
#include <cstdlib>
#include <cstring>

int main(int argc, char** argv) {
    if (argc != 3) return 2;
    if (std::strcmp(argv[1], "heap") == 0) {
        size_t size = std::strtoul(argv[2], nullptr, 10);
        unsigned char* volatile bytes = static_cast<unsigned char*>(std::malloc(size));
        if (!bytes) return 3;
        bytes[size] = 1;
        std::free(bytes);
    } else if (std::strcmp(argv[1], "overflow") == 0) {
        volatile int maximum = INT_MAX;
        volatile int result = maximum + 1;
        (void)result;
    } else {
        return 2;
    }
    return 0;
}
