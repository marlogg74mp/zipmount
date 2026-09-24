# ZipMount — русский. Формы множественного числа по CLDR: one, few, many, other.
#
# Строку «WinFsp - Windows File System Proxy, Copyright (C) Bill Zissimopoulos»
# не переводить: лицензия WinFsp требует именно её.

## Числа и единицы

decimal-separator = ,
unit-b = Б
unit-kb = КБ
unit-mb = МБ
unit-gb = ГБ
unit-tb = ТБ
seconds = { $value } с

## Открытие архивов

core-open-failed = не удалось открыть архив { $path }
core-unknown-format = не удалось определить формат { $path }: это не zip, не 7z и не tar
core-rar-not-built = { $path } — RAR-архив, а эта сборка собрана без поддержки rar: условия UnRAR несовместимы с GPL, поэтому rar включается только при сборке из исходников для себя (cargo build --release --features zipmount/rar)
core-unknown-encoding = неизвестная кодировка «{ $value }»; допустимы: auto, utf8, cp866, cp1251
core-deflate-corrupt = данные deflate повреждены (код zlib { $code })
core-password-missing = архив зашифрован: нужен пароль (-p или --password-stdin)
core-password-wrong = неверный пароль

## Чтение архивов

core-mmap-failed = не удалось отобразить архив в память
core-entry-out-of-bounds = данные записи выходят за пределы архива
core-zip-structure = не удалось разобрать структуру zip-архива
core-zip-too-small = файл слишком мал для zip: { $size } байт
core-zip-no-eocd = не найдена запись EOCD — файл не является zip-архивом или повреждён
core-zip64-missing = архив помечен как zip64, но запись zip64 EOCD не найдена
core-zip-cd-out-of-bounds = central directory выходит за пределы файла
core-zip-bad-local-header = повреждён локальный заголовок по смещению { $offset }
core-zip-method = неподдерживаемый метод сжатия { $method } (поддерживаются stored и deflate)
core-deflate-failed = ошибка распаковки deflate: { $error }
core-aes-unknown-strength = неизвестная стойкость AES в записи: { $strength }
core-encrypted-too-short = зашифрованная запись короче собственных служебных полей
core-aes-auth-failed = не сошёлся код аутентификации: запись повреждена
core-7z-structure = не удалось разобрать структуру 7z-архива { $path }; если он под паролем, укажите пароль ключом -p
core-7z-structure-password = не удалось разобрать структуру 7z-архива { $path }: возможно, пароль неверен или архив повреждён
core-7z-block-failed = не удалось развернуть блок { $block }
core-7z-block-error = ошибка распаковки блока { $block }: { $error }
core-tar-structure = не удалось разобрать структуру tar
core-tar-bad-header = первый заголовок tar не проходит проверку контрольной суммы
core-gzip-not-tar = это gzip, но внутри не tar — такой архив не поддерживается
core-gzip-damaged = ошибка распаковки gzip (архив повреждён или обрезан)
core-targz-too-large = развёрнутый tar.gz не помещается в отведённую память: уже { $used } ГБ при потолке { $limit } ГБ. Увеличьте потолок ключом --cache-mb
core-rar-error = ошибка чтения RAR-архива: { $error }
core-grep-substring = не удалось построить поиск подстроки: { $pattern }
core-grep-regex = некорректное регулярное выражение: { $pattern }
core-grep-block-errors = ошибки при обходе блоков: { $errors }
core-verify-short-read = чтение с { $offset } вернуло { $got } байт вместо { $want }
core-verify-crc = CRC32 не совпал: получено { $actual }, ожидалось { $expected }
core-verify-block = <блок { $block }>

## Монтирование через WinFsp

mount-err-create = не удалось создать том WinFsp: { $error }
mount-err-mount = не удалось смонтировать в { $mountpoint }: { $error }
mount-err-dispatcher = не удалось запустить диспетчер WinFsp: { $error }
mount-err-no-winfsp = WinFsp не найден. Установите его командой:  winget install WinFsp.WinFsp

## Общие ошибки

