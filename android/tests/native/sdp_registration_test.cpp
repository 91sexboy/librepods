// SPDX-License-Identifier: GPL-3.0-or-later
#include "l2c_fcr_hook.h"
#include <jni.h>
#include <dlfcn.h>
#include <iostream>
#include <stdexcept>
#include <string>

namespace {
class Library {
public:
    explicit Library(const char *path) : handle(dlopen(path, RTLD_NOW | RTLD_LOCAL)) {
        if (!handle) {
            throw std::runtime_error(std::string("dlopen failed: ") + dlerror());
        }
    }
    ~Library() { dlclose(handle); }
    Library(const Library &) = delete;
    Library &operator=(const Library &) = delete;

    template<typename Function>
    Function symbol(const char *name) const {
        dlerror();
        void *address = dlsym(handle, name);
        const char *error = dlerror();
        if (error || !address) {
            throw std::runtime_error(std::string("Missing required symbol ") + name);
        }
        return reinterpret_cast<Function>(address);
    }
    void *get() const { return handle; }

private:
    void *handle;
};

void *sdpTarget = nullptr;
void *checkTarget = nullptr;
void *sdpReplacement = nullptr;
bool unexpectedRegistration = false;

// The native-hook SDK is an external system boundary. Capture the installed
// replacement and supply the real fixture function as its original trampoline.
int registerHook(void *target, void *replacement, void **backup) {
    if (!replacement || !backup || replacement == target ||
        (target != sdpTarget && target != checkTarget)) {
        unexpectedRegistration = true;
        return -1;
    }
    if (target == sdpTarget) {
        if (sdpReplacement) {
            unexpectedRegistration = true;
            return -1;
        }
        sdpReplacement = replacement;
    }
    *backup = target;
    return 0;
}

using RegisterRecord = tBTA_STATUS (*)(tSDP_DI_RECORD *, uint32_t *);
using SetSdpHook = void (*)(JNIEnv *, jobject, jboolean);
using RecordCount = uint32_t (*)();
}

int main(int argc, char **argv) {
    try {
        if (argc != 3 && argc != 4) {
            throw std::runtime_error("Usage: sdp_registration_test MODULE FIXTURE [CASE]");
        }
        const std::string scenario = argc == 4 ? argv[3] : "single_registration";
        if (scenario != "single_registration" && scenario != "disabled_vendor") {
            throw std::runtime_error("Unknown regression case: " + scenario);
        }
        Library fixture(argv[2]);
        const auto original = fixture.symbol<RegisterRecord>("BTA_DmSetLocalDiRecord");
        sdpTarget = reinterpret_cast<void *>(original);
        checkTarget = reinterpret_cast<void *>(fixture.symbol<uint8_t (*)(void *)>(
                "l2c_fcr_chk_chan_modes"));
        const auto recordCount = fixture.symbol<RecordCount>("FixtureRegisteredRecordCount");

        Library module(argv[1]);
        NativeAPIEntries entries{1, reinterpret_cast<void *>(&registerHook), nullptr};
        const auto moduleLoaded = module.symbol<NativeInit>("native_init")(&entries);
        if (!moduleLoaded) {
            throw std::runtime_error("native_init returned no module-loaded callback");
        }
        moduleLoaded("libbluetooth_jni.so", fixture.get());
        if (!sdpReplacement || unexpectedRegistration) {
            throw std::runtime_error("Expected a valid SDP hook installed through native_init");
        }
        module.symbol<SetSdpHook>(
                "Java_me_kavishdevar_librepods_utils_NativeBridge_setSdpHook")(
                nullptr, nullptr, scenario == "disabled_vendor" ? JNI_FALSE : JNI_TRUE);

        tSDP_DI_RECORD record{};
        record.vendor = 0x1234;
        record.vendor_id_source = 0x0002;
        record.primary_record = false;
        uint32_t handle = 0;
        reinterpret_cast<RegisterRecord>(sdpReplacement)(&record, &handle);

        const uint32_t actual = recordCount();
        if (actual != 1) {
            throw std::runtime_error("Expected one native SDP registration, got " +
                                     std::to_string(actual));
        }
        if (scenario == "disabled_vendor") {
            if (record.vendor != 0x1234 || record.vendor_id_source != 0x0002) {
                throw std::runtime_error("Disabled SDP spoof must preserve the original vendor identity");
            }
            std::cout << "PASS: disabled spoof preserves vendor identity and normal SDP registration\n";
        } else {
            std::cout << "PASS: one hook invocation creates exactly one native SDP record\n";
        }
        return 0;
    } catch (const std::exception &error) {
        std::cerr << "FAIL: " << error.what() << '\n';
        return 1;
    }
}
