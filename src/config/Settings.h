#pragma once

#include <string>

namespace vocawin {

struct Settings {
    std::string model_id{"base"};
    std::string language{"auto"};
    bool launch_at_startup{true};
    bool sound_effects{true};
    bool preserve_clipboard{true};
};

}  // namespace vocawin
