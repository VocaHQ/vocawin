#include <cassert>
#include <string>

#include "updater/Updater.h"

int main() {
    using namespace vocawin;

    // 1. Version compare: a < b.
    assert(Updater::isNewer("0.1.0", "0.2.0"));
    assert(Updater::isNewer("0.1.0", "1.0.0"));
    assert(Updater::isNewer("0.0.9", "0.1.0"));

    // 2. Not newer.
    assert(!Updater::isNewer("0.2.0", "0.1.0"));
    assert(!Updater::isNewer("0.1.0", "0.1.0"));
    assert(!Updater::isNewer("1.0.0", "0.9.0"));

    // 3. parseVersion works on a v-prefixed string too.
    const auto v = Updater::parseVersion("v1.2.3");
    assert(v.major == 1);
    assert(v.minor == 2);
    assert(v.patch == 3);

    // 4. defaultChannel is a non-empty string.
    assert(!Updater::defaultChannel().empty());

    return 0;
}
