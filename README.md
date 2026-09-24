# ZipMount

[![Latest release](https://img.shields.io/github/v/release/marlogg74mp/zipmount?label=download)](https://github.com/marlogg74mp/zipmount/releases/latest)
[![CI](https://github.com/marlogg74mp/zipmount/actions/workflows/ci.yml/badge.svg)](https://github.com/marlogg74mp/zipmount/actions/workflows/ci.yml)

*[Русская версия](README.ru.md)*

### [⬇ Download ZipMount for Windows](https://github.com/marlogg74mp/zipmount/releases/latest)

Take **`ZipMount-Setup-<version>.exe`** from the release page and run it —
WinFsp comes inside. 64-bit Windows 10 (version 2004 or later) or Windows 11.
More in [Installation](#installation) · [website](https://marlogg74mp.github.io/zipmount/)

**Linux**: a static binary for x86_64 and ARM64 is in the same release — see [Linux](#linux).
**macOS**: `brew install marlogg74mp/tap/zipmount` — see [macOS](#macos).

![ZipMount](docs/hero.jpg)

Mounts an archive as an ordinary Windows drive — read-only, without extracting
anything. A volume shows up in File Explorer with working navigation, filename
search, opening files from any program, and copying out. The archive stays a
single file; contents are decompressed lazily.

Supports **zip**, **7z**, **tar** and **tar.gz**; **rar** in a build you make
yourself ([why](#about-rar-not-in-official-builds)).

```
zipmount mount D:\data\archive.7z Z:
```

Speaks English, Russian, Chinese, Japanese, Korean, Portuguese, Spanish and
German — the command line, the context menu and the installer
([how the language is chosen](#languages)).

A personal project, maintained in spare time. Bug reports with an archive that
reproduces the problem are the most useful kind.

## Why, when 7-Zip exists

7-Zip and Windows' built-in Compressed Folders are shells over an archive: no
drive letter, no access from arbitrary programs through an ordinary path, no
usable recursive search. ZipMount brings up a real volume through
[WinFsp](https://winfsp.dev/) — the same mechanism behind rclone, sshfs-win and
Google Drive for desktop.

## Installation

Take the newest installer from
[Releases](https://github.com/marlogg74mp/zipmount/releases/latest). Run
**`ZipMount-Setup-<version>.exe`** and confirm the elevation prompt. It installs
ZipMount into `C:\Program Files\ZipMount`, adds that directory to `PATH`, and
sets up the modern context menu — including trusting the package certificate,
which needs no separate prompt because an installer is elevated already.

It also carries WinFsp, the driver through which Windows sees an archive as a
drive, and installs it **only if it is missing**. The file inside is the
official installer from winfsp.dev, unmodified and with its author's signature
intact; an already installed driver is never touched, including a newer one.

Uninstalling from "Installed apps" takes ZipMount back out, the certificate
included. **WinFsp stays.** It is shared — rclone and sshfs-win run on the same
driver — and carrying it off would break them without warning. Remove it
separately from the same list if you want it gone.

Two things worth knowing up front. The installer is not code-signed, so the
elevation prompt says "Publisher: Unknown" and a freshly downloaded copy will
draw a SmartScreen warning; removing that needs a real code-signing
certificate, and a self-signed one will not do. Until then there are two ways
to check what you downloaded: compare it against `SHA256SUMS.txt` from the same
release, or have GitHub confirm it was built by this repository's release
workflow from the tagged commit:

```
gh attestation verify ZipMount-Setup-<version>.exe --repo marlogg74mp/zipmount
```

And if the menu fails to
register, the install still succeeds — the menu is a bonus, not a precondition,
and `zipmount doctor` says whether it came up.

`ZipMount-<version>.msi` is published alongside, for when WinFsp is already
installed and a plain package is preferable — for deployment through group
policy, say.

If you had already set the menu up by hand, run `zipmount shell-uninstall`
first, or the unmount item will show up twice.

### From source

```
winget install WinFsp.WinFsp
cargo build --release
```

Check readiness with `zipmount doctor`. For rar support, add
`--features zipmount/rar` — for your own use only, see
[below](#about-rar-not-in-official-builds).

`shell-install --modern` additionally needs the Windows SDK — `makeappx` and
`signtool` come from there:

```
winget install Microsoft.WindowsSDK.10.0.26100
```

The build does not require LLVM: the vendored copy of `winfsp-sys` in `vendor/`
uses pre-generated bindings instead of running bindgen (see the comments in its
`build.rs`).

### Linux

Every release carries `zipmount-<version>-linux-x86_64.tar.gz` and
`-linux-arm64.tar.gz`: one static binary (musl), so it runs on any
distribution. Mounting goes through FUSE, which every mainstream distribution
ships as `fuse3` — the `/dev/fuse` device and the `fusermount3` helper.

```
sudo apt install fuse3            # Debian, Ubuntu; dnf or pacman elsewhere
tar -xzf zipmount-<version>-linux-x86_64.tar.gz
sudo install zipmount-<version>-linux-x86_64/zipmount /usr/local/bin/
zipmount mount archive.7z
```

Building it yourself needs nothing but Rust — no libfuse, no `-dev` packages:
`cargo build --release`.

Without a directory the archive is mounted on `~/ZipMount/<archive name>`,
created for the mount and removed after it; file managers show a mount in the
home folder in their sidebar. `zipmount unmount`, `fusermount3 -u` or Ctrl+C in
the terminal that mounted it all take it down cleanly. `zipmount mounts` reads
the kernel's own list, so nothing goes stale when a process is killed. The
archive commands (`ls`, `find`, `grep`, `info`, `verify`) work exactly as on
Windows, with or without FUSE.

`zipmount shell-install` adds **Mount with ZipMount** to the file managers:
straight in Dolphin's context menu (KDE), under Scripts in GNOME Files, and
under Open With in any file manager. The items are files in your home
directory — no root, nothing to sign — and `zipmount shell-uninstall` takes
them away. Started from a menu, a failure shows up as a notification.

### macOS

Through Homebrew, which builds it on your Mac — so Gatekeeper, which blocks
unsigned downloads, has nothing to object to:

```
brew install marlogg74mp/tap/zipmount
zipmount mount archive.7z --open
```

Every release also carries `zipmount-<version>-macos-arm64.tar.gz` for Apple
Silicon; it is not signed by Apple, so macOS refuses to start it until you
allow it in System Settings → Privacy & Security. Building it yourself needs
the Xcode command line tools (`xcode-select --install`) and Rust. Nothing else
is needed — no macFUSE, no kernel extension, no administrator rights.

macOS has no FUSE of its own, and macFUSE asks the user to lower the system's
security on Apple Silicon. Its NFS client, though, is built in: `zipmount`
serves the archive as an NFSv3 export on this Mac only, bound to 127.0.0.1,
and the system mounts it with `mount_nfs` like any network share. Finder shows
it in its sidebar under **localhost**, with the archive's name as the volume;
Eject there unmounts it, as do `zipmount unmount` and Ctrl+C. The same
`~/ZipMount/<archive name>` directory is used as on Linux, and `zipmount
mounts` reads the system's own list of mounts.

Two things differ from the other systems. Reading through the volume runs at
about 200 MB/s on an M-series Mac — NFS costs a round trip per request, however
local — so for searching content `zipmount grep`, which reads the archive
directly, is the tool (1.2 s for 5.6 GB of logs against 70 s through the
volume). And right after a read the NFS client keeps the volume busy for a
while; `zipmount unmount` forces the unmount then, unless one of your
programs has a file open there, which it names instead.

`zipmount shell-install` adds **Mount with ZipMount** to Finder: right-click
an archive → Quick Actions. It is a Quick Action in `~/Library/Services`, so
nothing needs signing; it mounts, opens the folder, and shows an alert if
something went wrong. (The Quick Action opens the folder itself: when zipmount
opens it, macOS asks whether zipmount may access files on network volumes.)

Built and tested on macOS 26; older versions are not a target.

### Building the installers

```
powershell -ExecutionPolicy Bypass -File tools\build-installer.ps1
```

Needs the Windows SDK and WiX 4 with two extensions. The version matters: with
none given, the current branch is fetched, which WiX 4 cannot load and silently
marks "damaged".

```
dotnet tool install --global wix
wix extension add WixToolset.Bal.wixext/4.0.5
wix extension add WixToolset.Util.wixext/4.0.5
```

The order inside the script is not arbitrary: the program first, then the
context menu package — which the program builds and signs itself — then the
MSI, and only then the bundle that carries the MSI inside. The target machine
has no SDK, so the package cannot be built there. The WinFsp installer is
downloaded once into `target/deps` and its signature is verified before it goes
into the bundle.

The installer version is the version from `Cargo.toml` with the commit count
as a fourth field, `0.2.0.37`. That is not decoration: with a version that
never changes, Windows Installer does not see a new build as an upgrade, and
every install adds another entry to "Installed apps" instead of replacing the
previous one. The commit count alone used to serve, but it starts over in a
fresh clone of the history, and a smaller version stops upgrading. Windows
Installer compares only three fields, so the package also allows same-version
upgrades; the bundle compares all four.

### Continuous integration and releases

Every push to `master` and every pull request runs formatting, clippy with
warnings as errors, and the tests — twice: for the official build and for the
`rar` build, which is never distributed but has to keep working for those who
build it themselves. See `.github/workflows/ci.yml`.

A release is a tag:

```
# bump version in Cargo.toml [workspace.package], commit, then:
git tag v0.3.0
git push origin v0.3.0
```

The release workflow (`.github/workflows/release.yml`) refuses a tag that does
not match `Cargo.toml`, runs the tests on every system, and builds the Windows
installers, static Linux binaries for x86_64 and ARM64 and a macOS binary for
Apple Silicon. It then attaches a build provenance attestation to all of them
and creates one **draft** release with the files, `SHA256SUMS.txt` and notes.
Publishing it is one click on the release page — a release is public and
lands in watchers' notifications, so the last look is human. Started by hand
(Actions → Release → Run workflow), it builds and packs everything and stops
short of the release: a rehearsal before tagging.

The runner signs the context menu package with a self-signed certificate it
creates on the spot; its private key vanishes with the runner, so no key
outlives the release it signed.

The same can be done by hand:

```
powershell -ExecutionPolicy Bypass -File tools\build-installer.ps1
powershell -ExecutionPolicy Bypass -File tools\publish-release.ps1
```

The second script needs the GitHub CLI signed in to the repository owner's
account (`gh auth login`), refuses to publish a package built from a different
version than the one in `Cargo.toml`, and with `-WhatIf` prints what would be
published, and the SHA256 of each file, without touching anything.

## Commands

| Command | What it does |
|---|---|
| `zipmount mount <archive> [Z:]` | mount; without a letter the first free one is taken |
| `zipmount unmount <Z:\|archive>` | unmount |
| `zipmount mounts` | what is mounted right now |
| `zipmount ls <archive> [path]` | directory contents, `-r` for the whole tree |
| `zipmount find <archive> <pattern>` | search by name |
| `zipmount grep <archive> <pattern>` | search by content, bypassing the filesystem |
| `zipmount verify <archive>` | end-to-end read check against the CRC32 stored in the archive |
| `zipmount info <archive>` | summary; solid block count for 7z, memory held for tar.gz |
| `zipmount doctor` | check that the environment is ready for mounting |
| `zipmount shell-install [--modern]` | add items to the File Explorer context menu |
| `zipmount shell-uninstall` | remove them |
| `zipmount language [code\|auto]` | show or choose the program's language |

`mount` also takes `--detach` (mount in the background and return) and `--open`
(open the drive in File Explorer). The context menu uses both.

The format is detected from the file signature, not the extension. The mount
point can be a drive letter (`Z:`) or a path to an empty NTFS directory.

Every command accepts `-p` (prompt for a password with hidden input) and
`--password-stdin` (read the password from standard input).

## Languages

| Code | Language |
|---|---|
| `en` | English — the default and the reference |
| `ru` | Русский |
| `zh-CN` | 简体中文 |
| `ja` | 日本語 |
| `ko` | 한국어 |
| `pt-BR` | Português (Brasil) — Portugal gets it too |
| `es` | Español |
| `de` | Deutsch |

The language is decided in this order:

1. `ZIPMOUNT_LANG` — for one command or one console session;
2. the choice saved with `zipmount language <code>`, kept in
   `HKCU\Software\ZipMount\Language` (on Linux in
   `~/.config/zipmount/language`, on macOS in
   `~/Library/Application Support/ZipMount/language`);
3. the Windows display language, following the user's order of preference (on
   Linux and macOS the locale: `LANGUAGE`, `LC_ALL`, `LC_MESSAGES`, `LANG`,
   and then macOS's own list of preferred languages);
4. English.

```
zipmount language          # what is in effect now, and why
zipmount language ja       # choose Japanese
zipmount language auto     # follow Windows again
```

A language choice also reaches the context menu. The modern menu asks for its
texts every time it is drawn, so it follows at once. Registry items are static
strings, so `zipmount language` rewrites them — and for the drive item written
by the installer to `HKEY_LOCAL_MACHINE`, which a user cannot touch without
elevation, it places a copy with the new text in `HKEY_CURRENT_USER`, which File
Explorer prefers. Should ZipMount be uninstalled later, that copy stays
harmless: its `AppliesTo` shows it only on ZipMount volumes, and there are none
without the program.

The installer's window follows the Windows display language on its own.

The Russian translation is native; the others are machine-assisted, and
corrections from native speakers are very welcome. Every message lives in
`crates/zipmount-i18n/locales/<code>.ftl`, in [Fluent](https://projectfluent.org/)
syntax — Fluent because of plurals: Russian needs three forms ("1 файл",
"2 файла", "5 файлов") picked by CLDR rules, which simpler formats cannot
express. The tests insist that every language carries exactly the English set
of messages, so a missing or misspelled id fails the build rather than showing
up in a user's console. Adding a language is one `.ftl` file, one line in
`LANGUAGES`, and — for the installer — `installer/locales/<code>/` with a
`thm.wxl` and an `about.txt`.

Clap's own error messages ("unexpected argument", "[default: …]") stay in
English: clap has no way to translate them.

## File Explorer context menu

```
zipmount shell-install --modern   # the main Windows 11 menu
zipmount shell-install            # registry only, no administrator needed
zipmount shell-uninstall          # remove either variant
```

Four items, identical in both variants:

| Item | Where it appears |
|---|---|
| Mount as a drive | on `.zip .7z .tar .gz .tgz` files (and `.rar` in a rar build) |
| Mount to a letter | same, with a submenu |
| Search in archive… | same, content search in a separate console |
| Unmount (ZipMount) | on a folder inside a mounted volume, and on empty space in it |

The difference is where exactly Windows puts them.

### Why the modern variant is more involved

In Windows 11, registry commands do not reach the main context menu: they live
under "Show more options", or behind Shift+right-click. The main menu only
admits `IExplorerCommand` handlers declared by a package with application
identity. Hence three parts:

**`zipmount_shell.dll`** implements `IExplorerCommand` and lives inside the
File Explorer process. It knows nothing about archives — it does not even
depend on `zipfs-core`. Its entire job is to show the item and launch
`zipmount.exe` next to it. Opening an archive costs time and memory, and
Explorer would be the one paying.

**A sparse package** grants identity without pulling the program inside it: the
`.msix` holds only the manifest and the tiles, while `zipmount.exe` and the
library stay ordinary files in the "external content" directory. Everything
still works from the command line exactly as before.

**A signature and one elevated step.** Windows only accepts a signed package,
and trusting a certificate is a machine-wide decision. So the installer creates
a self-signed certificate (in the user's personal store, no elevation),
signs the package with it, and asks for elevation once to place the certificate
among the trusted ones. The certificate stays a visible file next to the
program, and `shell-uninstall` prints the command to remove it.

What this buys beyond the main menu: the letter submenu is built **at display
time**, so it lists only free letters. The registry cannot do that — there the
list is fixed at install time and quickly starts lying.

What it does not buy: unmounting by clicking the drive itself. The manifest
schema allows exactly four targets — a file extension, `*`, `Directory` and
`Directory\Background` — and a drive is not among them. `Directory` was tried
on the off chance that Windows counts a drive root as a folder; it does not, the
same way registry `Directory` verbs never show up on drives. So in the modern
variant the unmount item sits on folders inside the volume and on empty space
in it, while on the drive itself it is still placed by the registry — the one
item that stays under "Show more options".

To confirm the whole chain came together, `zipmount doctor` does more than list
installed packages: it creates the handler's COM object and asks it for the
item title. Between a registered package and a rendered line there is still the
library load, and a failure there looks exactly like an item that silently
never appeared.

### Three decisions in the registry variant

**Items go into `SystemFileAssociations`, not into the extension's ProgID.**
An archive's ProgID is usually taken by an archiver, and writing our commands
there would mean meddling with someone else's registration.
`SystemFileAssociations` is the designated place for adding verbs to a file
type without touching the association.

**The letter submenu lists every letter, not just the free ones** — for the
reason above. Picking a busy letter produces a clear error, which is more
honest than a stale list.

**The unmount item is restricted to our volumes** by
`AppliesTo = System.Volume.FileSystem:ZipFS`. Without it, the item would hang
on every drive in the system.

Everything is written to `HKEY_CURRENT_USER` only: no administrator rights.

### How it works underneath

From the menu the drive is mounted in the background, so some bookkeeping is
needed: which letter belongs to which archive, and which process holds it. The
list lives in `%LOCALAPPDATA%\ZipMount\mounts.tsv` and is re-checked on every
read.

Stopping is done with a named event rather than by killing the process: the
mount process waits for either Ctrl+C or a signal from `unmount`, and in both
cases gets to release the volume cleanly.

The modern install copies the program to `C:\ProgramData\ZipMount\bin` and
points the package at that copy. The build directory will not do: File Explorer
holds the loaded library and will not let it be overwritten, and `cargo clean`
should not break the menu either. The library's filename carries a fingerprint
of its contents — no need to overwrite a file in use, a new build simply lands
beside it. **After rebuilding, run `zipmount shell-install --modern` again**,
or the menu will keep launching the previous copy.

## About search: why `grep` is a separate command

Content search **through the filesystem** (the "File contents" checkbox in
Explorer, `findstr /s`) is inevitably slow over gigabytes: the kernel drives
callbacks with small reads, files are processed one at a time, and the archive
is decompressed again for every search. That is a property of the approach, not
of the implementation.

So search is split across two levels:

* **by name** — instant, including in Explorer: the whole tree is in memory;
* **by content** — via `zipmount grep`, which works with the archive directly,
  in parallel, using the ripgrep engine.

### A pattern is a substring, not a regex

This deliberately breaks with grep habit, and here is why. On a real log
archive, searching for `[ERROR]` in regex mode matched **29,748 files out of
30,778**: as a regex that reads "any of the letters E, R, O". No error is
raised — you simply get plausible-looking nonsense. For strings like `[ERROR]`,
`app.config` or `GET /api/v1` this happens constantly.

So by default the pattern is matched literally, and regexes are opt-in:

```
zipmount grep archive.7z "[ERROR]"           # looks for literally [ERROR]
zipmount grep archive.7z "ERROR|WARN" --regex
```

### Search options

| Option | What it does |
|---|---|
| `-l` | paths only (already as `Z:\...`, ready to open) |
| `-c` | how many matches in each file |
| `-i` | ignore case |
| `-C N` / `-A N` / `-B N` | context lines around a match |
| `--max-count N` | at most N matches per file |
| `--max-total N` | at most N lines in the whole output |
| `--path <path>` | search only inside a branch of the archive |
| `--glob "*.log"` | restrict by filename pattern |
| `--copy-to <dir>` | extract every matched file at once |

Line output follows grep: a colon after the line number for the match itself,
a hyphen for a context line.

In 7z, `--path` and `--glob` save more than scanning — they save decompression:
blocks with no matching entry are never expanded at all. On the log archive,
restricting to one branch cuts the search from 2.23 s to 0.84 s, because one
solid block out of three gets expanded.

## Measurements

**zip**: 239 MB, 2 GB of text, 10,411 files, 16 cores.

| Operation | Time |
|---|---|
| Opening the archive (building the tree) | 0.02 s |
| Reading every file + CRC32 check | 0.32 s (6.5 GB/s) |
| `zipmount grep` over all contents | 0.28 s (7.2 GB/s) |
| `findstr /s` over the same, through the volume | 17.4 s |

**7z**: 152 MB, 5.63 GB of logs, 30,778 files, LZMA2, solid, 3 blocks.

| Operation | Time |
|---|---|
| Parsing the header | 0.067 s |
| `zipmount grep` over all 5.63 GB | 3.2 s (1792 MB/s) |
| `zipmount verify` (CRC32 of every entry) | 2.4 s |
| Walking the whole tree in Explorer | 1.0 s |
| Searching `*.xml` in Explorer (4301 files) | 0.3 s |
| First read of a file (expanding a solid block) | 1.35 s |
| Repeat read from the same block | 0.04 s |

**tar and tar.gz**: 1 GB of text, 4554 files.

| Operation | tar | tar.gz |
|---|---|---|
| Opening the archive | 0.014 s | 1.14 s (expanding the stream) |
| `zipmount grep` over all contents | 0.08 s | 0.02 s |
| Walking the tree in Explorer | 1.0 s | — |
| Searching `*.csv` in Explorer (1587 files) | 0.8 s | — |

Search in tar is that fast precisely because there is nothing to decompress:
contents are read as slices, and `grep` degenerates into a parallel scan of
memory.

## Five formats — three access models

The difference is not hidden behind a common interface, because behaviour
depends on it directly: open time, memory use, and which read order is cheap.

**zip**: every entry is compressed independently, so reading can be per-file.
There are three read paths — a single "decompress the whole entry" path would
break on a file larger than memory:

* `stored` — a slice of the memory-mapped file, no copying and no decompression;
* small `deflate` (up to 8 MB by default) — decompressed whole into an LRU cache
  with a byte budget, then served as slices from memory;
* large `deflate` — a streaming decoder with a checkpoint index: forward reads
  stream at constant memory, backward seeks jump to the nearest checkpoint.

### The checkpoint index: reading deflate backwards

Deflate cannot be read from an arbitrary position: decompressing each byte
depends on the previous 32 KB. The naive fix — restarting the stream from the
beginning — turns random access on a large entry into hundreds of megabytes of
wasted work.

The answer is the `zran` technique from the zlib examples: every few megabytes
it records enough state to resume — the position **in bits** inside the
compressed stream, plus 32 KB of already decompressed data as a dictionary.
Any seek then costs at most one step between checkpoints.

Measured on a single 315 MB entry, reading in random order in 64 KB blocks:

| | Time | Throughput |
|---|---|---|
| Restarting the stream | 245.93 s | 1 MB/s |
| Checkpoint index | **1.30 s** | 231 MB/s |

That is **189× faster**. Sequential reading was unaffected (1192 MB/s), and the
gap between random and sequential access dropped from one and a half orders of
magnitude to a factor of five.

Two things mattered more than they look:

**The span between checkpoints cannot be fixed.** It was set to 16 MB at first
— and on a 12 MB entry it produced not a single checkpoint, so the index was
empty and changed nothing. Then the span was derived from a target checkpoint
count, and on 315 MB every seek cost 9 MB. The policy that works ties it to
index memory: `size/span` checkpoints of 32 KB each, hence a span derived from
a 32 MB per-entry ceiling.

**The index is built lazily.** A pass costs one full decompression, so it only
starts once a backward read has actually happened — and only for entries larger
than 4 MB, where restarting is more expensive than the index.

Restoring the state requires `inflatePrime`, `inflateSetDictionary` and
`inflateReset2`, which convenient wrappers like `flate2` do not expose. They
come from `libz-rs-sys` — the same C-compatible zlib interface, but implemented
in Rust, so neither a C compiler nor an external library enters the build.

**7z**: entries are glued into solid blocks, and getting one file out means
expanding its block from the start. Hence two modes:

* random access (a mounted drive) — through a cache of expanded blocks. The
  cache has one special rule: **the most recent block is never evicted**, even
  if a single block exceeds the budget, otherwise every read would expand it
  again. The budget is set by `--cache-mb`, 6 GB by default;
* full scans (search, verification) — block by block as a stream, without the
  cache, at constant memory. For a solid archive this also happens to be the
  optimal read order, which is why `grep` over 5.6 GB finishes in seconds on
  modest memory.

Narrowing the search area (`--path`, `--glob`) saves decompression here, not
just scanning: blocks with no matching entry are never expanded at all.

Contrary to expectation, solid does not necessarily mean slow. On
well-compressible data the decoder reads few input bytes: in the measurements
above the log archive is compressed 37×, so a 2 GB block expands in about a
second.

**tar**: the happiest case of all. Each file's data sits in one contiguous
chunk, so reading is a slice of the memory-mapped archive: no decompression, no
copying, no cache. There is one price: the format has no directory at all, so
learning the contents means walking the whole chain of headers. But the walk
only touches the headers, skipping over data, so a gigabyte archive opens in
hundredths of a second.

**rar**: in access model the closest to 7z — an archive can be solid, and then
reading one file requires going through the entire preceding stream. Hence the
same two modes: random access through a cache (on a solid archive the very
first miss fills it completely, because one pass is cheaper than a pass per
file), and full scans in a single sequential pass. A plain, non-solid archive is
simpler: an entry is read by address, and skipping the previous ones does not
require decompressing them.

**tar.gz**: the same fork as 7z, taken to the limit — exactly one solid stream
for the whole archive. There are no entry points, so even producing a file list
expands the stream in full, once, at open time; after that the archive behaves
like an ordinary tar. Memory held equals the uncompressed size and is reported
by `zipmount info`; the ceiling is set by `--cache-mb`, and exceeding it aborts
the open honestly with a clear message instead of eating all memory.

This is the one place where the checkpoint index suggests itself but is not yet
applied: it would let the index be held in memory instead of the whole archive,
lifting the size limit on `.tar.gz`. The price would be losing the copy-free
slice reads that tar search speed currently rests on.

## Password-protected archives

All three schemes seen in practice are supported:

| Scheme | Where | How it is verified |
|---|---|---|
| WinZip AES-128/192/256 | zip, method 99 | HMAC-SHA1 over the ciphertext |
| ZipCrypto | zip, a 1990s legacy | header check byte |
| AES-256 | 7z, including an encrypted header (`-mhe=on`) | entry CRC32 |
| AES-256 | rar (in a rar build), including an encrypted header (`-hp`) | UnRAR checks it itself |

ZipCrypto is cryptographically weak and falls to a known-plaintext attack.
ZipMount only **reads** such archives and never creates them.

### How to supply a password

```
zipmount ls archive.7z -p                    # prompts with hidden input
type pw.txt | zipmount ls archive.7z --password-stdin
```

There is deliberately no `--password=VALUE` option. A password on the command
line settles into shell history and is visible in the process list to other
users of the system — not a price worth paying to save one keystroke.

`--password-stdin` reads **bytes**, not text: a password need not be valid
UTF-8. For passwords with non-ASCII characters `-p` is more reliable: there the
input encoding is known, while through a pipe it depends on the console code
page.

### Two things worth knowing

**WinZip AE-2 entries have no CRC32.** The header field is deliberately zeroed:
integrity there is confirmed by the HMAC computed during decryption, not by a
checksum. So `zipmount verify` reports such entries on a separate line, "no
checksum in the archive" — this is neither an error nor a skipped check:
corruption would be caught by the HMAC.

**A 7z header can be encrypted too** (7-Zip's `-mhe=on`). Then the password is
needed just to see the file list, not only to read contents.

## Filename encodings

This concerns zip only: in 7z names are always Unicode. If a zip was built
without the UTF-8 flag (which is what WinRAR does on a Russian Windows), names
are stored in CP866 or CP1251, and a naive implementation shows mojibake.
ZipMount detects the encoding heuristically and honours the Info-ZIP Unicode
Path extra field. Override with `--encoding auto|utf8|cp866|cp1251`.

## Layout

```
crates/zipfs-core/    archive parsing, tree, reading, search — no WinFsp
  entry.rs            format-independent entry description, format detection
  tree.rs             directory tree (node arena), knows nothing about formats
  zipbackend.rs       zip: mmap + central directory
  zipfmt.rs           zip structure parsing, names.rs — filename encodings
  sevenz.rs           7z: solid blocks, streaming walk, cache
  rarbackend.rs       rar: a wrapper over UnRAR, cache for solid archives (feature `rar`)
  tarfmt.rs           tar parsing: ustar, GNU longname, PAX
  tarbackend.rs       tar and tar.gz: copy-free slices
  blockcache.rs       cache of expanded solid blocks
  cache.rs            per-file LRU (zip)
  zipcrypt.rs         WinZip AES and ZipCrypto, secret.rs — password in memory
  deflate_index.rs    checkpoint index (zran) for reading deflate backwards
  reader.rs           read paths, search.rs / verify.rs — search and verification
crates/zipfs-mount/   the WinFsp layer (FileSystemContext implementation)
crates/zipmount/      CLI; main.rs — the commands that are the same everywhere
  windows/            mounting through WinFsp, drive letters, the File Explorer menu
    mounts.rs         mount bookkeeping, background launch, stopping
    shell.rs          context menu items via the registry
    modern.rs         building, signing and installing the package for the main menu
  unix/               mounting on a directory; linux.rs — FUSE, macos.rs — NFS,
                      menu.rs — the Finder, Dolphin and Nautilus items
crates/zipfs-fuse/    the FUSE layer (Linux)
crates/zipfs-nfs/     the NFS server and mount_nfs (macOS)
crates/zipmount-shell/  the IExplorerCommand COM handler (loaded by File Explorer)
crates/zipmount-i18n/ language choice and every user-facing text
  locales/*.ftl       the translations, one Fluent file per language
vendor/winfsp-sys/    patch: pre-generated bindings instead of bindgen
tools/                the icon build, test archive generator, the installer build
docs/                 the README key visual and icon
site/                 the website (GitHub Pages), one static page
installer/            the WiX authoring: the package, the bundle
  locales/<code>/     the installer window's texts and the page it shows
```

The core deliberately knows nothing about WinFsp: the most error-prone part is
debugged with ordinary tests and console commands, without the driver.

## Testing

```
python tools\make-test-archives.py          # test archives, including nasty ones
cargo test --workspace
zipmount verify testdata\big.zip --random   # check reads against the archive's CRC
```

The test archives deliberately contain what naive implementations break on:
Cyrillic in CP866 without the UTF-8 flag, missing parent directory entries,
duplicate names differing only in case, reserved device names (`CON.txt`),
characters invalid on Windows, zip-slip (`../../evil`), a `stored` entry, and an
entry larger than the materialisation threshold.

For tar both long-name formats are generated — GNU (the `L` block) and PAX
(`key=value`) — because that is exactly where parsing breaks: a long Cyrillic
path simply does not fit in the ustar fields.

## About rar: not in official builds

RAR is closed, and the only complete decompression implementation is RARLAB's
UnRAR sources. The alternatives were checked and rejected:

| Option | Why not |
|---|---|
| `rar` (native Rust, MIT) | only handles uncompressed RAR5 (`SAVE`), no RAR4 at all |
| `unrar-rs` (native, complete) | GPL-3.0-or-later when first checked; its latest versions carry a non-standard license |
| `unrar` (a wrapper over UnRAR) | **chosen**: complete support, wrapper under MIT |

The UnRAR terms allow using the code to read archives without restriction and
free of charge, but forbid building a compatible packer on its basis. ZipMount
only reads — but the restriction itself is the problem. The mounting layer
links the `winfsp` crate, which is plain GPLv3, and the GPL forbids adding
restrictions of this kind; WinFsp's own FLOSS exception separately requires
that the program not be linked or distributed with non-free code.

So rar is a cargo feature, **off by default**, and the published installers do
not contain UnRAR. A rar-enabled build is fine for your own use:

```
cargo build --release --features zipmount/rar
```

Distributing such a build is not. Without the feature, a rar archive is still
recognised by its signature and gets a clear error rather than "unknown
format", and the context menu does not offer itself on `.rar` files.

The feature also brings C++ into the build: UnRAR is the project's only
dependency that needs a C++ compiler. Elsewhere it is not needed even where it
would have been the obvious choice — the checkpoint index uses a zlib
implemented in Rust.

## Limitations

Read-only. Not supported: writing to an archive, multi-volume archives, nested
archives, and among compressed tars only gzip (not bz2/xz/zst).

**tar has no content checksums** — the header carries a checksum of the header
alone. So `zipmount verify` on a tar only checks that every entry reads in full
and stays within the archive bounds. For `tar.gz` the CRC32 of the whole
stream is added on top, verified during decompression.

**RAR5 deliberately corrupts the checksum** of files encrypted without header
encryption, so that a guessed password cannot be checked against it without
decrypting the data. `verify` reads such entries but does not compare, and says
so explicitly. RAR4 has no such trick; there the checksum is verified normally.

**RAR4 was only verified for reading.** Creating such an archive locally did not
work out: RAR 7.23 can read them but no longer create them. The check was done
on the samples shipped with the `unrar` crate.

One more untested corner: zip AES decryption with a non-ASCII password is
implemented to spec (password bytes in UTF-8) but was never checked against a
reference archive — 7-Zip refuses to create a zip with a Cyrillic password, and
no other source of such archives was at hand. For 7z the same case was checked
and works.

## Traps that cost debugging time

All of them are invisible until you test exactly that case — left here so as
not to step on them again.

**Generic rights in the security descriptor.** An ACE with `GR`/`GX`
(GENERIC_READ, GENERIC_EXECUTE) is not expanded into specific rights by the
kernel during an access check — the volume silently answers "access denied" to
everything. Specific file rights are required: `FR`/`FX`. See
`crates/zipfs-mount/src/security.rs`.

**Handle inheritance when launching a background process.** `mount --detach`
finished, but the script that called it hung forever. The cause: an ordinary
process launch hands the child every inheritable handle, and among them the
parent's output pipe. The parent died, the pipe stayed open, and the reading
side waited for end of stream. The cure is launching with
`bInheritHandles = FALSE`; the password then has to reach the background
process through a temporary file, which it deletes immediately, rather than
through a pipe. See `crates/zipmount/src/mounts.rs`.

**Process IDs get reused.** The mount list considered an entry alive if a
process with its ID was alive — and after a mount was killed, the entry "came
back to life" on an unrelated process that had taken the same number. Now an
entry counts as alive only when the process is alive **and** the drive exists.

**Package deployment does not work with `AppData`.** A signed `.msix` will not
install from there at all, and external content does install but then COM
cannot find the library. Both failures look identical — "path not found"
(0x80070003) — even though the files are in place and the signature is valid.
Checked for both `Local` and `Roaming`; `%TEMP%` works, despite living inside
`AppData`. So the package is built in a temporary directory and the program is
installed into `ProgramData`.

**GUIDs in a package manifest carry no braces.** The schema wants
`XXXXXXXX-XXXX-…` unadorned, and the familiar `{…}` form rejects the entire
package. The error comes from `makeappx`, so it is caught immediately — but the
table of identifiers lives next to the COM classes themselves, and a test
compares the manifest string against the constant: had they diverged, the menu
item would have vanished silently. See `crates/zipmount-shell/src/lib.rs`.

**A `SurrogateServer` class is not created as in-process.** A probing
`CoCreateInstance` with `CLSCTX_INPROC_SERVER` returns "class not registered"
even though registration is perfectly fine; `CLSCTX_ALL` is what is needed. The
error sends the investigation entirely the wrong way — towards the manifest and
package installation.

**`[INSTALLFOLDER]` ends with a backslash.** Handed to a custom action as
`--external "[INSTALLFOLDER]"`, that backslash escapes the closing quote and the
argument swallows whatever comes next. The install looked fine — files in place,
no error reported — while the menu silently never registered. Writing
`"[INSTALLFOLDER]."` moves the backslash away from the quote and changes nothing
about the path. See `installer/ZipMount.wxs`.

**A custom action's `HKEY_CURRENT_USER` is not reliably the user's.** The action
that registered the per-user package succeeded — the package really did land on
the right account — yet a registry value written by that same process moments
later never appeared in that user's hive. For a per-machine install `HKLM` is
the right place anyway, and Windows Installer writes it declaratively, which
also means it is removed on uninstall.

**A background menu item is handed no items — it is handed a site.** The unmount
item was declared for `Directory\Background` and simply never appeared.
`IExplorerCommand` gets an empty item array there, because nothing is selected,
so the check that hides the item outside our volumes found no path and hid it
everywhere. The open folder comes from the site the shell passes through
`IObjectWithSite`: cast it to `IServiceProvider`, ask for `IFolderView`, take
its folder. See `crates/zipmount-shell/src/command.rs`.

**File Explorer never lets go of a shell extension library.** Uninstalling
silently left the old one in place, and an upgrade would have put a new manifest
on top of old code. Windows Installer cannot replace a file that is in use, and
restarting Explorer does not help — it loads the library straight back, because
the desktop itself is a folder background. Cured the same way as in the manual
install: a fingerprint of the contents inside the filename, so a new build lands
beside the old one instead of over it.

**A process launched from the context menu arrives without `LOCALAPPDATA`.**
The mount list lives in that folder, so for such a process the list came back
empty and nothing was written back either: a drive mounted from the menu could
not then be unmounted from the menu — "not listed as mounted", with the drive
right there. The folder is now asked of the system through
`SHGetKnownFolderPath`, which does not depend on the environment, and unmounting
a drive letter no longer needs the list at all: the stop event's name follows
from the letter alone. See `crates/zipmount/src/mounts.rs`.

**The menu group's own row needs the package logo, under a very particular
name.** Windows collects several verbs of one app into a submenu titled with
the app name, and draws that row itself — the icon for it comes from the
package, not from the handler. A plain `Square44x44Logo.png` is not enough:
the shell looks for an unplated variant sized for the row, by filename, as
`Square44x44Logo.altform-unplated_targetsize-32.png` and friends. Without
those files the row stays blank while every item under it has an icon.

**An unchanging version turns every install into another entry.** Windows
Installer gives each build its own product code but compares versions, so with
a version that never moved, a reinstall was not an upgrade — it was a second
product beside the first. Seven ZipMount rows had piled up in "Installed apps"
before anyone looked, and none of them could be removed by the current `.msi`
file, because that file's product code no longer matched any of them. The
version now grows with every build — see "Building the installers".

**A terminating NUL inside the name length.** The `set_name` method from the
`winfsp` crate appends `\0` and counts it in the name length, so the kernel sees
the name as `file.log\0`. Prefix patterns (`file*`) still match, but anything
anchored at the end (`*.log`) no longer does. Worked around with `set_name_raw`
without the NUL. See `crates/zipfs-mount/src/fs.rs`, `read_directory`.

**Fluent wraps every substituted value in invisible marks.** By default it puts
Unicode isolation characters (U+2068, U+2069) around each placeable, to keep
right-to-left text in order. A console prints them as junk around every path
and number. None of our languages is written right to left, so isolation is
switched off, and a test makes sure it stays off. See
`crates/zipmount-i18n/src/lib.rs`.

**Recognising an error by its text breaks the moment the text is translated.**
The search window from the context menu asks for a password when opening the
archive fails for want of one — and it used to tell by finding the word
"пароль" in the message. Now a password problem is a type, `PasswordError`,
found through the whole chain of error context, whatever the language.

**The text catalogue runs inside File Explorer.** The context menu handler
formats its titles through the same crate as the command line, so that crate
must not panic — with `panic = "abort"` a panic takes all of File Explorer
down — and cannot fix the language once per process: File Explorer keeps the
library loaded for hours, while `zipmount language` may change the choice in
the meantime. The state sits behind a lock and is decided again before every
menu is built.

**WiX's standard installer window speaks only English out of the box.** Its
bootstrapper looks for `<LCID>\thm.wxl` and `<LCID>\license.rtf` next to itself
and falls back to the built-in English, so every translation is a pair of
extra payloads per LCID. Two details cost a rebuild each: WiX derives a
payload's id from its source file, so one translation serving several LCIDs
needs explicit ids; and RTF takes `\u` codes as signed 16-bit numbers, so
everything above U+7FFF — all of Korean Hangul — must be written negative, or
it comes out as garbage.

## License

[GPL-3.0-or-later](LICENSE). Not by preference but by fact: the mounting layer
links the `winfsp` crate, which is plain GPLv3, so every build of
`zipmount.exe` is a GPLv3 work anyway. What else is inside and under which
terms is in [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md); how to report a
security problem is in [SECURITY.md](SECURITY.md).

Mounting goes through **WinFsp - Windows File System Proxy, Copyright (C) Bill
Zissimopoulos** — <https://github.com/winfsp/winfsp>.
