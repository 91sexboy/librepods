# Native SDP hook regressions and forwarding contract

Run compilation and tests in GitHub Actions, not on the editing server:

```sh
cmake -S android/tests/native -B "$RUNNER_TEMP/librepods-native-tests"
cmake --build "$RUNNER_TEMP/librepods-native-tests" --parallel 2
ctest --test-dir "$RUNNER_TEMP/librepods-native-tests" --output-on-failure --no-tests=error
```

## Public boundary

The test compiles the unmodified production native module and real XZ decoder,
loads a real Bluetooth-named ELF fixture, and enters through `native_init` and its
library-load callback. The production resolver reads the fixture's actual ELF
and process mappings. The native-hook SDK boundary rejects replacements that
alias the target, records the installed replacement, and supplies the fixture's
real SDK function as its original trampoline. The test invokes only that
installed SDP replacement, setting vendor policy through the public JNI setter.
It never edits the production original-function pointer or calls a private helper.

Only the unavailable external Android logging and Bluetooth/native-hook SDK
boundaries are substituted. `bluetooth_fixture.h` exposes configuration and
request observations for that external SDK, not the module's internal state.
Sanitizers instrument the module, fixture, and test executable;
undefined-behavior recovery is disabled so UBSan findings cannot recover to a
successful test exit. Unknown CLI case names fail before loading either library.

## Cases

| CTest name | CLI case | Contract |
| --- | --- | --- |
| `sdp_one_registration` | `single_registration` (default) | One installed-hook invocation creates exactly one successful non-primary SDK record. |
| `sdp_disabled_preserves_vendor` | `disabled_vendor` | Disabled vendor policy preserves `0x1234` / `0x0002` without skipping normal registration; the SDK record count remains exactly one. |
| `sdp_sdk_forwarding_contract` | `forwarding_contract` | One parameterized characterization of existing SDK forwarding with enabled vendor policy. |

The forwarding case covers all named `tBTA_STATUS` values: `BTA_SUCCESS`,
`BTA_FAILURE`, `BTA_PENDING`, `BTA_BUSY`, `BTA_NO_RESOURCES`, and `BTA_WRONG_MODE`,
plus the unnamed byte `0xA5`. The enum has a fixed `uint8_t` underlying type, so
that byte is representable; the forwarding contract does not normalize it to a
named status. An additional failure row leaves the handle untouched. These are
existing-code characterization checks, not a new failing regression or TDD RED.

For every row, the installed callback must:

- Return the external SDK's exact status and preserve its handle output,
  including nonzero success/failure outputs, a zero output, and an unchanged
  nonzero caller sentinel when the SDK does not write the handle.
- Forward the original record and handle pointers, including the input handle.
- Rewrite only `vendor` to `0x004C` and `vendor_id_source` to `0x0001`, both at SDK
  entry and in the caller's record after return.
- Preserve `product`, `version`, `primary_record`, and all bytes of
  `client_executable_url`, `service_description`, and `documentation_url`, both
  at SDK entry and after return. The buffers include nonzero bytes after their
  first NUL; checks compare complete arrays, not struct padding or only strings.
- Make exactly one SDK attempt. The cumulative registry contains only the first
  successful non-primary record, regardless of subsequent non-success outputs.

Status/handle outcomes and Apple vendor values are literal oracles independent
of the production wrapper. Untouched fields are compared with a pre-call copy of
caller-owned data. Both primary flag values are supplied; `true` occurs only on
non-success rows, without assuming successful primary allocation behavior.

## Fixture accounting and exclusions

The first two cases retain the default successful non-primary fixture behavior:
one SDK call adds one registry entry and writes a generated handle.
`FixtureRegistrationAttemptCount` counts SDK entries separately;
`FixtureRegisteredRecordCount` increments only for `BTA_SUCCESS` with
`primary_record = false`. Failure, pending, busy, resource/mode errors, and an
unknown status allocate no registry entry. Configuring a non-success handle
output tests forwarding, not allocation or an assertion that every real SDK
writes handles on failure.

In
[AOSP's `BTA_DmSetLocalDiRecord`](https://android.googlesource.com/platform/packages/modules/Bluetooth/+/refs/heads/main/system/bta/dm/bta_dm_api.cc),
the returned handle is added to `di_handle` and `di_num` increments only for a
non-primary record. This bounded fixture does not prove OEM primary-record
allocation behavior. A missing-original case is intentionally excluded: valid
SDK installation at this seam supplies an original trampoline; the harness does
not violate that contract or overwrite a private backup just for coverage.

The tests do not exercise LSPosed inline patching, an OEM Bluetooth binary,
compressed `.gnu_debugdata` resolution, or Android service restart behavior. The
real XZ sources remain compiled, but their decompression branch is not exercised
by this fixture. A RED/GREEN result from the existing regressions does not
identify the exact Xiaomi crash from #873, whose report contains no crash
stack/tombstone. Keep #873 related/open until a reporter-side reproduction
establishes that connection.