error-prefix = Ошибка
err-create-dir = не удалось создать { $path }
err-write = не удалось записать { $path }
err-read = не удалось прочитать { $path }
err-read-password = не удалось прочитать пароль
err-read-password-stdin = не удалось прочитать пароль со стандартного ввода
err-spawn-background = не удалось запустить фоновый процесс
err-run-failed = не удалось запустить { $tool }
err-tool-failed =
    { $tool } завершился с ошибкой:
    { $output }

## Справка командной строки

help-about = Архив как диск Windows: просмотр, поиск и извлечение без распаковки
help-notice =
    Лицензия GPL-3.0-or-later, исходники: https://github.com/marlogg74mp/zipmount

    Монтирование работает через WinFsp - Windows File System Proxy, Copyright (C) Bill Zissimopoulos — https://github.com/winfsp/winfsp
help-heading-usage = Использование:
help-heading-commands = Команды
help-heading-arguments = Аргументы
help-heading-options = Параметры
help-flag-help = Показать справку
help-flag-version = Показать версию
help-archive = Путь к архиву
help-encoding = Кодировка имён в zip: auto, utf8, cp866, cp1251
help-password = Запросить пароль к архиву (ввод скрыт)
help-password-stdin = Прочитать пароль из стандартного ввода (первая строка)
help-prefix = Префикс пути в выводе, например "Z:\"
help-mount = Смонтировать архив как диск
help-mount-mountpoint = Буква диска (Z:) или путь к пустому каталогу NTFS; если не указана, берётся первая свободная
help-mount-detach = Смонтировать в фоне и сразу вернуть управление
help-mount-open = Открыть смонтированный диск в Проводнике
help-mount-label = Метка тома; по умолчанию имя файла архива
help-mount-cache-mb = Потолок кэша распакованных данных, МБ (для 7z — кэш solid-блоков)
help-ls = Содержимое каталога внутри архива
help-ls-path = Путь внутри архива; по умолчанию корень
help-ls-recursive = Рекурсивно, всё дерево
help-find = Поиск файлов по маске имени
help-find-pattern = Маска, например "*.log"
help-grep = Поиск по содержимому файлов внутри архива
help-grep-pattern = Искомая подстрока (или регулярное выражение с ключом --regex)
help-grep-regex = Трактовать шаблон как регулярное выражение
help-grep-files-only = Только пути файлов, без строк
help-grep-count = Только число совпадений в каждом файле
help-grep-ignore-case = Игнорировать регистр
help-grep-after-context = Строк контекста после совпадения
help-grep-before-context = Строк контекста до совпадения
help-grep-context = Строк контекста с обеих сторон
help-grep-max-count = Не больше N совпадений в одном файле
help-grep-max-total = Не больше N строк во всём выводе
help-grep-path = Искать только внутри этой ветки архива
help-grep-glob = Ограничить маской имени, например "*.log"
help-grep-copy-to = Извлечь найденные файлы в указанный каталог
help-info = Сводка по архиву
help-verify = Сквозная проверка чтения всех записей по их CRC32
help-verify-random = Читать вперемешку, а не последовательно: проверяет перемотку назад
help-unmount = Размонтировать диск
help-unmount-target = Буква диска (Z:) или путь к архиву
help-mounts = Показать смонтированные архивы
help-search = Интерактивный поиск по архиву (вызывается из контекстного меню)
help-shell-install = Добавить пункты в контекстное меню Проводника
help-shell-install-modern = Основное меню Windows 11, без «Показать дополнительные параметры» (один раз попросит права администратора)
help-shell-uninstall = Убрать пункты из контекстного меню
help-language = Показать или выбрать язык программы
help-language-code = Код языка (en, ru, zh-CN, ja, ko, pt-BR, es, de) или auto — следовать языку Windows
help-doctor = Проверка готовности окружения к монтированию

## ls, find, grep

password-prompt = Пароль к архиву:
err-path-not-found = путь не найден в архиве: { $path }
find-summary = найдено: { $count }
grep-copied = извлечено файлов: { $count } -> { $path }
grep-summary = файлов с совпадениями: { $matched } | просмотрено: { $scanned } | пропущено: { $skipped } | распаковано: { $size } за { $seconds } с ({ $speed } МБ/с)
grep-truncated = вывод обрезан по достигнутому потолку (--max-total)
grep-regex-note = шаблон истолкован как регулярное выражение (--regex)

