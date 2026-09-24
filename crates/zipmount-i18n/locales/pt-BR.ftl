# ZipMount — português (Brasil). Machine-assisted translation; corrections are welcome.
#
# Não traduza "WinFsp - Windows File System Proxy, Copyright (C) Bill Zissimopoulos":
# a licença do WinFsp exige exatamente esse aviso.

## Números e unidades

decimal-separator = ,
unit-b = B
unit-kb = KB
unit-mb = MB
unit-gb = GB
unit-tb = TB
seconds = { $value } s

## Abrir arquivos compactados

core-open-failed = não foi possível abrir o arquivo compactado { $path }
core-unknown-format = não foi possível identificar o formato de { $path }: não é zip, 7z nem tar
core-rar-not-built = { $path } é um arquivo RAR, e esta versão não tem suporte a rar: a licença do UnRAR é incompatível com a GPL, então o rar só é ativado ao compilar a partir do código-fonte para uso próprio (cargo build --release --features zipmount/rar)
core-unknown-encoding = codificação desconhecida "{ $value }"; válidas: auto, utf8, cp866, cp1251
core-deflate-corrupt = os dados deflate estão danificados (código zlib { $code })
core-password-missing = o arquivo compactado está criptografado: é preciso uma senha (-p ou --password-stdin)
core-password-wrong = senha incorreta

## Ler arquivos compactados

core-mmap-failed = não foi possível mapear o arquivo compactado na memória
core-entry-out-of-bounds = os dados da entrada ultrapassam o fim do arquivo compactado
core-zip-structure = não foi possível analisar a estrutura do arquivo zip
core-zip-too-small = o arquivo é pequeno demais para ser um zip: { $size } bytes
core-zip-no-eocd = registro de fim do diretório central não encontrado — o arquivo não é zip ou está danificado
core-zip64-missing = o arquivo está marcado como zip64, mas falta o registro final zip64
core-zip-cd-out-of-bounds = o diretório central ultrapassa o fim do arquivo
core-zip-bad-local-header = cabeçalho local danificado no deslocamento { $offset }
core-zip-method = método de compressão { $method } não suportado (stored e deflate são suportados)
core-deflate-failed = erro de descompressão deflate: { $error }
core-aes-unknown-strength = força AES desconhecida na entrada: { $strength }
core-encrypted-too-short = a entrada criptografada é menor que seus próprios campos de cabeçalho
core-aes-auth-failed = o código de autenticação não confere: a entrada está danificada
core-7z-structure = não foi possível analisar a estrutura do arquivo 7z { $path }; se ele for protegido por senha, informe a senha com -p
core-7z-structure-password = não foi possível analisar a estrutura do arquivo 7z { $path }: a senha pode estar errada, ou o arquivo danificado
core-7z-block-failed = não foi possível expandir o bloco { $block }
core-7z-block-error = erro ao descompactar o bloco { $block }: { $error }
core-tar-structure = não foi possível analisar a estrutura do arquivo tar
core-tar-bad-header = o primeiro cabeçalho tar não passa na verificação da soma de controle
core-gzip-not-tar = é um gzip sem tar dentro — esse tipo de arquivo não é suportado
core-gzip-damaged = erro de descompressão gzip (o arquivo está danificado ou truncado)
core-targz-too-large = o tar.gz expandido não cabe na memória permitida: { $used } GB até agora, com limite de { $limit } GB. Aumente o limite com --cache-mb
core-rar-error = erro ao ler o arquivo RAR: { $error }
core-grep-substring = não foi possível montar a busca pela substring: { $pattern }
core-grep-regex = expressão regular inválida: { $pattern }
core-grep-block-errors = erros ao percorrer os blocos: { $errors }
core-verify-short-read = a leitura em { $offset } retornou { $got } bytes em vez de { $want }
core-verify-crc = CRC32 não confere: obtido { $actual }, esperado { $expected }
core-verify-block = <bloco { $block }>

## Montagem via WinFsp

mount-err-create = não foi possível criar o volume WinFsp: { $error }
mount-err-mount = não foi possível montar em { $mountpoint }: { $error }
mount-err-dispatcher = não foi possível iniciar o despachante do WinFsp: { $error }
mount-err-no-winfsp = WinFsp não encontrado. Instale-o com:  winget install WinFsp.WinFsp

## Erros comuns

