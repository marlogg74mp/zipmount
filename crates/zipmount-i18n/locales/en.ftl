# ZipMount — English. The reference language: every other file carries exactly
# the same set of messages. Plurals use CLDR categories: one, other.
#
# Keep "WinFsp - Windows File System Proxy, Copyright (C) Bill Zissimopoulos"
# as is in every language: WinFsp's license requires that exact notice.

## Numbers and units

decimal-separator = .
unit-b = B
unit-kb = KB
unit-mb = MB
unit-gb = GB
unit-tb = TB
seconds = { $value } s

## Opening archives

core-open-failed = cannot open archive { $path }
core-unknown-format = cannot tell the format of { $path }: it is not zip, 7z or tar
core-rar-not-built = { $path } is a RAR archive, and this build has no rar support: the UnRAR license is incompatible with the GPL, so rar is only enabled when building from source for your own use (cargo build --release --features zipmount/rar)
core-unknown-encoding = unknown encoding "{ $value }"; valid: auto, utf8, cp866, cp1251
core-deflate-corrupt = deflate data is damaged (zlib code { $code })
core-password-missing = the archive is encrypted: a password is needed (-p or --password-stdin)
core-password-wrong = wrong password

## Reading archives

core-mmap-failed = cannot map the archive into memory
core-entry-out-of-bounds = the entry's data runs past the end of the archive
core-zip-structure = cannot parse the structure of the zip archive
core-zip-too-small = the file is too small for a zip: { $size } bytes
core-zip-no-eocd = no end-of-central-directory record — the file is not a zip archive, or it is damaged
core-zip64-missing = the archive is marked as zip64, but its zip64 end record is missing
core-zip-cd-out-of-bounds = the central directory runs past the end of the file
core-zip-bad-local-header = damaged local header at offset { $offset }
core-zip-method = unsupported compression method { $method } (stored and deflate are supported)
core-deflate-failed = deflate decompression error: { $error }
core-aes-unknown-strength = unknown AES strength in the entry: { $strength }
core-encrypted-too-short = the encrypted entry is shorter than its own header fields
core-aes-auth-failed = the authentication code does not match: the entry is damaged
core-7z-structure = cannot parse the structure of the 7z archive { $path }; if it is password-protected, give the password with -p
core-7z-structure-password = cannot parse the structure of the 7z archive { $path }: the password may be wrong, or the archive damaged
core-7z-block-failed = cannot expand block { $block }
core-7z-block-error = error decompressing block { $block }: { $error }
core-tar-structure = cannot parse the structure of the tar archive
core-tar-bad-header = the first tar header fails its checksum
core-gzip-not-tar = this is a gzip with no tar inside — such an archive is not supported
core-gzip-damaged = gzip decompression error (the archive is damaged or truncated)
core-targz-too-large = the expanded tar.gz does not fit in the memory allowed: { $used } GB so far, with a ceiling of { $limit } GB. Raise the ceiling with --cache-mb
core-rar-error = error reading the RAR archive: { $error }
core-grep-substring = cannot build a substring search for: { $pattern }
core-grep-regex = invalid regular expression: { $pattern }
core-grep-block-errors = errors while walking blocks: { $errors }
core-verify-short-read = a read at { $offset } returned { $got } bytes instead of { $want }
core-verify-crc = CRC32 mismatch: got { $actual }, expected { $expected }
core-verify-block = <block { $block }>

## Mounting through WinFsp

mount-err-create = cannot create the WinFsp volume: { $error }
mount-err-mount = cannot mount at { $mountpoint }: { $error }
mount-err-dispatcher = cannot start the WinFsp dispatcher: { $error }
mount-err-no-winfsp = WinFsp not found. Install it with:  winget install WinFsp.WinFsp

## Common errors

error-prefix = Error
err-create-dir = cannot create { $path }
err-write = cannot write { $path }
err-read = cannot read { $path }
err-read-password = cannot read the password
err-read-password-stdin = cannot read the password from standard input
err-spawn-background = cannot start the background process
err-run-failed = cannot start { $tool }
err-tool-failed =
    { $tool } failed:
    { $output }

## Command-line help

help-about = An archive as a Windows drive: browse, search and extract without unpacking
help-notice =
    License GPL-3.0-or-later, source: https://github.com/marlogg74mp/zipmount

    Mounting goes through WinFsp - Windows File System Proxy, Copyright (C) Bill Zissimopoulos — https://github.com/winfsp/winfsp
