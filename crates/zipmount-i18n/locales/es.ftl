# ZipMount — español. Machine-assisted translation; corrections are welcome.
#
# No traducir "WinFsp - Windows File System Proxy, Copyright (C) Bill Zissimopoulos":
# la licencia de WinFsp exige exactamente ese aviso.

## Números y unidades

decimal-separator = ,
unit-b = B
unit-kb = KB
unit-mb = MB
unit-gb = GB
unit-tb = TB
seconds = { $value } s

## Abrir archivos comprimidos

core-open-failed = no se puede abrir el archivo comprimido { $path }
core-unknown-format = no se puede determinar el formato de { $path }: no es zip, 7z ni tar
core-rar-not-built = { $path } es un archivo RAR, y esta compilación no admite rar: la licencia de UnRAR es incompatible con la GPL, así que rar solo se activa al compilar desde el código fuente para uso propio (cargo build --release --features zipmount/rar)
core-unknown-encoding = codificación desconocida "{ $value }"; válidas: auto, utf8, cp866, cp1251
core-deflate-corrupt = los datos deflate están dañados (código zlib { $code })
core-password-missing = el archivo está cifrado: se necesita una contraseña (-p o --password-stdin)
core-password-wrong = contraseña incorrecta

## Leer archivos comprimidos

core-mmap-failed = no se puede asignar el archivo comprimido a la memoria
core-entry-out-of-bounds = los datos de la entrada sobrepasan el final del archivo comprimido
core-zip-structure = no se puede analizar la estructura del archivo zip
core-zip-too-small = el archivo es demasiado pequeño para ser un zip: { $size } bytes
core-zip-no-eocd = no se encuentra el registro de fin del directorio central — el archivo no es zip o está dañado
core-zip64-missing = el archivo está marcado como zip64, pero falta su registro final zip64
core-zip-cd-out-of-bounds = el directorio central sobrepasa el final del archivo
core-zip-bad-local-header = encabezado local dañado en el desplazamiento { $offset }
core-zip-method = método de compresión { $method } no admitido (se admiten stored y deflate)
core-deflate-failed = error de descompresión deflate: { $error }
core-aes-unknown-strength = intensidad AES desconocida en la entrada: { $strength }
core-encrypted-too-short = la entrada cifrada es más corta que sus propios campos de encabezado
core-aes-auth-failed = el código de autenticación no coincide: la entrada está dañada
core-7z-structure = no se puede analizar la estructura del archivo 7z { $path }; si está protegido con contraseña, indíquela con -p
core-7z-structure-password = no se puede analizar la estructura del archivo 7z { $path }: la contraseña puede ser incorrecta o el archivo estar dañado
core-7z-block-failed = no se puede expandir el bloque { $block }
core-7z-block-error = error al descomprimir el bloque { $block }: { $error }
core-tar-structure = no se puede analizar la estructura del archivo tar
core-tar-bad-header = el primer encabezado tar no supera la comprobación de la suma de control
core-gzip-not-tar = es un gzip sin tar dentro — ese tipo de archivo no se admite
core-gzip-damaged = error de descompresión gzip (el archivo está dañado o truncado)
core-targz-too-large = el tar.gz expandido no cabe en la memoria permitida: { $used } GB hasta ahora, con un límite de { $limit } GB. Aumente el límite con --cache-mb
core-rar-error = error al leer el archivo RAR: { $error }
core-grep-substring = no se puede preparar la búsqueda de la subcadena: { $pattern }
core-grep-regex = expresión regular no válida: { $pattern }
core-grep-block-errors = errores al recorrer los bloques: { $errors }
core-verify-short-read = la lectura en { $offset } devolvió { $got } bytes en lugar de { $want }
core-verify-crc = el CRC32 no coincide: obtenido { $actual }, esperado { $expected }
core-verify-block = <bloque { $block }>

## Montaje mediante WinFsp

mount-err-create = no se puede crear el volumen WinFsp: { $error }
mount-err-mount = no se puede montar en { $mountpoint }: { $error }
mount-err-dispatcher = no se puede iniciar el despachador de WinFsp: { $error }
mount-err-no-winfsp = No se encuentra WinFsp. Instálelo con:  winget install WinFsp.WinFsp

## Errores comunes

error-prefix = Error
err-create-dir = no se puede crear { $path }
err-write = no se puede escribir { $path }
err-read = no se puede leer { $path }
err-read-password = no se puede leer la contraseña
err-read-password-stdin = no se puede leer la contraseña de la entrada estándar
err-spawn-background = no se puede iniciar el proceso en segundo plano
err-run-failed = no se puede iniciar { $tool }
err-tool-failed =
    { $tool } falló:
    { $output }