error-prefix = Erro
err-create-dir = não foi possível criar { $path }
err-write = não foi possível gravar { $path }
err-read = não foi possível ler { $path }
err-read-password = não foi possível ler a senha
err-read-password-stdin = não foi possível ler a senha da entrada padrão
err-spawn-background = não foi possível iniciar o processo em segundo plano
err-run-failed = não foi possível iniciar { $tool }
err-tool-failed =
    { $tool } falhou:
    { $output }

## Ajuda da linha de comando

help-about = Um arquivo compactado como unidade do Windows: navegue, pesquise e extraia sem descompactar
help-notice =
    Licença GPL-3.0-or-later, código-fonte: https://github.com/marlogg74mp/zipmount

    A montagem usa WinFsp - Windows File System Proxy, Copyright (C) Bill Zissimopoulos — https://github.com/winfsp/winfsp
help-heading-usage = Uso:
help-heading-commands = Comandos
help-heading-arguments = Argumentos
help-heading-options = Opções
help-flag-help = Mostrar a ajuda
help-flag-version = Mostrar a versão
help-archive = Caminho do arquivo compactado
help-encoding = Codificação dos nomes no zip: auto, utf8, cp866, cp1251
help-password = Pedir a senha do arquivo (digitação oculta)
help-password-stdin = Ler a senha da entrada padrão (a primeira linha)
help-prefix = Prefixo dos caminhos na saída, por exemplo "Z:\"
help-mount = Montar um arquivo compactado como unidade
help-mount-mountpoint = Letra de unidade (Z:) ou caminho de uma pasta NTFS vazia; se omitido, a primeira letra livre
help-mount-detach = Montar em segundo plano e retornar imediatamente
help-mount-open = Abrir a unidade montada no Explorador de Arquivos
help-mount-label = Rótulo do volume; por padrão, o nome do arquivo compactado
help-mount-cache-mb = Limite do cache de dados descompactados, MB (no 7z, o cache de blocos solid)
help-ls = Listar uma pasta dentro do arquivo compactado
help-ls-path = Caminho dentro do arquivo; por padrão, a raiz
help-ls-recursive = Recursivamente, a árvore inteira
help-find = Encontrar arquivos por um padrão de nome
help-find-pattern = Padrão, por exemplo "*.log"
help-grep = Pesquisar o conteúdo dos arquivos dentro do arquivo compactado
help-grep-pattern = Substring a procurar (ou uma expressão regular com --regex)
help-grep-regex = Tratar o padrão como expressão regular
help-grep-files-only = Apenas os caminhos dos arquivos, sem as linhas
help-grep-count = Apenas o número de ocorrências em cada arquivo
help-grep-ignore-case = Ignorar maiúsculas e minúsculas
help-grep-after-context = Linhas de contexto após uma ocorrência
help-grep-before-context = Linhas de contexto antes de uma ocorrência
help-grep-context = Linhas de contexto dos dois lados
help-grep-max-count = No máximo N ocorrências por arquivo
help-grep-max-total = No máximo N linhas na saída inteira
help-grep-path = Pesquisar apenas dentro deste ramo do arquivo compactado
help-grep-glob = Restringir por um padrão de nome, por exemplo "*.log"
help-grep-copy-to = Extrair os arquivos encontrados para esta pasta
help-info = Resumo de um arquivo compactado
help-verify = Verificação de leitura de ponta a ponta de cada entrada pelo seu CRC32
help-verify-random = Ler em ordem embaralhada em vez de sequencial: testa o retrocesso
help-unmount = Desmontar uma unidade
help-unmount-target = Letra de unidade (Z:) ou caminho do arquivo compactado
help-mounts = Mostrar os arquivos compactados montados
help-search = Pesquisa interativa em um arquivo compactado (iniciada pelo menu de contexto)
help-shell-install = Adicionar itens ao menu de contexto do Explorador de Arquivos
help-shell-install-modern = O menu principal do Windows 11, sem "Mostrar mais opções" (pede direitos de administrador uma vez)
help-shell-uninstall = Remover os itens do menu de contexto
help-language = Mostrar ou escolher o idioma do programa
help-language-code = Código do idioma (en, ru, zh-CN, ja, ko, pt-BR, es, de), ou auto para seguir o Windows
help-doctor = Verificar se o ambiente está pronto para montar

## ls, find, grep