help-heading-usage = Usage:
help-heading-commands = Commands
help-heading-arguments = Arguments
help-heading-options = Options
help-flag-help = Print help
help-flag-version = Print version
help-archive = Path to the archive
help-encoding = Encoding of names in zip: auto, utf8, cp866, cp1251
help-password = Ask for the archive password (hidden input)
help-password-stdin = Read the password from standard input (the first line)
help-prefix = Path prefix in the output, for example "Z:\"
help-mount = Mount an archive as a drive
help-mount-mountpoint = Drive letter (Z:) or path to an empty NTFS directory; the first free letter if omitted
help-mount-detach = Mount in the background and return at once
help-mount-open = Open the mounted drive in File Explorer
help-mount-label = Volume label; the archive's file name by default
help-mount-cache-mb = Ceiling for the cache of decompressed data, MB (for 7z, the solid block cache)
help-ls = List a directory inside the archive
help-ls-path = Path inside the archive; the root by default
help-ls-recursive = Recursively, the whole tree
help-find = Find files by a name pattern
help-find-pattern = Pattern, for example "*.log"
help-grep = Search the contents of files inside the archive
help-grep-pattern = Substring to look for (or a regular expression with --regex)
help-grep-regex = Treat the pattern as a regular expression
help-grep-files-only = Only file paths, no lines
help-grep-count = Only the number of matches in each file
help-grep-ignore-case = Ignore case
help-grep-after-context = Lines of context after a match
help-grep-before-context = Lines of context before a match
help-grep-context = Lines of context on both sides
help-grep-max-count = At most N matches per file
help-grep-max-total = At most N lines in the whole output
help-grep-path = Search only inside this branch of the archive
help-grep-glob = Restrict by a name pattern, for example "*.log"
help-grep-copy-to = Extract the matching files into this directory
help-info = Summary of an archive
help-verify = End-to-end read check of every entry against its CRC32
help-verify-random = Read in shuffled order rather than sequentially: tests seeking backwards
help-unmount = Unmount a drive
help-unmount-target = Drive letter (Z:) or path to the archive
help-mounts = Show mounted archives
help-search = Interactive search in an archive (started from the context menu)
help-shell-install = Add items to the File Explorer context menu
help-shell-install-modern = The main Windows 11 menu, without "Show more options" (asks for administrator rights once)
help-shell-uninstall = Remove the items from the context menu
help-language = Show or choose the program's language
help-language-code = Language code (en, ru, zh-CN, ja, ko, pt-BR, es, de), or auto to follow Windows
help-doctor = Check that the environment is ready for mounting

## ls, find, grep

password-prompt = Archive password:
err-path-not-found = path not found in the archive: { $path }
find-summary = found: { $count }
grep-copied = files extracted: { $count } -> { $path }
grep-summary = files with matches: { $matched } | scanned: { $scanned } | skipped: { $skipped } | decompressed: { $size } in { $seconds } s ({ $speed } MB/s)
grep-truncated = output cut at the limit (--max-total)
grep-regex-note = the pattern was taken as a regular expression (--regex)

## info

info-archive = archive:
info-format = format:
info-size = archive size:
info-files = files:
info-dirs = directories:
info-uncompressed = uncompressed:
info-ratio = compression ratio:
info-nodes = tree nodes:
info-in-memory = in memory:
info-in-memory-value = { $size } (tar.gz is expanded in full when opened)
info-solid = solid:
info-rar-solid-yes = yes (reading one file means going through the whole preceding stream)
info-7z-solid-yes = yes (reading one file expands its whole block)
info-solid-no = no (every file is decompressed independently)
info-headers = table of contents:
info-headers-encrypted = encrypted
info-blocks = solid blocks:
info-encrypted = encrypted:
info-encrypted-value =
    { $count ->
        [one] { $count } entry
       *[other] { $count } entries
    }
info-parse-time = parsing took:

## verify

verify-failure = ERROR  { $path }: { $reason }
verify-summary = files checked: { $ok } | errors: { $errors } | read { $size } in { $seconds } s ({ $speed } MB/s)
verify-random = shuffled access
verify-no-checksum =
    { $count ->
        [one] read without a checksum to compare against: { $count } entry
       *[other] read without a checksum to compare against: { $count } entries
    }
verify-no-checksum-tar =
    tar keeps no checksums of contents — there is nothing to compare against.
    What was checked: every entry reads in full and stays within the archive.
verify-no-checksum-targz =
    tar keeps no checksums of contents, but the CRC32 of the whole gzip stream
    was checked during decompression — damage to the archive would have shown.
verify-no-checksum-rar =
    these are RAR5 entries encrypted without an encrypted header: the format
    garbles their checksum on purpose, so it cannot be used to guess the
    password. There is nothing to compare against — the data is still correct.
verify-no-checksum-zip =
    these are WinZip AE-2 entries: their CRC field is empty on purpose, and
    integrity is confirmed by the HMAC checked during decryption.
