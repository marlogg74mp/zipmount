# Security

ZipMount parses archives that may come from anywhere, and serves their
contents to every program on the machine through a mounted volume. Bugs in
that path — a crafted archive that crashes the mount process, escapes the
volume with a path like `../../evil`, exhausts memory, or reads outside an
entry's bounds — are security bugs.

## Reporting

Please **do not open a public issue.** Use GitHub's private reporting instead:
the **Security** tab of this repository → **Report a vulnerability**.

Include the archive that triggers the problem if you can (or the script that
produces it), the ZipMount version (`zipmount --version`), and what happened.

This is a personal project maintained in spare time: expect an answer within a
couple of weeks, not hours.

## Supported versions

Only the latest release receives fixes.

## Scope notes

- The volume is read-only by design; nothing ZipMount does writes to the
  archive.
- Passwords are never accepted on the command line (`-p` prompts, and
  `--password-stdin` reads a pipe), so they do not end up in shell history or
  the process list.
- The context-menu installer places a self-signed certificate in
  `LocalMachine\TrustedPeople` so Windows accepts the menu package. The
  certificate can only sign code (EKU 1.3.6.1.5.5.7.3.3) and is removed on
  uninstall. For released installers it is created on the GitHub Actions
  runner that builds the release, and its private key is discarded with that
  runner: no key exists anywhere that could sign another package trusted by
  your machine.
- Release files carry a build provenance attestation. `gh attestation verify
  <file> --repo marlogg74mp/zipmount` confirms that the file was built by this
  repository's release workflow, from the tagged commit.
