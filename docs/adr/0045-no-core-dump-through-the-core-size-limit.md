# 0045. No core dump, through the core size limit

- Status: Accepted; amends [0039](0039-the-exit-watch-leaves-no-core-dump-unless-asked.md) (how
  dumps are kept off disk)
- Date: 2026-11-10

## Context

[ADR-0039](0039-the-exit-watch-leaves-no-core-dump-unless-asked.md) keeps core dumps, which would
hold the services' tokens, the downloads' key and decrypted audio, off disk by making Opium not
dumpable (`prctl(PR_SET_DUMPABLE, 0)`). Before it was built, a search of the sources found that a
process that is not dumpable has every file of its `/proc/<pid>` folder but the folder itself owned
by root, so that other processes of the same user can no longer open `/proc/<pid>/root`.
xdg-desktop-portal opens that file to tell a sandboxed app from one installed on the host, and
refuses every call when it cannot: the folder and file windows, which Opium asks of the portal,
would no longer open. xdg-desktop-portal's issue 1490 ("Unable to open /proc/$pid/root") reports
it for KeePassXC and Bitwarden; its maintainers keep the refusal on purpose, as failing open could
be exploited, and closed unmerged the two pull requests that tried to change it (2119, 2178).
KeePassXC leaves `PR_SET_DUMPABLE` out of its snap build for this reason.

Under the usual `core_pattern`, which pipes a dump to `systemd-coredump`, the kernel does not
enforce the core size limit (`RLIMIT_CORE`), but `systemd-coredump` reads the crashed process's
limit and stores no dump below a page ("Resource limits disable core dumping for process…", in
systemd's `src/coredump/`; the kernel's `fs/coredump.c` sets no limit on a piped dump).

## Decision

**Opium sets its core size limit to 0 at start**, the soft and the hard limit both, so that nothing
in the process raises it again and the programs it starts inherit it: a hang that the exit watch
ends by `abort()`, or any crash, leaves no core dump. **`OPIUM_CORE_DUMPS=1` in the environment
leaves the limit as it was**, for whoever debugs a hang on a throwaway profile. Opium stays
dumpable: the portal, PipeWire and the session bus see it as before. Built in M1.15.

## Alternatives considered

- **Not dumpable, as ADR-0039 decided**: no dump is even attempted, and other processes of the
  user can no longer read Opium's memory; but every call to the portal fails, the folder and file
  windows with it.
- **Both, as ssh-agent does**: the strongest, and the portal lost too, unless the file windows are
  made without it, in frozen code.

## Consequences

- With `systemd-coredump`, a crash still leaves a line in the journal with Opium's command line,
  environment, open files and memory map, though no dump: nothing secret may be put in Opium's
  environment, which holds none today.
- A crash handler other than `systemd-coredump` that ignores the limit (some distributions'
  `apport` or `abrt`, a custom script) may still keep a dump; Opium does not guard against it.
- Other processes of the same user can still read Opium's memory, as they could before.
- ADR-0039's variable is named here: `OPIUM_CORE_DUMPS`.