err-verify-failed =
    { $count ->
        [one] check failed: { $count } entry
       *[other] check failed: { $count } entries
    }

## mount, unmount, mounts

mount-already = Already mounted: { $letter }
mount-done = Mounted: { $letter }
mount-done-stats =
    Mounted: { $letter }  ({ $files ->
        [one] { $files } file
       *[other] { $files } files
    }, { $dirs ->
        [one] { $dirs } directory
       *[other] { $dirs } directories
    }, { $size } of contents)
mount-parse-time = Parsing the archive took { $seconds } s.
mount-stop-hint = Ctrl+C or `zipmount unmount { $letter }` to unmount.
mount-unmounting = Unmounting...
mount-finished = Done. The archive is unchanged.
err-no-free-letters = no free drive letters
err-spawn-password = cannot pass the password to the background process
err-mount-failed-detached =
    cannot mount { $path }.
    Run without --detach to see why.
err-letter-busy =
    drive letter { $letter } is taken by another drive.
    Free: { $free }
letters-none = no free letters
unmount-done = Unmounted: { $letter }
err-unmount-not-responding = the mounting process of { $letter } does not respond; its record was removed from the list
err-unmount-timeout = { $letter } did not unmount in time: files on it may still be open
err-not-mounted = { $target } is not listed as mounted. List: zipmount mounts
mounts-none = Nothing is mounted.

## Search from the context menu

search-archive = Archive: { $path }
search-stats =
    { $files ->
        [one] { $files } file
       *[other] { $files } files
    }, { $size } uncompressed. Format: { $format }.
search-intro = The search goes through file contents. An empty line quits.
search-prompt = Search for:
search-summary =
    files with matches: { $matched } of { $scanned } scanned, in { $seconds } s
search-truncated = (output cut)
search-error = Search error: { $error }
press-enter = Press Enter to close...

## Context menu items — also written into the registry

menu-mount = Mount as a drive
menu-mount-as = Mount to a letter
menu-search = Search in archive…
menu-unmount = Unmount (ZipMount)
menu-unmount-letter = Unmount { $letter }: (ZipMount)

## shell-install, shell-uninstall

shell-installed = Items added to the context menu for: { $extensions }
shell-installed-items =
    { menu-mount } — a free letter, opened in File Explorer
    { menu-mount-as } — a submenu to choose from
    { menu-search } — search by content
    { menu-unmount } — in the menu of the mounted drive itself
shell-installed-where =
    In Windows 11 these items live under "Show more options"
    — or straight away with Shift+right-click.
shell-modern-hint = The main menu without Shift: zipmount shell-install --modern
shell-remove-hint = To remove: zipmount shell-uninstall
shell-removed = Items removed from the context menu.

## The modern menu

modern-building = Building the package…
modern-trust-intro =
    One step with administrator rights is left.

    Windows gives a place in the main menu only to a signed package, and
    trusting a certificate is a machine-wide decision, hence the elevation
    prompt. The certificate is self-signed and lives here:
modern-trust-uac = A User Account Control window will appear now.
modern-registering = Installing the package…
modern-done =
    Done. Items in the main context menu:

      { menu-mount } — on an archive
      { menu-mount-as } — a submenu of free letters only
      { menu-search } — search by content
      { menu-unmount } — on empty space inside the drive
modern-installed-to = The program is installed in { $path }
modern-rebuild-hint = After rebuilding, run again: zipmount shell-install --modern
modern-files-left = The files stay in { $path }
modern-cert-left = The certificate stays trusted. To remove it (needs an administrator):
err-sdk-tool-missing =
    { $tool } not found. It comes with the Windows SDK — install it,
    for example: winget install Microsoft.WindowsSDK.10.0.26100
err-exe-busy =
    cannot update { $path }: the file is in use.
    Unmount the drives (zipmount unmount …) and try again.
err-shell-dll-missing =
    zipmount_shell.dll is not next to the program.
    Build it: cargo build --release
err-bin-dir =
    cannot create { $path }.
    If the directory exists already, it may be left over from another account —
    then delete or rename it.
err-cert-not-trusted =
    the certificate never became trusted.
    Windows will not accept the package without it. The registry menu works
    without an administrator: zipmount shell-install
err-handler-create = the handler's COM class cannot be created
err-handler-title = the handler returned no title

## language

