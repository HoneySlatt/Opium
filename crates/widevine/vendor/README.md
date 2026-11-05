# Chromium's CDM interface

`content_decryption_module.h` and `content_decryption_module_export.h` are Chromium's, unchanged:
byte for byte the files of <https://chromium.googlesource.com/chromium/cdm> at commit
`8f62f3ba6f06483f3908f9cce4f8d931800c9d7c` (2026-04-30), checked on 2026-11-05. They are under the
BSD-3-Clause licence in `LICENSE.chromium`, Chromium's own, which that repository does not carry.

To take a newer revision: clone that repository, copy both files, note the commit here, and check
that the interface Opium uses (`ContentDecryptionModule_11`, `Host_11`) is unchanged.
