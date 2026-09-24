# ZipMount — Deutsch. Machine-assisted translation; corrections are welcome.
#
# "WinFsp - Windows File System Proxy, Copyright (C) Bill Zissimopoulos" nicht übersetzen:
# Die Lizenz von WinFsp verlangt genau diesen Hinweis.

## Zahlen und Einheiten

decimal-separator = ,
unit-b = B
unit-kb = KB
unit-mb = MB
unit-gb = GB
unit-tb = TB
seconds = { $value } s

## Archive öffnen

core-open-failed = Archiv { $path } kann nicht geöffnet werden
core-unknown-format = Das Format von { $path } ist nicht erkennbar: Es ist weder zip noch 7z noch tar
core-rar-not-built = { $path } ist ein RAR-Archiv, und dieser Build unterstützt kein rar: Die UnRAR-Lizenz ist mit der GPL unvereinbar, daher lässt sich rar nur beim Bauen aus dem Quellcode für den Eigengebrauch einschalten (cargo build --release --features zipmount/rar)
core-unknown-encoding = unbekannte Kodierung „{ $value }“; gültig: auto, utf8, cp866, cp1251
core-deflate-corrupt = Die deflate-Daten sind beschädigt (zlib-Code { $code })
core-password-missing = Das Archiv ist verschlüsselt: Ein Passwort ist nötig (-p oder --password-stdin)
core-password-wrong = falsches Passwort

## Archive lesen

core-mmap-failed = Das Archiv kann nicht in den Speicher abgebildet werden
core-entry-out-of-bounds = Die Daten des Eintrags reichen über das Ende des Archivs hinaus
core-zip-structure = Die Struktur des zip-Archivs kann nicht gelesen werden
core-zip-too-small = Die Datei ist für ein zip zu klein: { $size } Bytes
core-zip-no-eocd = Kein Endeintrag des zentralen Verzeichnisses — die Datei ist kein zip-Archiv oder beschädigt
core-zip64-missing = Das Archiv ist als zip64 markiert, aber der zip64-Endeintrag fehlt
core-zip-cd-out-of-bounds = Das zentrale Verzeichnis reicht über das Dateiende hinaus
core-zip-bad-local-header = Beschädigter lokaler Header an Offset { $offset }
core-zip-method = Nicht unterstützte Kompressionsmethode { $method } (unterstützt werden stored und deflate)
core-deflate-failed = Fehler beim Entpacken von deflate: { $error }
core-aes-unknown-strength = Unbekannte AES-Stärke im Eintrag: { $strength }
core-encrypted-too-short = Der verschlüsselte Eintrag ist kürzer als seine eigenen Headerfelder
core-aes-auth-failed = Der Authentifizierungscode stimmt nicht: Der Eintrag ist beschädigt
core-7z-structure = Die Struktur des 7z-Archivs { $path } kann nicht gelesen werden; ist es passwortgeschützt, geben Sie das Passwort mit -p an
core-7z-structure-password = Die Struktur des 7z-Archivs { $path } kann nicht gelesen werden: Das Passwort ist vielleicht falsch oder das Archiv beschädigt
core-7z-block-failed = Block { $block } kann nicht entpackt werden
core-7z-block-error = Fehler beim Entpacken von Block { $block }: { $error }
core-tar-structure = Die Struktur des tar-Archivs kann nicht gelesen werden
core-tar-bad-header = Der erste tar-Header besteht die Prüfsummenkontrolle nicht
core-gzip-not-tar = Das ist ein gzip ohne tar darin — solche Archive werden nicht unterstützt
core-gzip-damaged = Fehler beim Entpacken von gzip (das Archiv ist beschädigt oder abgeschnitten)
core-targz-too-large = Das entpackte tar.gz passt nicht in den erlaubten Speicher: bisher { $used } GB bei einer Obergrenze von { $limit } GB. Erhöhen Sie die Grenze mit --cache-mb
core-rar-error = Fehler beim Lesen des RAR-Archivs: { $error }
core-grep-substring = Die Teilstring-Suche kann nicht aufgebaut werden für: { $pattern }
core-grep-regex = Ungültiger regulärer Ausdruck: { $pattern }
core-grep-block-errors = Fehler beim Durchlaufen der Blöcke: { $errors }
core-verify-short-read = Ein Lesevorgang bei { $offset } lieferte { $got } Bytes statt { $want }
core-verify-crc = CRC32 stimmt nicht: erhalten { $actual }, erwartet { $expected }
core-verify-block = <Block { $block }>