## Ayuda de la línea de comandos

help-about = Un archivo comprimido como unidad de Windows: explorar, buscar y extraer sin descomprimir
help-notice =
    Licencia GPL-3.0-or-later, código fuente: https://github.com/marlogg74mp/zipmount

    El montaje usa WinFsp - Windows File System Proxy, Copyright (C) Bill Zissimopoulos — https://github.com/winfsp/winfsp
help-heading-usage = Uso:
help-heading-commands = Comandos
help-heading-arguments = Argumentos
help-heading-options = Opciones
help-flag-help = Mostrar la ayuda
help-flag-version = Mostrar la versión
help-archive = Ruta del archivo comprimido
help-encoding = Codificación de los nombres en zip: auto, utf8, cp866, cp1251
help-password = Pedir la contraseña del archivo (entrada oculta)
help-password-stdin = Leer la contraseña de la entrada estándar (la primera línea)
help-prefix = Prefijo de las rutas en la salida, por ejemplo "Z:\"
help-mount = Montar un archivo comprimido como unidad
help-mount-mountpoint = Letra de unidad (Z:) o ruta de una carpeta NTFS vacía; si se omite, la primera letra libre
help-mount-detach = Montar en segundo plano y volver de inmediato
help-mount-open = Abrir la unidad montada en el Explorador de archivos
help-mount-label = Etiqueta del volumen; por defecto, el nombre del archivo comprimido
help-mount-cache-mb = Límite de la caché de datos descomprimidos, MB (en 7z, la caché de bloques solid)
help-ls = Listar una carpeta dentro del archivo comprimido
help-ls-path = Ruta dentro del archivo; por defecto, la raíz
help-ls-recursive = De forma recursiva, todo el árbol
help-find = Buscar archivos por un patrón de nombre
help-find-pattern = Patrón, por ejemplo "*.log"
help-grep = Buscar en el contenido de los archivos dentro del archivo comprimido
help-grep-pattern = Subcadena que buscar (o una expresión regular con --regex)
help-grep-regex = Tratar el patrón como una expresión regular
help-grep-files-only = Solo las rutas de los archivos, sin las líneas
help-grep-count = Solo el número de coincidencias en cada archivo
help-grep-ignore-case = Ignorar mayúsculas y minúsculas
help-grep-after-context = Líneas de contexto después de una coincidencia
help-grep-before-context = Líneas de contexto antes de una coincidencia
help-grep-context = Líneas de contexto a ambos lados
help-grep-max-count = Como máximo N coincidencias por archivo
help-grep-max-total = Como máximo N líneas en toda la salida
help-grep-path = Buscar solo dentro de esta rama del archivo comprimido
help-grep-glob = Restringir por un patrón de nombre, por ejemplo "*.log"
help-grep-copy-to = Extraer los archivos encontrados en esta carpeta
help-info = Resumen de un archivo comprimido
help-verify = Comprobación de lectura de extremo a extremo de cada entrada con su CRC32
help-verify-random = Leer en orden aleatorio en lugar de secuencial: prueba el retroceso
help-unmount = Desmontar una unidad
help-unmount-target = Letra de unidad (Z:) o ruta del archivo comprimido
help-mounts = Mostrar los archivos comprimidos montados
help-search = Búsqueda interactiva en un archivo comprimido (se inicia desde el menú contextual)
help-shell-install = Añadir elementos al menú contextual del Explorador de archivos
help-shell-install-modern = El menú principal de Windows 11, sin "Mostrar más opciones" (pide permisos de administrador una vez)
help-shell-uninstall = Quitar los elementos del menú contextual
help-language = Mostrar o elegir el idioma del programa
help-language-code = Código de idioma (en, ru, zh-CN, ja, ko, pt-BR, es, de), o auto para seguir a Windows
help-doctor = Comprobar que el entorno está listo para montar

## ls, find, grep

password-prompt = Contraseña del archivo:
err-path-not-found = ruta no encontrada en el archivo comprimido: { $path }
find-summary = encontrados: { $count }
grep-copied = archivos extraídos: { $count } -> { $path }
grep-summary = archivos con coincidencias: { $matched } | examinados: { $scanned } | omitidos: { $skipped } | descomprimido: { $size } en { $seconds } s ({ $speed } MB/s)
grep-truncated = salida recortada al alcanzar el límite (--max-total)
grep-regex-note = el patrón se interpretó como expresión regular (--regex)

