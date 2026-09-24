#!/usr/bin/env python3
"""Generates test archives for ZipMount.

Two kinds of archives:

* "nasty" ones — small, but full of the edge cases naive implementations
  break on: Cyrillic in CP866 without the UTF-8 flag, missing parent
  directory entries, duplicate names differing only in case, reserved device
  names, zip-slip, a stored entry, and an entry above the materialization
  threshold;
* a "big" one — many text files expanding to gigabytes, for measuring search
  and checking the memory ceiling.

The names and the marker are Cyrillic on purpose: they are what is tested.
"""

import argparse
import contextlib
import io
import os
import random
import tarfile
import zipfile

MARKER = "МАРКЕР-ИГОЛКА-7F3A"
COMMON_WORD = "исключение"


@contextlib.contextmanager
def cp866_names():
    """Makes zipfile write names in CP866 without the UTF-8 flag.

    Exactly what WinRAR does on a Russian Windows: the name is in a single-byte
    encoding and bit 11 is not set. A reader that blindly decodes UTF-8 shows
    mojibake instead of the tree.
    """
    original = zipfile.ZipInfo._encodeFilenameFlags

    def encode(self):
        try:
            return self.filename.encode("ascii"), self.flag_bits
        except UnicodeEncodeError:
            return self.filename.encode("cp866"), self.flag_bits & ~0x800

    zipfile.ZipInfo._encodeFilenameFlags = encode
    try:
        yield
    finally:
        zipfile.ZipInfo._encodeFilenameFlags = original


def tricky_entries(big_entry_bytes):
    """Contents of the nasty archive: (name, data, compression method)."""
    return [
        ("Документы/отчёт за 2024.txt", f"обычный текст, {COMMON_WORD}\n".encode("utf-8"), zipfile.ZIP_DEFLATED),
        ("Документы/Отчёт За 2024.txt", b"otlichaetsya tolko registrom\n", zipfile.ZIP_DEFLATED),
        # Not a single entry for the intermediate directories — they must be
        # synthesized.
        ("a/b/c/d/e/f/deep.txt", f"{MARKER} lezhit gluboko\n".encode("utf-8"), zipfile.ZIP_DEFLATED),
        # An explicit entry for an empty directory.
        ("empty/", b"", zipfile.ZIP_STORED),
        # Names Windows does not allow.
        ("bad/na:me*.txt", b"invalid chars\n", zipfile.ZIP_DEFLATED),
        ("bad/CON.txt", b"reserved device name\n", zipfile.ZIP_DEFLATED),
        ("bad/trailing .txt", b"trailing space in stem\n", zipfile.ZIP_DEFLATED),
        ("bad/dots...", b"trailing dots\n", zipfile.ZIP_DEFLATED),
        # The classic zip-slip: must not lead a node outside the root.
        ("../../evil.txt", b"should stay inside root\n", zipfile.ZIP_DEFLATED),
        # A stored entry — read as a slice of the mmap, no decompression.
        ("stored/plain.bin", bytes(range(256)) * 512, zipfile.ZIP_STORED),
        # Above the materialization threshold: tests the streaming read path.
        ("big/large.txt", make_text(big_entry_bytes, seed=7).encode("utf-8"), zipfile.ZIP_DEFLATED),
    ]


def make_text(size_bytes, seed):
    """Pseudo-text of a given size with plausible compressibility."""
    rnd = random.Random(seed)
    words = [
        "система", "запрос", "ответ", "ошибка", COMMON_WORD, "модуль", "поток",
        "файл", "каталог", "индекс", "значение", "параметр", "сервер", "клиент",
        "connection", "timeout", "buffer", "request", "handler", "session",
    ]
    chunks = []
    total = 0
    line_no = 0
    while total < size_bytes:
        line = "%06d %s" % (line_no, " ".join(rnd.choice(words) for _ in range(rnd.randint(6, 14))))
        chunks.append(line)
        total += len(line.encode("utf-8")) + 1
        line_no += 1
    return "\n".join(chunks)


def write_tricky(path, encoding, big_entry_bytes):
    entries = tricky_entries(big_entry_bytes)
    ctx = cp866_names() if encoding == "cp866" else contextlib.nullcontext()
    with ctx, zipfile.ZipFile(path, "w", zipfile.ZIP_DEFLATED, compresslevel=6) as z:
        for name, data, method in entries:
            info = zipfile.ZipInfo(name, date_time=(2024, 3, 15, 14, 30, 10))
            info.compress_type = method
            if name.endswith("/"):
                info.external_attr = 0x10
            z.writestr(info, data)
    return path


