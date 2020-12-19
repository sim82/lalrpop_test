
#include <cstdint>
#include <iostream>

extern "C"
{
    extern int64_t test1();
}

int main()
{
    auto ref = 1 + 2 * 5 + 4 * 7 * 4 + 2 * 2 + 11;
    std::cout << "res: " << test1() << " " << ref << std::endl;
}