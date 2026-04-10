#include <cassert>

#include "app/SingleInstance.h"

int main() {
    vocawin::SingleInstance instance(L"VocaWin-Test-Mutex");
    const bool acquired = instance.acquire();
    assert(acquired == instance.hasLock());
    return 0;
}