password-prompt = Senha do arquivo compactado:
err-path-not-found = caminho não encontrado no arquivo compactado: { $path }
find-summary = encontrados: { $count }
grep-copied = arquivos extraídos: { $count } -> { $path }
grep-summary = arquivos com ocorrências: { $matched } | analisados: { $scanned } | ignorados: { $skipped } | descompactado: { $size } em { $seconds } s ({ $speed } MB/s)
grep-truncated = saída cortada no limite (--max-total)
grep-regex-note = o padrão foi interpretado como expressão regular (--regex)

## info

info-archive = arquivo:
info-format = formato:
info-size = tamanho do arquivo:
info-files = arquivos:
info-dirs = pastas:
info-uncompressed = descompactado:
info-ratio = taxa de compressão:
info-nodes = nós da árvore:
info-in-memory = na memória:
info-in-memory-value = { $size } (o tar.gz é expandido por inteiro ao ser aberto)
info-solid = solid:
info-rar-solid-yes = sim (ler um arquivo exige percorrer todo o fluxo anterior)
info-7z-solid-yes = sim (ler um arquivo expande o bloco inteiro dele)
info-solid-no = não (cada arquivo é descompactado de forma independente)
info-headers = índice:
info-headers-encrypted = criptografado
info-blocks = blocos solid:
info-encrypted = criptografadas:
info-encrypted-value =
    { $count ->
        [one] { $count } entrada
       *[other] { $count } entradas
    }
info-parse-time = a análise levou:

## verify

verify-failure = ERRO  { $path }: { $reason }
verify-summary = arquivos verificados: { $ok } | erros: { $errors } | lidos { $size } em { $seconds } s ({ $speed } MB/s)
verify-random = acesso embaralhado
verify-no-checksum =
    { $count ->
        [one] lida sem soma de controle para comparar: { $count } entrada
       *[other] lidas sem soma de controle para comparar: { $count } entradas
    }
verify-no-checksum-tar =
    o tar não guarda somas de controle do conteúdo — não há com o que comparar.
    O que foi verificado: toda entrada é lida por inteiro e fica dentro do arquivo.
verify-no-checksum-targz =
    o tar não guarda somas de controle do conteúdo, mas o CRC32 de todo o fluxo
    gzip foi conferido na descompressão — um dano no arquivo teria aparecido.
verify-no-checksum-rar =
    são entradas RAR5 criptografadas sem índice criptografado: o formato
    embaralha a soma de controle de propósito, para que ela não sirva para
    adivinhar a senha. Não há com o que comparar — os dados estão corretos.
verify-no-checksum-zip =
    são entradas WinZip AE-2: o campo CRC fica vazio de propósito, e a
    integridade é confirmada pelo HMAC verificado na descriptografia.
err-verify-failed =
    { $count ->
        [one] verificação falhou: { $count } entrada
       *[other] verificação falhou: { $count } entradas
    }

## mount, unmount, mounts

mount-already = Já montado: { $letter }
mount-done = Montado: { $letter }
mount-done-stats =
    Montado: { $letter }  ({ $files ->
        [one] { $files } arquivo
       *[other] { $files } arquivos
    }, { $dirs ->
        [one] { $dirs } pasta
       *[other] { $dirs } pastas
    }, { $size } de conteúdo)
mount-parse-time = A análise do arquivo levou { $seconds } s.
mount-stop-hint = Ctrl+C ou `zipmount unmount { $letter }` para desmontar.
mount-unmounting = Desmontando...
mount-finished = Pronto. O arquivo compactado não foi alterado.
err-no-free-letters = não há letras de unidade livres
err-spawn-password = não foi possível passar a senha ao processo em segundo plano
err-mount-failed-detached =
    não foi possível montar { $path }.
    Execute sem --detach para ver o motivo.
err-letter-busy =
    a letra { $letter } já está em uso por outra unidade.
    Livres: { $free }
letters-none = nenhuma letra livre
unmount-done = Desmontado: { $letter }
err-unmount-not-responding = o processo de montagem de { $letter } não responde; o registro foi removido da lista
err-unmount-timeout = { $letter } não foi desmontado a tempo: pode haver arquivos abertos nele
err-not-mounted = { $target } não consta como montado. Lista: zipmount mounts
mounts-none = Nada está montado.

## Pesquisa pelo menu de contexto

search-archive = Arquivo: { $path }
search-stats =
    { $files ->
        [one] { $files } arquivo
       *[other] { $files } arquivos
    }, { $size } descompactado. Formato: { $format }.
