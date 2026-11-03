# Native SDP hook regression

Run compilation and tests in GitHub Actions, not on the editing server:

```sh
cmake -S android/tests/native -B "$RUNNER_TEMP/librepods-native-tests"
cmake --build "$RUNNER_TEMP/librepods-native-tests" --parallel 2
ctest --test-dir "$RUNNER_TEMP/librepods-native-tests" --output-on-failure --no-tests=error
```

The test compiles the production native module and real XZ decoder, loads a real
Bluetooth-named ELF fixture, and enters through `native_init` and its library-load
callback. The production resolver reads the fixture's actual ELF and process
mappings. The native-hook SDK boundary rejects replacements that alias the target,
records the installed replacement, and supplies the fixture's real SDK function
as its original trampoline; the test then invokes that installed SDP replacement
and observes the fixture's registry.
It never edits the production original-function pointer or calls a private helper.

Only the unavailable external Android logging and Bluetooth/native-hook SDK
boundaries are substituted. Sanitizers instrument the module, fixture, and test
executable; undefined-behavior recovery is disabled so UBSan findings cannot
recover to a successful test exit.

The test sets `primary_record = false`. The fixture models successful non-primary
DI registration, allocating one record per SDK call. In
[AOSP's `BTA_DmSetLocalDiRecord`](https://android.googlesource.com/platform/packages/modules/Bluetooth/+/refs/heads/main/system/bta/dm/bta_dm_api.cc),
the returned handle is added to `di_handle` and `di_num` increments only for a
non-primary record. This bounded case covers repeated non-primary registration
in v1.0.4; it does not prove OEM primary-record allocation behavior.

The test does not exercise LSPosed inline patching, an OEM Bluetooth binary, or
Android service restart behavior. A RED/GREEN result does not identify the exact
Xiaomi crash from #873, whose report contains no crash stack/tombstone. Keep #873
related/open until a reporter-side reproduction establishes that connection.
