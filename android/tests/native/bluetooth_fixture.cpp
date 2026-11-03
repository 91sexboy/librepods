// SPDX-License-Identifier: GPL-3.0-or-later
#include "l2c_fcr_hook.h"

namespace {
uint32_t registeredRecords = 0;
}

extern "C" uint8_t l2c_fcr_chk_chan_modes(void *) {
    return 0;
}

// This external Bluetooth SDK fixture models successful non-primary DI registration.
// Each call allocates one record; its registry catches extra registrations, not
// private hook state. It does not model OEM primary-record allocation behavior.
extern "C" tBTA_STATUS BTA_DmSetLocalDiRecord(tSDP_DI_RECORD *record, uint32_t *handle) {
    if (!record || !handle) {
        return BTA_FAILURE;
    }
    ++registeredRecords;
    *handle = 0xA100 + registeredRecords;
    return BTA_SUCCESS;
}

extern "C" uint32_t FixtureRegisteredRecordCount() {
    return registeredRecords;
}