## info

info-archive = archivo:
info-format = formato:
info-size = tamaño del archivo:
info-files = archivos:
info-dirs = carpetas:
info-uncompressed = descomprimido:
info-ratio = tasa de compresión:
info-nodes = nodos del árbol:
info-in-memory = en memoria:
info-in-memory-value = { $size } (el tar.gz se expande por completo al abrirlo)
info-solid = solid:
info-rar-solid-yes = sí (leer un archivo exige recorrer todo el flujo anterior)
info-7z-solid-yes = sí (leer un archivo expande todo su bloque)
info-solid-no = no (cada archivo se descomprime por separado)
info-headers = índice:
info-headers-encrypted = cifrado
info-blocks = bloques solid:
info-encrypted = cifradas:
info-encrypted-value =
    { $count ->
        [one] { $count } entrada
       *[other] { $count } entradas
    }
info-parse-time = el análisis tardó:

## verify

verify-failure = ERROR  { $path }: { $reason }
verify-summary = archivos comprobados: { $ok } | errores: { $errors } | leídos { $size } en { $seconds } s ({ $speed } MB/s)
verify-random = acceso aleatorio
verify-no-checksum =
    { $count ->
        [one] leída sin suma de control con la que comparar: { $count } entrada
       *[other] leídas sin suma de control con la que comparar: { $count } entradas
    }
verify-no-checksum-tar =
    tar no guarda sumas de control del contenido — no hay nada con qué comparar.
    Se comprobó que cada entrada se lee completa y no sale del archivo.
verify-no-checksum-targz =
    tar no guarda sumas de control del contenido, pero el CRC32 de todo el flujo
    gzip se comprobó al descomprimir — un daño en el archivo se habría notado.
verify-no-checksum-rar =
    son entradas RAR5 cifradas sin índice cifrado: el formato altera su suma
    de control a propósito para que no sirva para adivinar la contraseña.
    No hay nada con qué comparar — los datos siguen siendo correctos.
verify-no-checksum-zip =
    son entradas WinZip AE-2: su campo CRC está vacío a propósito, y la
    integridad la confirma el HMAC comprobado al descifrar.
err-verify-failed =
    { $count ->
        [one] la comprobación falló: { $count } entrada
       *[other] la comprobación falló: { $count } entradas
    }

## mount, unmount, mounts

mount-already = Ya montado: { $letter }
mount-done = Montado: { $letter }
mount-done-stats =
    Montado: { $letter }  ({ $files ->
        [one] { $files } archivo
       *[other] { $files } archivos
    }, { $dirs ->
        [one] { $dirs } carpeta
       *[other] { $dirs } carpetas
    }, { $size } de contenido)
mount-parse-time = El análisis del archivo tardó { $seconds } s.
mount-stop-hint = Ctrl+C o `zipmount unmount { $letter }` para desmontar.
mount-unmounting = Desmontando...
mount-finished = Listo. El archivo comprimido no ha cambiado.
err-no-free-letters = no hay letras de unidad libres
err-spawn-password = no se puede pasar la contraseña al proceso en segundo plano
err-mount-failed-detached =
    no se puede montar { $path }.
    Ejecute sin --detach para ver el motivo.
err-letter-busy =
    la letra { $letter } ya la usa otra unidad.
    Libres: { $free }
letters-none = ninguna letra libre
unmount-done = Desmontado: { $letter }
err-unmount-not-responding = el proceso de montaje de { $letter } no responde; se quitó su registro de la lista
err-unmount-timeout = { $letter } no se desmontó a tiempo: puede que haya archivos abiertos en ella
err-not-mounted = { $target } no figura como montado. Lista: zipmount mounts
mounts-none = No hay nada montado.

## Búsqueda desde el menú contextual

search-archive = Archivo: { $path }
search-stats =
    { $files ->
        [one] { $files } archivo
       *[other] { $files } archivos
    }, { $size } descomprimido. Formato: { $format }.
search-intro = La búsqueda recorre el contenido de los archivos. Una línea vacía termina.
search-prompt = Buscar:
search-summary = archivos con coincidencias: { $matched } de { $scanned } examinados, en { $seconds } s
search-truncated = (salida recortada)
search-error = Error de búsqueda: { $error }
press-enter = Pulse Intro para cerrar...

## Elementos del menú contextual — también se escriben en el registro