## info

info-archive = архив:
info-format = формат:
info-size = размер архива:
info-files = файлов:
info-dirs = каталогов:
info-uncompressed = в распакованном:
info-ratio = степень сжатия:
info-nodes = узлов в дереве:
info-in-memory = в памяти:
info-in-memory-value = { $size } (tar.gz разворачивается целиком при открытии)
info-solid = solid:
info-rar-solid-yes = да (чтение одного файла требует пройти весь предшествующий поток)
info-7z-solid-yes = да (чтение одного файла разворачивает весь его блок)
info-solid-no = нет (каждый файл распаковывается независимо)
info-headers = оглавление:
info-headers-encrypted = зашифровано
info-blocks = solid-блоков:
info-encrypted = зашифровано:
info-encrypted-value =
    { $count ->
        [one] { $count } запись
        [few] { $count } записи
       *[many] { $count } записей
    }
info-parse-time = разбор занял:

## verify

verify-failure = ОШИБКА  { $path }: { $reason }
verify-summary = проверено файлов: { $ok } | ошибок: { $errors } | прочитано { $size } за { $seconds } с ({ $speed } МБ/с)
verify-random = доступ вперемешку
verify-no-checksum =
    прочитано без сверки с контрольной суммой: { $count ->
        [one] { $count } запись
        [few] { $count } записи
       *[many] { $count } записей
    }
verify-no-checksum-tar =
    tar не хранит контрольных сумм содержимого — сверять не с чем.
    Проверено то, что все записи читаются целиком и не выходят за границы архива.
verify-no-checksum-targz =
    tar не хранит контрольных сумм содержимого, но CRC32 всего потока
    gzip сверен при распаковке — повреждение архива было бы замечено.
verify-no-checksum-rar =
    это зашифрованные записи RAR5 без шифрования оглавления: формат
    намеренно искажает их контрольную сумму, чтобы по ней нельзя было
    подбирать пароль. Сверять с ней нечего — распаковка при этом верна.
verify-no-checksum-zip =
    это записи WinZip AE-2: там поле CRC намеренно пустое, а целостность
    подтверждает HMAC, проверенный при расшифровке.
err-verify-failed =
    проверка не пройдена: { $count ->
        [one] { $count } запись
        [few] { $count } записи
       *[many] { $count } записей
    }

## mount, unmount, mounts

mount-already = Уже смонтирован: { $letter }
mount-done = Смонтировано: { $letter }
mount-done-stats =
    Смонтировано: { $letter }  ({ $files ->
        [one] { $files } файл
        [few] { $files } файла
       *[many] { $files } файлов
    }, { $dirs ->
        [one] { $dirs } каталог
        [few] { $dirs } каталога
       *[many] { $dirs } каталогов
    }, { $size } содержимого)
mount-parse-time = Разбор архива занял { $seconds } с.
mount-stop-hint = Ctrl+C или `zipmount unmount { $letter }` — размонтировать.
mount-unmounting = Размонтирование...
mount-finished = Готово. Архив не изменён.
err-no-free-letters = нет свободных букв диска
err-spawn-password = не удалось передать пароль фоновому процессу
err-mount-failed-detached =
    не удалось смонтировать { $path }.
    Запустите без --detach, чтобы увидеть причину.
err-letter-busy =
    буква { $letter } уже занята другим диском.
    Свободны: { $free }
letters-none = нет свободных букв
unmount-done = Размонтировано: { $letter }
err-unmount-not-responding = процесс монтирования { $letter } не отвечает; запись убрана из списка
err-unmount-timeout = { $letter } не размонтировался за отведённое время: возможно, файлы на нём ещё открыты
err-not-mounted = { $target } не числится смонтированным. Список: zipmount mounts
mounts-none = Ничего не смонтировано.

## Поиск из контекстного меню