## Einhängen über WinFsp

mount-err-create = Das WinFsp-Volume kann nicht erstellt werden: { $error }
mount-err-mount = Einhängen unter { $mountpoint } nicht möglich: { $error }
mount-err-dispatcher = Der WinFsp-Dispatcher kann nicht gestartet werden: { $error }
mount-err-no-winfsp = WinFsp nicht gefunden. Installieren Sie es mit:  winget install WinFsp.WinFsp

## Allgemeine Fehler

error-prefix = Fehler
err-create-dir = { $path } kann nicht erstellt werden
err-write = { $path } kann nicht geschrieben werden
err-read = { $path } kann nicht gelesen werden
err-read-password = Das Passwort kann nicht gelesen werden
err-read-password-stdin = Das Passwort kann nicht von der Standardeingabe gelesen werden
err-spawn-background = Der Hintergrundprozess kann nicht gestartet werden
err-run-failed = { $tool } kann nicht gestartet werden
err-tool-failed =
    { $tool } ist fehlgeschlagen:
    { $output }

## Befehlszeilenhilfe

help-about = Ein Archiv als Windows-Laufwerk: durchsuchen und entnehmen ohne Entpacken
help-notice =
    Lizenz GPL-3.0-or-later, Quellcode: https://github.com/marlogg74mp/zipmount

    Das Einhängen läuft über WinFsp - Windows File System Proxy, Copyright (C) Bill Zissimopoulos — https://github.com/winfsp/winfsp
help-heading-usage = Aufruf:
help-heading-commands = Befehle
help-heading-arguments = Argumente
help-heading-options = Optionen
help-flag-help = Hilfe anzeigen
help-flag-version = Version anzeigen
help-archive = Pfad zum Archiv
help-encoding = Kodierung der Namen in zip: auto, utf8, cp866, cp1251
help-password = Nach dem Archivpasswort fragen (verdeckte Eingabe)
help-password-stdin = Das Passwort von der Standardeingabe lesen (erste Zeile)
help-prefix = Pfadpräfix in der Ausgabe, zum Beispiel "Z:\"
help-mount = Ein Archiv als Laufwerk einhängen
help-mount-mountpoint = Laufwerksbuchstabe (Z:) oder Pfad zu einem leeren NTFS-Ordner; ohne Angabe der erste freie Buchstabe
help-mount-detach = Im Hintergrund einhängen und sofort zurückkehren
help-mount-open = Das eingehängte Laufwerk im Datei-Explorer öffnen
help-mount-label = Volumebezeichnung; standardmäßig der Dateiname des Archivs
help-mount-cache-mb = Obergrenze des Caches für entpackte Daten, MB (bei 7z der Cache der solid-Blöcke)
help-ls = Einen Ordner im Archiv auflisten
help-ls-path = Pfad im Archiv; standardmäßig das Stammverzeichnis
help-ls-recursive = Rekursiv, der ganze Baum
help-find = Dateien nach einem Namensmuster suchen
help-find-pattern = Muster, zum Beispiel "*.log"
help-grep = Den Inhalt der Dateien im Archiv durchsuchen
help-grep-pattern = Gesuchter Teilstring (oder ein regulärer Ausdruck mit --regex)
help-grep-regex = Das Muster als regulären Ausdruck behandeln
help-grep-files-only = Nur Dateipfade, keine Zeilen
help-grep-count = Nur die Anzahl der Treffer je Datei
help-grep-ignore-case = Groß-/Kleinschreibung ignorieren
help-grep-after-context = Kontextzeilen nach einem Treffer
help-grep-before-context = Kontextzeilen vor einem Treffer
help-grep-context = Kontextzeilen auf beiden Seiten
help-grep-max-count = Höchstens N Treffer je Datei
help-grep-max-total = Höchstens N Zeilen in der gesamten Ausgabe
help-grep-path = Nur in diesem Zweig des Archivs suchen
help-grep-glob = Auf ein Namensmuster beschränken, zum Beispiel "*.log"
help-grep-copy-to = Die gefundenen Dateien in diesen Ordner entnehmen
help-info = Übersicht über ein Archiv
help-verify = Lesen jedes Eintrags durchgehend gegen seine CRC32 prüfen
help-verify-random = In gemischter statt fortlaufender Reihenfolge lesen: prüft das Zurückspringen
help-unmount = Ein Laufwerk aushängen
help-unmount-target = Laufwerksbuchstabe (Z:) oder Pfad zum Archiv
help-mounts = Eingehängte Archive anzeigen
help-search = Interaktive Suche in einem Archiv (aus dem Kontextmenü gestartet)
help-shell-install = Einträge zum Kontextmenü des Datei-Explorers hinzufügen
help-shell-install-modern = Das Hauptmenü von Windows 11, ohne „Weitere Optionen anzeigen“ (fragt einmal nach Administratorrechten)
help-shell-uninstall = Die Einträge aus dem Kontextmenü entfernen
help-language = Die Sprache des Programms anzeigen oder wählen
help-language-code = Sprachcode (en, ru, zh-CN, ja, ko, pt-BR, es, de) oder auto, um Windows zu folgen
help-doctor = Prüfen, ob die Umgebung zum Einhängen bereit ist