menu-mount = Montar como unidad
menu-mount-as = Montar en una letra
menu-search = Buscar en el archivo…
menu-unmount = Desmontar (ZipMount)
menu-unmount-letter = Desmontar { $letter }: (ZipMount)

## shell-install, shell-uninstall

shell-installed = Elementos añadidos al menú contextual para: { $extensions }
shell-installed-items =
    { menu-mount } — una letra libre, abierta en el Explorador de archivos
    { menu-mount-as } — un submenú para elegir
    { menu-search } — búsqueda por contenido
    { menu-unmount } — en el menú de la propia unidad montada
shell-installed-where =
    En Windows 11 estos elementos están en "Mostrar más opciones"
    — o aparecen directamente con Mayús+clic derecho.
shell-modern-hint = El menú principal sin Mayús: zipmount shell-install --modern
shell-remove-hint = Para quitarlos: zipmount shell-uninstall
shell-removed = Elementos quitados del menú contextual.

## El menú moderno

modern-building = Compilando el paquete…
modern-trust-intro =
    Queda un paso con permisos de administrador.

    Windows solo da un lugar en el menú principal a un paquete firmado, y
    confiar en un certificado es una decisión para todo el equipo; de ahí la
    solicitud de elevación. El certificado es autofirmado y está aquí:
modern-trust-uac = Ahora aparecerá una ventana del Control de cuentas de usuario.
modern-registering = Instalando el paquete…
modern-done =
    Listo. Elementos en el menú contextual principal:

      { menu-mount } — sobre un archivo comprimido
      { menu-mount-as } — un submenú solo con letras libres
      { menu-search } — búsqueda por contenido
      { menu-unmount } — en un espacio vacío dentro de la unidad
modern-installed-to = El programa está instalado en { $path }
modern-rebuild-hint = Después de recompilar, vuelva a ejecutar: zipmount shell-install --modern
modern-files-left = Los archivos siguen en { $path }
modern-cert-left = El certificado sigue siendo de confianza. Para quitarlo (hace falta un administrador):
err-sdk-tool-missing =
    No se encuentra { $tool }. Viene con el Windows SDK — instálelo,
    por ejemplo: winget install Microsoft.WindowsSDK.10.0.26100
err-exe-busy =
    no se puede actualizar { $path }: el archivo está en uso.
    Desmonte las unidades (zipmount unmount …) y vuelva a intentarlo.
err-shell-dll-missing =
    zipmount_shell.dll no está junto al programa.
    Compílelo: cargo build --release
err-bin-dir =
    no se puede crear { $path }.
    Si la carpeta ya existe, puede haber quedado de otra cuenta —
    en ese caso, elimínela o cámbiele el nombre.
err-cert-not-trusted =
    el certificado no llegó a ser de confianza.
    Sin eso Windows no acepta el paquete. El menú del registro funciona
    sin administrador: zipmount shell-install
err-handler-create = no se puede crear la clase COM del controlador
err-handler-title = el controlador no devolvió ningún título

## language

language-current = Idioma: { $name } ({ $code }) — { $source }
language-source-environment = definido por ZIPMOUNT_LANG
language-source-saved = elegido con zipmount language
language-source-windows = el idioma de visualización de Windows
language-source-default = el predeterminado
language-available = Disponibles:
language-hint = Elija uno: zipmount language <código>. Volver a seguir a Windows: zipmount language auto
language-set = Idioma: { $name } ({ $code }).
language-follows-windows = El idioma vuelve a seguir a Windows: { $name } ({ $code }).
language-menu-updated =
    { $count ->
        [0] El registro no tiene elementos de menú que reescribir; el menú moderno adopta el idioma por sí solo.
        [one] Se reescribió en este idioma un tipo de elemento de menú del registro; el menú moderno lo adopta por sí solo.
       *[other] Se reescribieron en este idioma { $count } tipos de elementos de menú del registro; el menú moderno lo adopta por sí solo.
    }
language-env-overrides = Nota: ZIPMOUNT_LANG={ $value } está definida y, en esta consola, sigue decidiendo el idioma.
err-language-unknown = idioma desconocido "{ $code }". Disponibles: { $available }; o auto para seguir a Windows
err-language-save = no se puede guardar la elección de idioma

## doctor