search-archive = Архив: { $path }
search-stats =
    { $files ->
        [one] { $files } файл
        [few] { $files } файла
       *[many] { $files } файлов
    }, { $size } в распакованном виде. Формат: { $format }.
search-intro = Поиск идёт по содержимому. Пустая строка — выход.
search-prompt = Что искать:
search-summary =
    файлов с совпадениями: { $matched } из { $scanned } просмотренных, за { $seconds } с
search-truncated = (вывод обрезан)
search-error = Ошибка поиска: { $error }
press-enter = Нажмите Enter, чтобы закрыть...

## Пункты контекстного меню — пишутся и в реестр

menu-mount = Смонтировать как диск
menu-mount-as = Смонтировать на букву
menu-search = Найти в архиве…
menu-unmount = Размонтировать (ZipMount)
menu-unmount-letter = Размонтировать { $letter }: (ZipMount)

## shell-install, shell-uninstall

shell-installed = Пункты добавлены в контекстное меню для: { $extensions }
shell-installed-items =
    { menu-mount } — свободная буква, открыть в Проводнике
    { menu-mount-as } — подменю с выбором
    { menu-search } — поиск по содержимому
    { menu-unmount } — в меню самого смонтированного диска
shell-installed-where =
    В Windows 11 эти пункты живут под «Показать дополнительные параметры»
    — или сразу по Shift+правый клик.
shell-modern-hint = Основное меню без Shift: zipmount shell-install --modern
shell-remove-hint = Убрать: zipmount shell-uninstall
shell-removed = Пункты убраны из контекстного меню.

## Современное меню

modern-building = Собираю пакет…
modern-trust-intro =
    Осталось одно действие с правами администратора.

    Windows выдаёт пункт в основном меню только пакету с подписью,
    а доверие к сертификату — решение уровня машины, поэтому
    нужен запрос прав. Сертификат самоподписанный и лежит здесь:
modern-trust-uac = Сейчас появится окно контроля учётных записей.
modern-registering = Устанавливаю пакет…
modern-done =
    Готово. Пункты в основном контекстном меню:

      { menu-mount } — на архиве
      { menu-mount-as } — подменю только из свободных букв
      { menu-search } — поиск по содержимому
      { menu-unmount } — на пустом месте внутри диска
modern-installed-to = Программа установлена в { $path }
modern-rebuild-hint = После пересборки повторите: zipmount shell-install --modern
modern-files-left = Файлы остались в { $path }
modern-cert-left = Сертификат остался доверенным. Убрать его (нужен администратор):
err-sdk-tool-missing =
    { $tool } не найден. Нужен Windows SDK — поставьте его,
    например: winget install Microsoft.WindowsSDK.10.0.26100
err-exe-busy =
    не удалось обновить { $path }: файл занят.
    Размонтируйте диски (zipmount unmount …) и повторите.
err-shell-dll-missing =
    рядом с программой нет zipmount_shell.dll.
    Соберите её: cargo build --release
err-bin-dir =
    не удалось создать { $path }.
    Если каталог уже есть, он мог остаться от другой учётной записи —
    тогда удалите его или переименуйте.
err-cert-not-trusted =
    сертификат так и не попал в доверенные.
    Без этого Windows не примет пакет. Реестровое меню
    работает без администратора: zipmount shell-install
err-handler-create = COM-класс обработчика не создаётся
err-handler-title = обработчик не вернул заголовок

## language

language-current = Язык: { $name } ({ $code }) — { $source }
language-source-environment = задан переменной ZIPMOUNT_LANG
language-source-saved = выбран командой zipmount language
language-source-windows = по языку Windows
language-source-default = по умолчанию
language-available = Доступны:
language-hint = Выбрать: zipmount language <код>. Снова следовать языку Windows: zipmount language auto
language-set = Язык: { $name } ({ $code }).
language-follows-windows = Язык снова следует языку Windows: { $name } ({ $code }).
language-menu-updated =
    { $count ->
        [0] В реестре нет пунктов меню, которые нужно переписать; современное меню подхватит язык само.
        [one] Пункты меню в реестре переписаны на этом языке: { $count } вид; современное меню подхватит язык само.
        [few] Пункты меню в реестре переписаны на этом языке: { $count } вида; современное меню подхватит язык само.
       *[many] Пункты меню в реестре переписаны на этом языке: { $count } видов; современное меню подхватит язык само.
    }