def write_big(path, target_bytes, file_count, seed=42):
    """Many text files in a directory tree.

    `MARKER` goes into exactly five files with known paths — it shows that the
    parallel search neither loses nor invents matches.
    """
    rnd = random.Random(seed)
    # The corpus is reused in slices: generating gigabytes character by
    # character in Python takes too long, and 32 KB deflate windows cannot see
    # repeats across files anyway.
    corpus = make_text(24 * 1024 * 1024, seed=seed)
    per_file = max(4096, target_bytes // file_count)

    marker_files = set(rnd.sample(range(file_count), 5))
    written = 0
    marker_paths = []

    with zipfile.ZipFile(path, "w", zipfile.ZIP_DEFLATED, compresslevel=6) as z:
        for i in range(file_count):
            depth_dirs = "/".join("dir%02d" % rnd.randint(0, 20) for _ in range(rnd.randint(1, 4)))
            ext = rnd.choice([".log", ".txt", ".log", ".csv"])
            name = "%s/file%05d%s" % (depth_dirs, i, ext)

            size = max(1024, int(per_file * rnd.uniform(0.3, 2.2)))
            start = rnd.randint(0, max(0, len(corpus) - size - 1))
            body = corpus[start:start + size]

            if i in marker_files:
                body = body + "\n" + MARKER + " найден в файле %d\n" % i
                marker_paths.append(name)

            data = body.encode("utf-8")
            info = zipfile.ZipInfo(name, date_time=(2024, 6, 1, 12, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            z.writestr(info, data)
            written += len(data)

            if written >= target_bytes:
                break

    return written, sorted(marker_paths)


def write_tar(path, tar_format, big_entry_bytes):
    """A nasty tar in a given format.

    The formats differ in how long and non-ASCII names are stored: ustar is
    limited by the header fields, GNU puts the name in a separate `L` block,
    PAX in `key=value` records. Naive parsing breaks on the last two.
    """
    long_name = "очень/длинный/путь/" * 8 + "файл.txt"
    items = [
        ("Документы/отчёт за 2024.txt", ("обычный текст, " + COMMON_WORD + "\n").encode("utf-8")),
        ("a/b/c/d/e/f/deep.txt", (MARKER + " lezhit gluboko\n").encode("utf-8")),
        (long_name, b"very long path\n"),
        ("stored/plain.bin", bytes(range(256)) * 512),
        ("big/large.txt", make_text(big_entry_bytes, seed=11).encode("utf-8")),
    ]

    with tarfile.open(path, "w", format=tar_format) as t:
        # An explicit directory entry: the intermediate ones are still missing.
        d = tarfile.TarInfo("empty")
        d.type = tarfile.DIRTYPE
        d.mtime = 1700000000
        t.addfile(d)

        for name, data in items:
            info = tarfile.TarInfo(name)
            info.size = len(data)
            info.mtime = 1700000000
            t.addfile(info, io.BytesIO(data))

    return path


def write_tar_gz(src_tar, dst):
    """The same tar, gzipped: one stream without entry points."""
    import gzip
    import shutil

    with open(src_tar, "rb") as fin, gzip.open(dst, "wb", compresslevel=6) as fout:
        shutil.copyfileobj(fin, fout, length=1 << 20)
    return dst


def write_big_tar_gz(path, target_bytes, file_count, seed=99):
    """Many text files in one compressed stream — for measurements."""
    rnd = random.Random(seed)
    corpus = make_text(24 * 1024 * 1024, seed=seed)
    per_file = max(4096, target_bytes // file_count)
    written = 0

    import gzip

    with gzip.open(path, "wb", compresslevel=6) as raw:
        with tarfile.open(fileobj=raw, mode="w|", format=tarfile.GNU_FORMAT) as t:
            for i in range(file_count):
                depth = "/".join("dir%02d" % rnd.randint(0, 20) for _ in range(rnd.randint(1, 3)))
                name = "%s/file%05d%s" % (depth, i, rnd.choice([".log", ".txt", ".csv"]))
                size = max(1024, int(per_file * rnd.uniform(0.3, 2.0)))
                start = rnd.randint(0, max(0, len(corpus) - size - 1))
                body = corpus[start:start + size]
                if i == file_count // 2:
                    body += "\n" + MARKER + " найден\n"
                data = body.encode("utf-8")

                info = tarfile.TarInfo(name)
                info.size = len(data)
                info.mtime = 1700000000
                t.addfile(info, io.BytesIO(data))
                written += len(data)
                if written >= target_bytes:
                    break
    return written


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--out", default="testdata", help="directory for the archives")
    ap.add_argument("--size-gb", type=float, default=2.0, help="uncompressed size of the big archive")
    ap.add_argument("--files", type=int, default=20000, help="number of files in the big archive")
    ap.add_argument("--skip-big", action="store_true")
    ap.add_argument("--tar-gb", type=float, default=1.0, help="uncompressed size of the big tar.gz")
    args = ap.parse_args()

    os.makedirs(args.out, exist_ok=True)

    # 12 MB — safely above the 8 MB materialization threshold.
    big_entry = 12 * 1024 * 1024

    utf8_path = os.path.join(args.out, "tricky-utf8.zip")
    cp866_path = os.path.join(args.out, "tricky-cp866.zip")
    write_tricky(utf8_path, "utf8", big_entry)
    write_tricky(cp866_path, "cp866", big_entry)
    print("created %s (%.1f MB)" % (utf8_path, os.path.getsize(utf8_path) / 1e6))
    print("created %s (%.1f MB)" % (cp866_path, os.path.getsize(cp866_path) / 1e6))

    for label, fmt in (("gnu", tarfile.GNU_FORMAT), ("pax", tarfile.PAX_FORMAT), ("ustar", tarfile.USTAR_FORMAT)):
        # ustar cannot do long names — skip it for this set.
        if fmt is tarfile.USTAR_FORMAT:
            continue
        tp = os.path.join(args.out, "tricky-%s.tar" % label)
        write_tar(tp, fmt, big_entry)
        write_tar_gz(tp, tp + ".gz")
        print("created %s and %s.gz" % (tp, tp))

    if not args.skip_big:
        big_path = os.path.join(args.out, "big.zip")
        target = int(args.size_gb * 1024 ** 3)
        written, markers = write_big(big_path, target, args.files)
        print("created %s: %.2f GB uncompressed, archive %.1f MB" % (
            big_path, written / 1024 ** 3, os.path.getsize(big_path) / 1e6))
        print("files with the marker %s:" % MARKER)
        for m in markers:
            print("   ", m)

        big_tgz = os.path.join(args.out, "big.tar.gz")
        n = write_big_tar_gz(big_tgz, int(args.tar_gb * 1024 ** 3), 8000)
        print("created %s: %.2f GB uncompressed, archive %.1f MB" % (
            big_tgz, n / 1024 ** 3, os.path.getsize(big_tgz) / 1e6))


if __name__ == "__main__":
    main()