search-intro = A pesquisa é feita no conteúdo dos arquivos. Uma linha vazia encerra.
search-prompt = Pesquisar:
search-summary = arquivos com ocorrências: { $matched } de { $scanned } analisados, em { $seconds } s
search-truncated = (saída cortada)
search-error = Erro na pesquisa: { $error }
press-enter = Pressione Enter para fechar...

## Itens do menu de contexto — também gravados no registro

menu-mount = Montar como unidade
menu-mount-as = Montar em uma letra
menu-search = Pesquisar no arquivo…
menu-unmount = Desmontar (ZipMount)
menu-unmount-letter = Desmontar { $letter }: (ZipMount)

## shell-install, shell-uninstall

shell-installed = Itens adicionados ao menu de contexto para: { $extensions }
shell-installed-items =
    { menu-mount } — uma letra livre, aberta no Explorador de Arquivos
    { menu-mount-as } — um submenu para escolher
    { menu-search } — pesquisa pelo conteúdo
    { menu-unmount } — no menu da própria unidade montada
shell-installed-where =
    No Windows 11 esses itens ficam em "Mostrar mais opções"
    — ou aparecem direto com Shift+clique com o botão direito.
shell-modern-hint = O menu principal sem Shift: zipmount shell-install --modern
shell-remove-hint = Para remover: zipmount shell-uninstall
shell-removed = Itens removidos do menu de contexto.

## O menu moderno

modern-building = Compilando o pacote…
modern-trust-intro =
    Falta uma etapa com direitos de administrador.

    O Windows só dá lugar no menu principal a um pacote assinado, e confiar
    em um certificado é uma decisão para a máquina inteira — daí o pedido de
    elevação. O certificado é autoassinado e está aqui:
modern-trust-uac = Uma janela do Controle de Conta de Usuário vai aparecer agora.
modern-registering = Instalando o pacote…
modern-done =
    Pronto. Itens no menu de contexto principal:

      { menu-mount } — em um arquivo compactado
      { menu-mount-as } — um submenu só com letras livres
      { menu-search } — pesquisa pelo conteúdo
      { menu-unmount } — em um espaço vazio dentro da unidade
modern-installed-to = O programa está instalado em { $path }
modern-rebuild-hint = Depois de recompilar, execute de novo: zipmount shell-install --modern
modern-files-left = Os arquivos continuam em { $path }
modern-cert-left = O certificado continua confiável. Para removê-lo (é preciso um administrador):
err-sdk-tool-missing =
    { $tool } não encontrado. Ele vem com o Windows SDK — instale-o,
    por exemplo: winget install Microsoft.WindowsSDK.10.0.26100
err-exe-busy =
    não foi possível atualizar { $path }: o arquivo está em uso.
    Desmonte as unidades (zipmount unmount …) e tente de novo.
err-shell-dll-missing =
    o zipmount_shell.dll não está ao lado do programa.
    Compile-o: cargo build --release
err-bin-dir =
    não foi possível criar { $path }.
    Se a pasta já existe, pode ter sobrado de outra conta —
    nesse caso, exclua-a ou renomeie-a.
err-cert-not-trusted =
    o certificado não se tornou confiável.
    Sem isso o Windows não aceita o pacote. O menu pelo registro funciona
    sem administrador: zipmount shell-install
err-handler-create = não foi possível criar a classe COM do manipulador
err-handler-title = o manipulador não retornou um título

## language

language-current = Idioma: { $name } ({ $code }) — { $source }
language-source-environment = definido por ZIPMOUNT_LANG
language-source-saved = escolhido com zipmount language
language-source-windows = o idioma de exibição do Windows
language-source-default = o padrão
language-available = Disponíveis:
language-hint = Escolha um: zipmount language <código>. Voltar a seguir o Windows: zipmount language auto
language-set = Idioma: { $name } ({ $code }).
language-follows-windows = O idioma volta a seguir o Windows: { $name } ({ $code }).
language-menu-updated =
    { $count ->
        [0] O registro não tem itens de menu para reescrever; o menu moderno adota o idioma sozinho.
        [one] Um tipo de item de menu no registro foi reescrito neste idioma; o menu moderno adota o idioma sozinho.
       *[other] { $count } tipos de itens de menu no registro foram reescritos neste idioma; o menu moderno adota o idioma sozinho.
    }