## ls, find, grep

password-prompt = Archivpasswort:
err-path-not-found = Pfad im Archiv nicht gefunden: { $path }
find-summary = gefunden: { $count }
grep-copied = entnommene Dateien: { $count } -> { $path }
grep-summary = Dateien mit Treffern: { $matched } | durchsucht: { $scanned } | übersprungen: { $skipped } | entpackt: { $size } in { $seconds } s ({ $speed } MB/s)
grep-truncated = Ausgabe an der Grenze abgeschnitten (--max-total)
grep-regex-note = Das Muster wurde als regulärer Ausdruck verstanden (--regex)

## info

info-archive = Archiv:
info-format = Format:
info-size = Archivgröße:
info-files = Dateien:
info-dirs = Ordner:
info-uncompressed = entpackt:
info-ratio = Kompressionsrate:
info-nodes = Baumknoten:
info-in-memory = im Speicher:
info-in-memory-value = { $size } (tar.gz wird beim Öffnen vollständig entpackt)
info-solid = solid:
info-rar-solid-yes = ja (eine Datei zu lesen heißt, den ganzen vorherigen Datenstrom zu durchlaufen)
info-7z-solid-yes = ja (eine Datei zu lesen entpackt ihren ganzen Block)
info-solid-no = nein (jede Datei wird einzeln entpackt)
info-headers = Inhaltsverzeichnis:
info-headers-encrypted = verschlüsselt
info-blocks = solid-Blöcke:
info-encrypted = verschlüsselt:
info-encrypted-value =
    { $count ->
        [one] { $count } Eintrag
       *[other] { $count } Einträge
    }
info-parse-time = Einlesen dauerte:

## verify

verify-failure = FEHLER  { $path }: { $reason }
verify-summary = geprüfte Dateien: { $ok } | Fehler: { $errors } | gelesen { $size } in { $seconds } s ({ $speed } MB/s)
verify-random = gemischter Zugriff
verify-no-checksum =
    { $count ->
        [one] ohne Prüfsumme zum Vergleich gelesen: { $count } Eintrag
       *[other] ohne Prüfsumme zum Vergleich gelesen: { $count } Einträge
    }
verify-no-checksum-tar =
    tar speichert keine Prüfsummen des Inhalts — es gibt nichts zum Vergleichen.
    Geprüft wurde, dass jeder Eintrag vollständig lesbar ist und im Archiv bleibt.
verify-no-checksum-targz =
    tar speichert keine Prüfsummen des Inhalts, aber die CRC32 des ganzen
    gzip-Stroms wurde beim Entpacken geprüft — ein Schaden wäre aufgefallen.