language-current = Language: { $name } ({ $code }) — { $source }
language-source-environment = set by ZIPMOUNT_LANG
language-source-saved = chosen with zipmount language
language-source-windows = the Windows display language
language-source-default = the default
language-available = Available:
language-hint = Choose one: zipmount language <code>. Follow Windows again: zipmount language auto
language-set = Language: { $name } ({ $code }).
language-follows-windows = The language follows Windows again: { $name } ({ $code }).
language-menu-updated =
    { $count ->
        [0] The registry holds no menu items to rewrite; the modern menu picks the language up by itself.
        [one] One kind of registry menu item was rewritten in this language; the modern menu picks it up by itself.
       *[other] { $count } kinds of registry menu items were rewritten in this language; the modern menu picks it up by itself.
    }
language-env-overrides = Note: ZIPMOUNT_LANG={ $value } is set, and in this console it still decides the language.
err-language-unknown = unknown language "{ $code }". Available: { $available }; or auto to follow Windows
err-language-save = cannot save the language choice

## doctor

doctor-winfsp = WinFsp library:
doctor-winfsp-found = found ({ $path })
doctor-winfsp-missing = not found
doctor-rar = rar support:
doctor-rar-yes = yes (a personal build with --features rar; it must not be distributed)
doctor-rar-no = no (official build: UnRAR is incompatible with the GPL)
doctor-language = Language:
doctor-modern = Modern menu:
doctor-modern-ok = works ("{ $title }")
doctor-modern-silent = package present, the handler is silent — { $error }
doctor-modern-missing = not installed (zipmount shell-install --modern)
doctor-init = Initialization:
doctor-init-ok = succeeded
doctor-init-failed = FAILED
doctor-ready =
    All set. Mount an archive:
        zipmount mount <archive.zip> Z:
doctor-reason = Reason: { $error }
doctor-no-winfsp-note =
    Browsing, search and extraction (ls, find, grep, verify) work without
    WinFsp too — the driver is only needed for mounting a drive.

## Linux and macOS: no drive letters, no File Explorer

help-about-unix = An archive as a folder: browse, search and extract without unpacking
help-notice-unix = License GPL-3.0-or-later, source: https://github.com/marlogg74mp/zipmount
help-prefix-unix = Path prefix in the output, for example "/home/me/ZipMount/logs/"
help-mount-mountpoint-unix = An empty directory to mount on; ~/ZipMount/<archive name> if omitted
help-mount-open-unix = Open the mounted folder in the file manager
help-unmount-unix = Unmount an archive
help-unmount-target-unix = The mount directory or the path to the archive
help-language-code-unix = Language code (en, ru, zh-CN, ja, ko, pt-BR, es, de), or auto to follow the system
language-source-system = the system locale
language-hint-unix = Choose one: zipmount language <code>. Follow the system again: zipmount language auto
language-follows-system = The language follows the system again: { $name } ({ $code }).
err-language-unknown-unix = unknown language "{ $code }". Available: { $available }; or auto to follow the system
mount-err-fuse = cannot mount on { $mountpoint }: { $error }
err-no-fuse = FUSE is not available here: { $missing } is missing. Install the fuse3 package, for example:  sudo apt install fuse3
err-mountpoint-not-dir = { $path } is not a directory
err-mountpoint-not-empty = { $path } is not empty; mount on an empty directory
err-unmount-failed = cannot unmount { $target }: { $error }
err-unmount-in-use = cannot unmount { $target }: files on it are open in { $programs }. Close them and try again
err-mount-unsupported = mounting is not available on this system yet
doctor-fuse = FUSE:
doctor-fuse-ok = available (/dev/fuse, fusermount3)
mount-err-nfs = cannot mount on { $mountpoint }: { $error }
doctor-nfs-ok = available (the system's NFS client)
doctor-fuse-note =
    Browsing, search and extraction (ls, find, grep, verify) work without
    FUSE too — it is only needed for mounting. Install it with the package
    manager: sudo apt install fuse3 (Debian, Ubuntu), sudo dnf install fuse3
    (Fedora), sudo pacman -S fuse3 (Arch).
doctor-mount = Mounting:
doctor-mount-unsupported-note =
    Browsing, search and extraction (ls, find, grep, verify) work on this
    system; mounting is not available here yet.
doctor-ready-unix =
    All set. Mount an archive:
        zipmount mount <archive.zip>

## File manager menu on Linux and macOS

menu-mount-unix = Mount with ZipMount
help-shell-install-unix = Add "Mount with ZipMount" to the file manager's context menu
shell-installed-unix = Menu items installed:
shell-installed-where-macos = In Finder: right-click an archive → Quick Actions → { menu-mount-unix }.
shell-installed-where-linux =
    GNOME Files: right-click an archive → Scripts → { menu-mount-unix }.
    Dolphin (KDE): right-click an archive → { menu-mount-unix }.
    Any file manager: Open With → { menu-mount-unix }.
language-menu-updated-unix = The file manager menu items were rewritten in this language.