language-env-overrides = Observação: ZIPMOUNT_LANG={ $value } está definida e, neste console, continua decidindo o idioma.
err-language-unknown = idioma desconhecido "{ $code }". Disponíveis: { $available }; ou auto para seguir o Windows
err-language-save = não foi possível salvar a escolha de idioma

## doctor

doctor-winfsp = Biblioteca WinFsp:
doctor-winfsp-found = encontrada ({ $path })
doctor-winfsp-missing = não encontrada
doctor-rar = Suporte a rar:
doctor-rar-yes = sim (uma compilação pessoal com --features rar; não pode ser distribuída)
doctor-rar-no = não (versão oficial: o UnRAR é incompatível com a GPL)
doctor-language = Idioma:
doctor-modern = Menu moderno:
doctor-modern-ok = funciona ("{ $title }")
doctor-modern-silent = pacote presente, o manipulador não responde — { $error }
doctor-modern-missing = não instalado (zipmount shell-install --modern)
doctor-init = Inicialização:
doctor-init-ok = bem-sucedida
doctor-init-failed = FALHOU
doctor-ready =
    Tudo pronto. Monte um arquivo compactado:
        zipmount mount <arquivo.zip> Z:
doctor-reason = Motivo: { $error }
doctor-no-winfsp-note =
    Navegar, pesquisar e extrair (ls, find, grep, verify) também funciona sem
    o WinFsp — o driver só é necessário para montar uma unidade.

## Linux e macOS: sem letras de unidade e sem Explorador de Arquivos

help-about-unix = Um arquivo compactado como pasta: navegue, pesquise e extraia sem descompactar
help-notice-unix = Licença GPL-3.0-or-later, código-fonte: https://github.com/marlogg74mp/zipmount
help-prefix-unix = Prefixo dos caminhos na saída, por exemplo "/home/me/ZipMount/logs/"
help-mount-mountpoint-unix = Um diretório vazio onde montar; ~/ZipMount/<nome do arquivo> se omitido
help-mount-open-unix = Abrir a pasta montada no gerenciador de arquivos
help-unmount-unix = Desmontar um arquivo compactado
help-unmount-target-unix = O diretório de montagem ou o caminho do arquivo compactado
help-language-code-unix = Código do idioma (en, ru, zh-CN, ja, ko, pt-BR, es, de), ou auto para seguir o sistema
language-source-system = o idioma do sistema
language-hint-unix = Escolha um: zipmount language <código>. Voltar a seguir o sistema: zipmount language auto
language-follows-system = O idioma volta a seguir o sistema: { $name } ({ $code }).
err-language-unknown-unix = idioma desconhecido "{ $code }". Disponíveis: { $available }; ou auto para seguir o sistema
mount-err-fuse = não foi possível montar em { $mountpoint }: { $error }
err-no-fuse = O FUSE não está disponível aqui: falta { $missing }. Instale o pacote fuse3, por exemplo:  sudo apt install fuse3
err-mountpoint-not-dir = { $path } não é um diretório
err-mountpoint-not-empty = { $path } não está vazio; monte em um diretório vazio
err-unmount-failed = não foi possível desmontar { $target }: { $error }
err-unmount-in-use = não foi possível desmontar { $target }: há arquivos abertos em { $programs }. Feche-os e tente de novo
err-mount-unsupported = a montagem ainda não está disponível neste sistema
doctor-fuse = FUSE:
doctor-fuse-ok = disponível (/dev/fuse, fusermount3)
mount-err-nfs = não foi possível montar em { $mountpoint }: { $error }
doctor-nfs-ok = disponível (o cliente NFS do sistema)
doctor-fuse-note =
    Navegar, pesquisar e extrair (ls, find, grep, verify) também funciona sem
    o FUSE — ele só é necessário para montar. Instale-o com o gerenciador de
    pacotes: sudo apt install fuse3 (Debian, Ubuntu), sudo dnf install fuse3
    (Fedora), sudo pacman -S fuse3 (Arch).
doctor-mount = Montagem:
doctor-mount-unsupported-note =
    Navegar, pesquisar e extrair (ls, find, grep, verify) funciona neste
    sistema; a montagem ainda não está disponível aqui.
doctor-ready-unix =
    Tudo pronto. Monte um arquivo compactado:
        zipmount mount <arquivo.zip>