verify-no-checksum-rar =
    Das sind verschlüsselte RAR5-Einträge ohne verschlüsseltes Inhaltsverzeichnis:
    Das Format verfälscht ihre Prüfsumme absichtlich, damit sie nicht zum Erraten
    des Passworts taugt. Es gibt nichts zum Vergleichen — die Daten stimmen trotzdem.
verify-no-checksum-zip =
    Das sind WinZip-AE-2-Einträge: Ihr CRC-Feld ist absichtlich leer, und die
    Integrität bestätigt der beim Entschlüsseln geprüfte HMAC.
err-verify-failed =
    { $count ->
        [one] Prüfung fehlgeschlagen: { $count } Eintrag
       *[other] Prüfung fehlgeschlagen: { $count } Einträge
    }

## mount, unmount, mounts

mount-already = Bereits eingehängt: { $letter }
mount-done = Eingehängt: { $letter }
mount-done-stats =
    Eingehängt: { $letter }  ({ $files ->
        [one] { $files } Datei
       *[other] { $files } Dateien
    }, { $dirs ->
        [one] { $dirs } Ordner
       *[other] { $dirs } Ordner
    }, { $size } Inhalt)
mount-parse-time = Das Einlesen des Archivs dauerte { $seconds } s.
mount-stop-hint = Strg+C oder `zipmount unmount { $letter }` zum Aushängen.
mount-unmounting = Wird ausgehängt...
mount-finished = Fertig. Das Archiv ist unverändert.
err-no-free-letters = Keine freien Laufwerksbuchstaben
err-spawn-password = Das Passwort kann nicht an den Hintergrundprozess übergeben werden
err-mount-failed-detached =
    { $path } kann nicht eingehängt werden.
    Ohne --detach ausführen, um den Grund zu sehen.
err-letter-busy =
    Laufwerksbuchstabe { $letter } ist von einem anderen Laufwerk belegt.
    Frei: { $free }
letters-none = keine freien Buchstaben
unmount-done = Ausgehängt: { $letter }
err-unmount-not-responding = Der Einhängeprozess von { $letter } antwortet nicht; sein Eintrag wurde aus der Liste entfernt
err-unmount-timeout = { $letter } wurde nicht rechtzeitig ausgehängt: Möglicherweise sind darauf noch Dateien geöffnet
err-not-mounted = { $target } ist nicht als eingehängt verzeichnet. Liste: zipmount mounts
mounts-none = Nichts eingehängt.

## Suche aus dem Kontextmenü

search-archive = Archiv: { $path }
search-stats =
    { $files ->
        [one] { $files } Datei
       *[other] { $files } Dateien
    }, entpackt { $size }. Format: { $format }.
search-intro = Gesucht wird im Inhalt der Dateien. Eine leere Zeile beendet.
search-prompt = Suchen nach:
search-summary = Dateien mit Treffern: { $matched } von { $scanned } durchsuchten, in { $seconds } s
search-truncated = (Ausgabe abgeschnitten)
search-error = Suchfehler: { $error }
press-enter = Zum Schließen Eingabe drücken...

## Kontextmenüeinträge — werden auch in die Registry geschrieben

menu-mount = Als Laufwerk einhängen
menu-mount-as = Unter Buchstaben einhängen
menu-search = Im Archiv suchen…
menu-unmount = Aushängen (ZipMount)
menu-unmount-letter = { $letter }: aushängen (ZipMount)

## shell-install, shell-uninstall

shell-installed = Kontextmenüeinträge hinzugefügt für: { $extensions }
shell-installed-items =
    { menu-mount } — ein freier Buchstabe, geöffnet im Datei-Explorer
    { menu-mount-as } — ein Untermenü zur Auswahl
    { menu-search } — Suche im Inhalt
    { menu-unmount } — im Menü des eingehängten Laufwerks selbst
shell-installed-where =
    In Windows 11 stehen diese Einträge unter „Weitere Optionen anzeigen“
    — oder erscheinen direkt mit Umschalt+Rechtsklick.
