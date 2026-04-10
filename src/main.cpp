#include <iostream>

#include "app/AppController.h"

int main() {
    vocawin::AppController app;
    if (!app.initialize()) {
        std::cerr << "Failed to initialize VocaWin\n";
        return 1;
    }

    std::cout << "VocaWin foundation initialized.\n";
    app.shutdown();
    return 0;
}