language-env-overrides = Обратите внимание: задана ZIPMOUNT_LANG={ $value }, и в этой консоли язык по-прежнему определяет она.
err-language-unknown = неизвестный язык «{ $code }». Доступны: { $available }; или auto — следовать языку Windows
err-language-save = не удалось сохранить выбор языка

## doctor

doctor-winfsp = Библиотека WinFsp:
doctor-winfsp-found = найдена ({ $path })
doctor-winfsp-missing = не найдена
doctor-rar = Поддержка rar:
doctor-rar-yes = есть (личная сборка с --features rar, распространять её нельзя)
doctor-rar-no = нет (официальная сборка: UnRAR несовместим с GPL)
doctor-language = Язык:
doctor-modern = Современное меню:
doctor-modern-ok = работает («{ $title }»)
doctor-modern-silent = пакет есть, обработчик молчит — { $error }
doctor-modern-missing = не установлено (zipmount shell-install --modern)
doctor-init = Инициализация:
doctor-init-ok = успешно
doctor-init-failed = НЕ УДАЛАСЬ
doctor-ready =
    Всё готово. Смонтировать архив:
        zipmount mount <архив.zip> Z:
doctor-reason = Причина: { $error }
doctor-no-winfsp-note =
    Просмотр, поиск и извлечение (ls, find, grep, verify) работают
    и без WinFsp — драйвер нужен только для монтирования диска.

## Linux и macOS: без букв дисков и без Проводника

help-about-unix = Архив как папка: просмотр, поиск и извлечение без распаковки
help-notice-unix = Лицензия GPL-3.0-or-later, исходники: https://github.com/marlogg74mp/zipmount
help-prefix-unix = Префикс пути в выводе, например "/home/me/ZipMount/logs/"
help-mount-mountpoint-unix = Пустой каталог для монтирования; если не указан — ~/ZipMount/<имя архива>
help-mount-open-unix = Открыть смонтированную папку в файловом менеджере
help-unmount-unix = Размонтировать архив
help-unmount-target-unix = Каталог монтирования или путь к архиву
help-language-code-unix = Код языка (en, ru, zh-CN, ja, ko, pt-BR, es, de) или auto — следовать языку системы
language-source-system = по языку системы
language-hint-unix = Выбрать: zipmount language <код>. Снова следовать языку системы: zipmount language auto
language-follows-system = Язык снова следует языку системы: { $name } ({ $code }).
err-language-unknown-unix = неизвестный язык «{ $code }». Доступны: { $available }; или auto — следовать языку системы
mount-err-fuse = не удалось смонтировать в { $mountpoint }: { $error }
err-no-fuse = FUSE здесь недоступен: нет { $missing }. Установите пакет fuse3, например:  sudo apt install fuse3
err-mountpoint-not-dir = { $path } — не каталог
err-mountpoint-not-empty = { $path } не пуст; монтировать можно только в пустой каталог
err-unmount-failed = не удалось размонтировать { $target }: { $error }
err-mount-unsupported = монтирование на этой системе пока недоступно
doctor-fuse = FUSE:
doctor-fuse-ok = доступен (/dev/fuse, fusermount3)
doctor-fuse-note =
    Просмотр, поиск и извлечение (ls, find, grep, verify) работают
    и без FUSE — он нужен только для монтирования. Установить его можно
    менеджером пакетов: sudo apt install fuse3 (Debian, Ubuntu),
    sudo dnf install fuse3 (Fedora), sudo pacman -S fuse3 (Arch).
doctor-mount = Монтирование:
doctor-mount-unsupported-note =
    Просмотр, поиск и извлечение (ls, find, grep, verify) на этой системе
    работают; монтирование здесь пока недоступно.
doctor-ready-unix =
    Всё готово. Смонтировать архив:
        zipmount mount <архив.zip>