shell-modern-hint = Das Hauptmenü ohne Umschalt: zipmount shell-install --modern
shell-remove-hint = Entfernen: zipmount shell-uninstall
shell-removed = Einträge aus dem Kontextmenü entfernt.

## Das moderne Menü

modern-building = Paket wird gebaut…
modern-trust-intro =
    Ein Schritt mit Administratorrechten bleibt.

    Windows lässt nur ein signiertes Paket ins Hauptmenü, und einem Zertifikat
    zu vertrauen ist eine Entscheidung für den ganzen Rechner — daher die
    Rechteabfrage. Das Zertifikat ist selbstsigniert und liegt hier:
modern-trust-uac = Gleich erscheint ein Fenster der Benutzerkontensteuerung.
modern-registering = Paket wird installiert…
modern-done =
    Fertig. Einträge im Hauptkontextmenü:

      { menu-mount } — auf einem Archiv
      { menu-mount-as } — ein Untermenü nur mit freien Buchstaben
      { menu-search } — Suche im Inhalt
      { menu-unmount } — auf einer leeren Fläche im Laufwerk
modern-installed-to = Das Programm ist installiert in { $path }
modern-rebuild-hint = Nach einem Neubau erneut ausführen: zipmount shell-install --modern
modern-files-left = Die Dateien bleiben in { $path }
modern-cert-left = Das Zertifikat bleibt vertrauenswürdig. Zum Entfernen (Administrator nötig):
err-sdk-tool-missing =
    { $tool } nicht gefunden. Es gehört zum Windows SDK — installieren Sie es,
    zum Beispiel: winget install Microsoft.WindowsSDK.10.0.26100
err-exe-busy =
    { $path } kann nicht aktualisiert werden: Die Datei wird verwendet.
    Hängen Sie die Laufwerke aus (zipmount unmount …) und versuchen Sie es erneut.
err-shell-dll-missing =
    zipmount_shell.dll liegt nicht neben dem Programm.
    Bauen Sie sie: cargo build --release
err-bin-dir =
    { $path } kann nicht erstellt werden.
    Falls der Ordner schon existiert, stammt er vielleicht von einem anderen Konto —
    dann löschen oder umbenennen Sie ihn.
err-cert-not-trusted =
    Das Zertifikat wurde nicht vertrauenswürdig.
    Ohne das nimmt Windows das Paket nicht an. Das Registry-Menü funktioniert
    ohne Administrator: zipmount shell-install
err-handler-create = Die COM-Klasse des Handlers kann nicht erstellt werden
err-handler-title = Der Handler hat keinen Titel geliefert

## language

language-current = Sprache: { $name } ({ $code }) — { $source }
language-source-environment = durch ZIPMOUNT_LANG festgelegt
language-source-saved = mit zipmount language gewählt
language-source-windows = die Anzeigesprache von Windows
language-source-default = die Voreinstellung
language-available = Verfügbar:
language-hint = Wählen: zipmount language <Code>. Wieder Windows folgen: zipmount language auto
language-set = Sprache: { $name } ({ $code }).
language-follows-windows = Die Sprache folgt wieder Windows: { $name } ({ $code }).
language-menu-updated =
    { $count ->
        [0] In der Registry gibt es keine Menüeinträge umzuschreiben; das moderne Menü übernimmt die Sprache von selbst.
        [one] Eine Art von Menüeinträgen in der Registry wurde in dieser Sprache neu geschrieben; das moderne Menü übernimmt sie von selbst.
       *[other] { $count } Arten von Menüeinträgen in der Registry wurden in dieser Sprache neu geschrieben; das moderne Menü übernimmt sie von selbst.
    }
language-env-overrides = Hinweis: ZIPMOUNT_LANG={ $value } ist gesetzt und bestimmt in dieser Konsole weiterhin die Sprache.
err-language-unknown = unbekannte Sprache „{ $code }“. Verfügbar: { $available }; oder auto, um Windows zu folgen
err-language-save = Die Sprachwahl kann nicht gespeichert werden

## doctor

