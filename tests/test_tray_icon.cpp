#include <cassert>

#include "ui/TrayIcon.h"

int main() {
    vocawin::TrayIcon tray;
    assert(tray.initialize());
    tray.shutdown();
    return 0;
}
