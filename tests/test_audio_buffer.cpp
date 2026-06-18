#include <cassert>
#include <vector>

#include "audio/AudioBuffer.h"

int main() {
    // 1. Push + pop happy path.
    {
        vocawin::AudioBuffer buffer(1024);
        std::vector<float> input(100);
        for (size_t i = 0; i < input.size(); ++i) {
            input[i] = static_cast<float>(i);
        }
        const bool pushed = buffer.push(input.data(), input.size());
        assert(pushed);
        assert(buffer.available() == 100);

        std::vector<float> output(100);
        const size_t popped = buffer.pop(output.data(), output.size());
        assert(popped == 100);
        assert(buffer.available() == 0);
        for (size_t i = 0; i < input.size(); ++i) {
            assert(output[i] == input[i]);
        }
    }

    // 2. Overflow: push more than capacity -> push returns false.
    {
        vocawin::AudioBuffer buffer(64);
        std::vector<float> input(100, 1.0f);
        const bool pushed = buffer.push(input.data(), input.size());
        assert(!pushed);
        // Buffer should not have accepted any samples when capacity is too small
        // (MVP: no partial push).
        assert(buffer.available() == 0);
    }

    // 3. Partial pop.
    {
        vocawin::AudioBuffer buffer(1024);
        std::vector<float> input(1000, 0.5f);
        const bool pushed = buffer.push(input.data(), input.size());
        assert(pushed);

        std::vector<float> first(500);
        const size_t firstPopped = buffer.pop(first.data(), first.size());
        assert(firstPopped == 500);
        assert(buffer.available() == 500);

        std::vector<float> second(500);
        const size_t secondPopped = buffer.pop(second.data(), second.size());
        assert(secondPopped == 500);
        assert(buffer.available() == 0);
    }

    // 4. Clear.
    {
        vocawin::AudioBuffer buffer(1024);
        std::vector<float> input(500, 0.25f);
        buffer.push(input.data(), input.size());
        assert(buffer.available() == 500);
        buffer.clear();
        assert(buffer.available() == 0);
    }

    // 5. Wrap-around: fill, drain, refill, drain. Exercises index wrap.
    {
        const size_t cap = 128;
        vocawin::AudioBuffer buffer(cap);
        std::vector<float> input(cap, 1.0f);
        for (size_t i = 0; i < cap; ++i) {
            input[i] = static_cast<float>(i);
        }

        const bool pushed1 = buffer.push(input.data(), input.size());
        assert(pushed1);
        std::vector<float> output1(cap);
        const size_t popped1 = buffer.pop(output1.data(), output1.size());
        assert(popped1 == cap);

        // Now refill — read/write indices have wrapped.
        const bool pushed2 = buffer.push(input.data(), input.size());
        assert(pushed2);
        std::vector<float> output2(cap);
        const size_t popped2 = buffer.pop(output2.data(), output2.size());
        assert(popped2 == cap);

        for (size_t i = 0; i < cap; ++i) {
            assert(output2[i] == input[i]);
        }
    }

    // 6. Empty pop: pop from empty buffer returns 0, no crash.
    {
        vocawin::AudioBuffer buffer(64);
        std::vector<float> output(10);
        const size_t popped = buffer.pop(output.data(), output.size());
        assert(popped == 0);
    }

    // 7. Capacity accessor.
    {
        vocawin::AudioBuffer buffer(256);
        assert(buffer.capacity() == 256);
    }

    return 0;
}
