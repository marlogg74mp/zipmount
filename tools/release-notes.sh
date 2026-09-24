#!/bin/sh
# Writes the release notes for the files in a directory, to stdout.
#
#     tools/release-notes.sh <version> <directory>
#
# The release workflow runs it on everything it is about to publish, so the
# table lists exactly what is attached, with real sizes.
set -eu

version=$1
dir=$2
repo=${GITHUB_REPOSITORY:-marlogg74mp/zipmount}

size() {
    # Bytes to megabytes with two decimals, without depending on bc.
    awk -v b="$(wc -c < "$1")" 'BEGIN { printf "%.2f MB", b / 1048576 }'
}

row() {
    file=$1
    use=$2
    [ -f "$dir/$file" ] && printf '| `%s` | %s | %s |\n' "$file" "$(size "$dir/$file")" "$use"
    return 0
}

cat <<EOF
Mounts zip, 7z, tar and tar.gz archives as ordinary drives and folders — read-only, without extracting — on Windows, Linux and macOS. Speaks English, Russian, Chinese, Japanese, Korean, Portuguese, Spanish and German.

| File | Size | For |
|---|---|---|
EOF
row "ZipMount-Setup-$version.exe" "Windows 10/11, 64-bit: the regular install, brings WinFsp if it is missing"
row "ZipMount-$version.msi" "Windows, when WinFsp is already there or for group policy"
row "zipmount-$version-linux-x86_64.tar.gz" "Linux, x86_64 — a static binary for any distribution"
row "zipmount-$version-linux-arm64.tar.gz" "Linux, ARM64 — a static binary for any distribution"
row "zipmount-$version-macos-arm64.tar.gz" "macOS 26, Apple Silicon"

cat <<EOF

**Windows**: run the installer. It is not code-signed yet, so Windows warns about an unknown publisher.

**Linux**: mounting needs FUSE (\`sudo apt install fuse3\` or your distribution's equivalent).

\`\`\`
tar -xzf zipmount-$version-linux-x86_64.tar.gz
sudo install zipmount-$version-linux-x86_64/zipmount /usr/local/bin/
zipmount shell-install      # "Mount with ZipMount" in the file manager
\`\`\`

**macOS**: easiest through Homebrew, which builds it on your Mac — nothing for Gatekeeper to object to:

\`\`\`
brew install marlogg74mp/tap/zipmount
zipmount shell-install      # "Mount with ZipMount" in Finder's Quick Actions
\`\`\`

The binary in the archive is not signed by Apple: macOS refuses to start it until you allow it in System Settings → Privacy & Security, or remove the quarantine with \`xattr -d com.apple.quarantine zipmount\`.

**Checking a download**: compare it with \`SHA256SUMS.txt\`, or have GitHub confirm it was built by this repository's release workflow from the tagged commit:

\`\`\`
gh attestation verify <file> --repo $repo
\`\`\`

More in the [README](https://github.com/$repo#installation).
EOF