doctor-winfsp = WinFsp-Bibliothek:
doctor-winfsp-found = gefunden ({ $path })
doctor-winfsp-missing = nicht gefunden
doctor-rar = rar-Unterstützung:
doctor-rar-yes = ja (ein persönlicher Build mit --features rar; darf nicht weitergegeben werden)
doctor-rar-no = nein (offizieller Build: UnRAR ist mit der GPL unvereinbar)
doctor-language = Sprache:
doctor-modern = Modernes Menü:
doctor-modern-ok = funktioniert („{ $title }“)
doctor-modern-silent = Paket vorhanden, der Handler antwortet nicht — { $error }
doctor-modern-missing = nicht installiert (zipmount shell-install --modern)
doctor-init = Initialisierung:
doctor-init-ok = erfolgreich
doctor-init-failed = FEHLGESCHLAGEN
doctor-ready =
    Alles bereit. Ein Archiv einhängen:
        zipmount mount <archiv.zip> Z:
doctor-reason = Grund: { $error }
doctor-no-winfsp-note =
    Durchsuchen und Entnehmen (ls, find, grep, verify) funktionieren auch ohne
    WinFsp — der Treiber wird nur zum Einhängen eines Laufwerks gebraucht.

## Linux und macOS: keine Laufwerksbuchstaben, kein Datei-Explorer

help-about-unix = Ein Archiv als Ordner: durchsuchen und entnehmen ohne Entpacken
help-notice-unix = Lizenz GPL-3.0-or-later, Quellcode: https://github.com/marlogg74mp/zipmount
help-prefix-unix = Pfadpräfix in der Ausgabe, zum Beispiel "/home/me/ZipMount/logs/"
help-mount-mountpoint-unix = Ein leeres Verzeichnis zum Einhängen; ohne Angabe ~/ZipMount/<Archivname>
help-mount-open-unix = Den eingehängten Ordner im Dateimanager öffnen
help-unmount-unix = Ein Archiv aushängen
help-unmount-target-unix = Das Einhängeverzeichnis oder der Pfad zum Archiv
help-language-code-unix = Sprachcode (en, ru, zh-CN, ja, ko, pt-BR, es, de) oder auto, um dem System zu folgen
language-source-system = die Sprache des Systems
language-hint-unix = Wählen: zipmount language <Code>. Wieder dem System folgen: zipmount language auto
language-follows-system = Die Sprache folgt wieder dem System: { $name } ({ $code }).
err-language-unknown-unix = unbekannte Sprache „{ $code }“. Verfügbar: { $available }; oder auto, um dem System zu folgen
mount-err-fuse = kann nicht in { $mountpoint } einhängen: { $error }
err-no-fuse = FUSE ist hier nicht verfügbar: { $missing } fehlt. Installieren Sie das Paket fuse3, zum Beispiel:  sudo apt install fuse3
err-mountpoint-not-dir = { $path } ist kein Verzeichnis
err-mountpoint-not-empty = { $path } ist nicht leer; hängen Sie in ein leeres Verzeichnis ein
err-unmount-failed = kann { $target } nicht aushängen: { $error }
err-mount-unsupported = Einhängen ist auf diesem System noch nicht verfügbar
doctor-fuse = FUSE:
doctor-fuse-ok = verfügbar (/dev/fuse, fusermount3)
mount-err-nfs = kann nicht in { $mountpoint } einhängen: { $error }
doctor-nfs-ok = verfügbar (der NFS-Client des Systems)
doctor-fuse-note =
    Durchsuchen und Entnehmen (ls, find, grep, verify) funktionieren auch ohne
    FUSE — es wird nur zum Einhängen gebraucht. Installieren mit dem
    Paketmanager: sudo apt install fuse3 (Debian, Ubuntu), sudo dnf install fuse3
    (Fedora), sudo pacman -S fuse3 (Arch).
doctor-mount = Einhängen:
doctor-mount-unsupported-note =
    Durchsuchen und Entnehmen (ls, find, grep, verify) funktionieren auf diesem
    System; Einhängen ist hier noch nicht verfügbar.
doctor-ready-unix =
    Alles bereit. Ein Archiv einhängen:
        zipmount mount <archiv.zip>