doctor-winfsp = Biblioteca WinFsp:
doctor-winfsp-found = encontrada ({ $path })
doctor-winfsp-missing = no encontrada
doctor-rar = Compatibilidad con rar:
doctor-rar-yes = sí (una compilación personal con --features rar; no debe distribuirse)
doctor-rar-no = no (compilación oficial: UnRAR es incompatible con la GPL)
doctor-language = Idioma:
doctor-modern = Menú moderno:
doctor-modern-ok = funciona ("{ $title }")
doctor-modern-silent = el paquete está, pero el controlador no responde — { $error }
doctor-modern-missing = no instalado (zipmount shell-install --modern)
doctor-init = Inicialización:
doctor-init-ok = correcta
doctor-init-failed = FALLÓ
doctor-ready =
    Todo listo. Monte un archivo comprimido:
        zipmount mount <archivo.zip> Z:
doctor-reason = Motivo: { $error }
doctor-no-winfsp-note =
    Explorar, buscar y extraer (ls, find, grep, verify) también funciona sin
    WinFsp — el controlador solo hace falta para montar una unidad.

## Linux y macOS: sin letras de unidad y sin Explorador de archivos

help-about-unix = Un archivo comprimido como carpeta: explorar, buscar y extraer sin descomprimir
help-notice-unix = Licencia GPL-3.0-or-later, código fuente: https://github.com/marlogg74mp/zipmount
help-prefix-unix = Prefijo de las rutas en la salida, por ejemplo "/home/me/ZipMount/logs/"
help-mount-mountpoint-unix = Un directorio vacío donde montar; ~/ZipMount/<nombre del archivo> si se omite
help-mount-open-unix = Abrir la carpeta montada en el gestor de archivos
help-unmount-unix = Desmontar un archivo comprimido
help-unmount-target-unix = El directorio de montaje o la ruta del archivo comprimido
help-language-code-unix = Código de idioma (en, ru, zh-CN, ja, ko, pt-BR, es, de), o auto para seguir al sistema
language-source-system = el idioma del sistema
language-hint-unix = Elija uno: zipmount language <código>. Volver a seguir al sistema: zipmount language auto
language-follows-system = El idioma vuelve a seguir al sistema: { $name } ({ $code }).
err-language-unknown-unix = idioma desconocido "{ $code }". Disponibles: { $available }; o auto para seguir al sistema
mount-err-fuse = no se puede montar en { $mountpoint }: { $error }
err-no-fuse = FUSE no está disponible aquí: falta { $missing }. Instale el paquete fuse3, por ejemplo:  sudo apt install fuse3
err-mountpoint-not-dir = { $path } no es un directorio
err-mountpoint-not-empty = { $path } no está vacío; monte en un directorio vacío
err-unmount-failed = no se puede desmontar { $target }: { $error }
err-unmount-in-use = no se puede desmontar { $target }: hay archivos abiertos en { $programs }. Ciérrelos e inténtelo de nuevo
err-mount-unsupported = el montaje aún no está disponible en este sistema
doctor-fuse = FUSE:
doctor-fuse-ok = disponible (/dev/fuse, fusermount3)
mount-err-nfs = no se puede montar en { $mountpoint }: { $error }
doctor-nfs-ok = disponible (el cliente NFS del sistema)
doctor-fuse-note =
    Explorar, buscar y extraer (ls, find, grep, verify) también funciona sin
    FUSE — solo hace falta para montar. Instálelo con el gestor de paquetes:
    sudo apt install fuse3 (Debian, Ubuntu), sudo dnf install fuse3 (Fedora),
    sudo pacman -S fuse3 (Arch).
doctor-mount = Montaje:
doctor-mount-unsupported-note =
    Explorar, buscar y extraer (ls, find, grep, verify) funciona en este
    sistema; el montaje aún no está disponible aquí.
doctor-ready-unix =
    Todo listo. Monte un archivo comprimido:
        zipmount mount <archivo.zip>

## Menú del gestor de archivos en Linux y macOS

menu-mount-unix = Montar con ZipMount
help-shell-install-unix = Añadir "Montar con ZipMount" al menú contextual del gestor de archivos
shell-installed-unix = Elementos de menú instalados:
shell-installed-where-macos = En el Finder: clic secundario en un archivo comprimido → Acciones rápidas → { menu-mount-unix }.
shell-installed-where-linux =
    Archivos de GNOME: clic derecho en un archivo comprimido → Scripts → { menu-mount-unix }.
    Dolphin (KDE): clic derecho en un archivo comprimido → { menu-mount-unix }.
    Cualquier gestor de archivos: Abrir con → { menu-mount-unix }.
language-menu-updated-unix = Los elementos de menú del gestor de archivos se reescribieron en este idioma.
