#include <cassert>

#include "ui/OverlayWindow.h"

int main() {
    using namespace vocawin;

    OverlayWindow o;
    assert(!o.isVisible());

    o.show();
    assert(o.isVisible());

    o.hide();
    assert(!o.isVisible());

    o.setEnabled(false);
    o.show();
    assert(!o.isVisible());

    o.setEnabled(true);
    o.show();
    assert(o.isVisible());
    o.setEnabled(false);
    assert(!o.isVisible());

    o.setEnabled(true);
    o.setState(OverlayWindow::State::Recording);
    o.show();
    assert(o.isVisible());

    o.setState(OverlayWindow::State::Idle);
    o.hide();
    assert(!o.isVisible());

    return 0;
}
