# ZipMount — 简体中文。Machine-assisted translation; corrections are welcome.
#
# 请勿翻译 "WinFsp - Windows File System Proxy, Copyright (C) Bill Zissimopoulos"：
# WinFsp 的许可证要求原样保留这句声明。

## 数字与单位

decimal-separator = .
unit-b = B
unit-kb = KB
unit-mb = MB
unit-gb = GB
unit-tb = TB
seconds = { $value } 秒

## 打开归档

core-open-failed = 无法打开归档 { $path }
core-unknown-format = 无法识别 { $path } 的格式：它不是 zip、7z 或 tar
core-rar-not-built = { $path } 是 RAR 归档，而此版本不支持 rar：UnRAR 许可证与 GPL 不兼容，因此只有从源代码自行构建时才能启用 rar（cargo build --release --features zipmount/rar）
core-unknown-encoding = 未知编码“{ $value }”；可用值：auto、utf8、cp866、cp1251
core-deflate-corrupt = deflate 数据已损坏（zlib 代码 { $code }）
core-password-missing = 归档已加密：需要密码（-p 或 --password-stdin）
core-password-wrong = 密码错误

## 读取归档

core-mmap-failed = 无法将归档映射到内存
core-entry-out-of-bounds = 条目数据超出了归档末尾
core-zip-structure = 无法解析 zip 归档的结构
core-zip-too-small = 文件太小，不可能是 zip：{ $size } 字节
core-zip-no-eocd = 找不到中央目录结束记录——该文件不是 zip 归档，或已损坏
core-zip64-missing = 归档标记为 zip64，但缺少 zip64 结束记录
core-zip-cd-out-of-bounds = 中央目录超出了文件末尾
core-zip-bad-local-header = 偏移 { $offset } 处的本地文件头已损坏
core-zip-method = 不支持的压缩方法 { $method }（支持 stored 和 deflate）
core-deflate-failed = deflate 解压错误：{ $error }
core-aes-unknown-strength = 条目中的 AES 强度未知：{ $strength }
core-encrypted-too-short = 加密条目比其自身的头部字段还短
core-aes-auth-failed = 认证码不匹配：条目已损坏
core-7z-structure = 无法解析 7z 归档 { $path } 的结构；如果它受密码保护，请用 -p 提供密码
core-7z-structure-password = 无法解析 7z 归档 { $path } 的结构：密码可能错误，或归档已损坏
core-7z-block-failed = 无法展开数据块 { $block }
core-7z-block-error = 解压数据块 { $block } 时出错：{ $error }
core-tar-structure = 无法解析 tar 归档的结构
core-tar-bad-header = 第一个 tar 头未通过校验和检查
core-gzip-not-tar = 这是 gzip 文件，但其中不是 tar——不支持此类归档
core-gzip-damaged = gzip 解压错误（归档已损坏或被截断）
core-targz-too-large = 展开后的 tar.gz 超出了允许的内存：已用 { $used } GB，上限 { $limit } GB。可用 --cache-mb 提高上限
core-rar-error = 读取 RAR 归档时出错：{ $error }
core-grep-substring = 无法为以下内容建立子串搜索：{ $pattern }
core-grep-regex = 无效的正则表达式：{ $pattern }
core-grep-block-errors = 遍历数据块时出错：{ $errors }
core-verify-short-read = 在 { $offset } 处读取返回 { $got } 字节，而不是 { $want } 字节
core-verify-crc = CRC32 不匹配：实际 { $actual }，应为 { $expected }
core-verify-block = <数据块 { $block }>

## 通过 WinFsp 挂载

mount-err-create = 无法创建 WinFsp 卷：{ $error }
mount-err-mount = 无法挂载到 { $mountpoint }：{ $error }
mount-err-dispatcher = 无法启动 WinFsp 调度器：{ $error }
mount-err-no-winfsp = 未找到 WinFsp。请用以下命令安装：  winget install WinFsp.WinFsp

## 常见错误

error-prefix = 错误
err-create-dir = 无法创建 { $path }
err-write = 无法写入 { $path }
err-read = 无法读取 { $path }
err-read-password = 无法读取密码
err-read-password-stdin = 无法从标准输入读取密码
err-spawn-background = 无法启动后台进程
err-run-failed = 无法启动 { $tool }
err-tool-failed =
    { $tool } 执行失败：
    { $output }

## 命令行帮助

help-about = 将归档作为 Windows 驱动器：无需解压即可浏览、搜索和提取
help-notice =
    许可证 GPL-3.0-or-later，源代码：https://github.com/marlogg74mp/zipmount

    挂载基于 WinFsp - Windows File System Proxy, Copyright (C) Bill Zissimopoulos — https://github.com/winfsp/winfsp
help-heading-usage = 用法：
help-heading-commands = 命令
help-heading-arguments = 参数
help-heading-options = 选项
help-flag-help = 显示帮助
help-flag-version = 显示版本
help-archive = 归档路径
help-encoding = zip 中文件名的编码：auto、utf8、cp866、cp1251
help-password = 询问归档密码（输入时隐藏）
help-password-stdin = 从标准输入读取密码（第一行）
help-prefix = 输出路径的前缀，例如 "Z:\"
help-mount = 将归档挂载为驱动器
help-mount-mountpoint = 驱动器号（Z:）或空 NTFS 目录的路径；省略时使用第一个空闲的驱动器号
help-mount-detach = 在后台挂载并立即返回
help-mount-open = 在文件资源管理器中打开挂载的驱动器
help-mount-label = 卷标；默认为归档的文件名
help-mount-cache-mb = 解压数据缓存的上限，MB（对 7z 而言是 solid 块缓存）
help-ls = 列出归档内的目录
help-ls-path = 归档内的路径；默认为根目录
help-ls-recursive = 递归列出整棵树
help-find = 按文件名模式查找文件
help-find-pattern = 模式，例如 "*.log"
help-grep = 搜索归档内文件的内容
help-grep-pattern = 要查找的子串（使用 --regex 时为正则表达式）
help-grep-regex = 将模式视为正则表达式
help-grep-files-only = 只显示文件路径，不显示行
help-grep-count = 只显示每个文件中的匹配数
help-grep-ignore-case = 忽略大小写
help-grep-after-context = 匹配后显示的上下文行数
help-grep-before-context = 匹配前显示的上下文行数
help-grep-context = 匹配前后显示的上下文行数
help-grep-max-count = 每个文件最多 N 个匹配
help-grep-max-total = 整个输出最多 N 行
help-grep-path = 只在归档的这个分支中搜索
help-grep-glob = 按文件名模式限定，例如 "*.log"
help-grep-copy-to = 将匹配的文件提取到此目录
help-info = 归档概要
help-verify = 按 CRC32 端到端校验每个条目的读取
help-verify-random = 以乱序而非顺序读取：用于测试向后定位
help-unmount = 卸载驱动器
help-unmount-target = 驱动器号（Z:）或归档路径
help-mounts = 显示已挂载的归档
help-search = 在归档中交互式搜索（从上下文菜单启动）
help-shell-install = 向文件资源管理器的上下文菜单添加菜单项
help-shell-install-modern = Windows 11 主菜单，无需“显示更多选项”（会请求一次管理员权限）
help-shell-uninstall = 从上下文菜单中移除菜单项
help-language = 显示或选择程序语言
help-language-code = 语言代码（en、ru、zh-CN、ja、ko、pt-BR、es、de），或 auto 以跟随 Windows
help-doctor = 检查挂载所需的环境是否就绪

## ls、find、grep

password-prompt = 归档密码：
err-path-not-found = 归档中找不到路径：{ $path }
find-summary = 找到：{ $count }
grep-copied = 已提取文件：{ $count } -> { $path }
grep-summary = 有匹配的文件：{ $matched } | 已扫描：{ $scanned } | 已跳过：{ $skipped } | 已解压：{ $size }，用时 { $seconds } 秒（{ $speed } MB/s）
grep-truncated = 输出已达上限而截断（--max-total）
grep-regex-note = 模式被视为正则表达式（--regex）

## info

info-archive = 归档：
info-format = 格式：
info-size = 归档大小：
info-files = 文件：
info-dirs = 目录：
info-uncompressed = 解压后：
info-ratio = 压缩比：
info-nodes = 树节点：
info-in-memory = 内存占用：
info-in-memory-value = { $size }（tar.gz 在打开时会完整展开）
info-solid = solid：
info-rar-solid-yes = 是（读取一个文件需要经过之前的整个数据流）
info-7z-solid-yes = 是（读取一个文件会展开其整个数据块）
info-solid-no = 否（每个文件独立解压）
info-headers = 目录表：
info-headers-encrypted = 已加密
info-blocks = solid 块：
info-encrypted = 已加密：
info-encrypted-value = { $count } 个条目
info-parse-time = 解析用时：

## verify

verify-failure = 错误  { $path }：{ $reason }
verify-summary = 已校验文件：{ $ok } | 错误：{ $errors } | 已读取 { $size }，用时 { $seconds } 秒（{ $speed } MB/s）
verify-random = 乱序访问
verify-no-checksum = 读取时无校验和可供比对：{ $count } 个条目
verify-no-checksum-tar =
    tar 不保存内容校验和——没有可比对的对象。
    已检查的是：每个条目都能完整读取，且不超出归档范围。
verify-no-checksum-targz =
    tar 不保存内容校验和，但整个 gzip 流的 CRC32 已在解压时核对——
    若归档损坏，本应已被发现。
verify-no-checksum-rar =
    这些是未加密目录表的 RAR5 加密条目：该格式有意打乱其校验和，
    使之无法用于猜测密码。没有可比对的对象——但数据仍然正确。
verify-no-checksum-zip =
    这些是 WinZip AE-2 条目：其 CRC 字段有意为空，
    完整性由解密时校验的 HMAC 保证。
err-verify-failed = 校验未通过：{ $count } 个条目

## mount、unmount、mounts

mount-already = 已挂载：{ $letter }
mount-done = 已挂载：{ $letter }
mount-done-stats = 已挂载：{ $letter }（{ $files } 个文件，{ $dirs } 个目录，内容共 { $size }）
mount-parse-time = 解析归档用时 { $seconds } 秒。
mount-stop-hint = 按 Ctrl+C 或运行 `zipmount unmount { $letter }` 卸载。
mount-unmounting = 正在卸载...
mount-finished = 完成。归档未被修改。
err-no-free-letters = 没有空闲的驱动器号
err-spawn-password = 无法将密码传给后台进程
err-mount-failed-detached =
    无法挂载 { $path }。
    请不带 --detach 运行以查看原因。
err-letter-busy =
    驱动器号 { $letter } 已被其他驱动器占用。
    空闲：{ $free }
letters-none = 没有空闲的驱动器号
unmount-done = 已卸载：{ $letter }
err-unmount-not-responding = { $letter } 的挂载进程没有响应；其记录已从列表中移除
err-unmount-timeout = { $letter } 未能及时卸载：其上的文件可能仍处于打开状态
err-not-mounted = { $target } 不在已挂载列表中。列表：zipmount mounts
mounts-none = 没有挂载任何内容。

## 从上下文菜单搜索

search-archive = 归档：{ $path }
search-stats = { $files } 个文件，解压后 { $size }。格式：{ $format }。
search-intro = 搜索的是文件内容。输入空行退出。
search-prompt = 搜索内容：
search-summary = 有匹配的文件：{ $matched } / 已扫描 { $scanned }，用时 { $seconds } 秒
search-truncated = （输出已截断）
search-error = 搜索出错：{ $error }
press-enter = 按 Enter 关闭...

## 上下文菜单项——也会写入注册表

menu-mount = 挂载为驱动器
menu-mount-as = 挂载到驱动器号
menu-search = 在归档中搜索…
menu-unmount = 卸载（ZipMount）
menu-unmount-letter = 卸载 { $letter }:（ZipMount）

## shell-install、shell-uninstall

shell-installed = 已为以下类型添加上下文菜单项：{ $extensions }
shell-installed-items =
    { menu-mount } — 使用空闲驱动器号，并在文件资源管理器中打开
    { menu-mount-as } — 可选择的子菜单
    { menu-search } — 按内容搜索
    { menu-unmount } — 位于已挂载驱动器本身的菜单中
shell-installed-where =
    在 Windows 11 中，这些菜单项位于“显示更多选项”之下
    ——或直接按 Shift+右键。
shell-modern-hint = 无需 Shift 的主菜单：zipmount shell-install --modern
shell-remove-hint = 移除：zipmount shell-uninstall
shell-removed = 已从上下文菜单中移除菜单项。

## 新式菜单

modern-building = 正在构建包…
modern-trust-intro =
    还剩一步需要管理员权限。

    Windows 只允许已签名的包进入主菜单，而信任证书是整台计算机
    范围的决定，因此需要提升权限。该证书为自签名证书，位于：
modern-trust-uac = 用户帐户控制窗口即将出现。
modern-registering = 正在安装包…
modern-done =
    完成。主上下文菜单中的菜单项：

      { menu-mount } — 在归档上
      { menu-mount-as } — 仅含空闲驱动器号的子菜单
      { menu-search } — 按内容搜索
      { menu-unmount } — 在驱动器内的空白处
modern-installed-to = 程序已安装到 { $path }
modern-rebuild-hint = 重新构建后，请再次运行：zipmount shell-install --modern
modern-files-left = 文件保留在 { $path }
modern-cert-left = 证书仍处于受信任状态。要移除它（需要管理员权限）：
err-sdk-tool-missing =
    找不到 { $tool }。它随 Windows SDK 提供——请安装，
    例如：winget install Microsoft.WindowsSDK.10.0.26100
err-exe-busy =
    无法更新 { $path }：文件正在使用中。
    请卸载驱动器（zipmount unmount …）后重试。
err-shell-dll-missing =
    程序旁边没有 zipmount_shell.dll。
    请构建它：cargo build --release
err-bin-dir =
    无法创建 { $path }。
    如果该目录已存在，它可能是其他帐户留下的——
    请删除或重命名它。
err-cert-not-trusted =
    证书始终未被信任。
    没有它，Windows 不会接受该包。注册表菜单无需
    管理员权限即可使用：zipmount shell-install
err-handler-create = 无法创建处理程序的 COM 类
err-handler-title = 处理程序未返回标题

## language

language-current = 语言：{ $name }（{ $code }）— { $source }
language-source-environment = 由 ZIPMOUNT_LANG 设置
language-source-saved = 通过 zipmount language 选择
language-source-windows = Windows 显示语言
language-source-default = 默认
language-available = 可用语言：
language-hint = 选择：zipmount language <代码>。重新跟随 Windows：zipmount language auto
language-set = 语言：{ $name }（{ $code }）。
language-follows-windows = 语言重新跟随 Windows：{ $name }（{ $code }）。
language-menu-updated =
    { $count ->
        [0] 注册表中没有需要改写的菜单项；新式菜单会自动采用新语言。
       *[other] 注册表中的 { $count } 类菜单项已用此语言改写；新式菜单会自动采用新语言。
    }
language-env-overrides = 注意：已设置 ZIPMOUNT_LANG={ $value }，在此控制台中仍由它决定语言。
err-language-unknown = 未知语言“{ $code }”。可用：{ $available }；或 auto 以跟随 Windows
err-language-save = 无法保存语言选择

## doctor

doctor-winfsp = WinFsp 库：
doctor-winfsp-found = 已找到（{ $path }）
doctor-winfsp-missing = 未找到
doctor-rar = rar 支持：
doctor-rar-yes = 有（使用 --features rar 的个人构建；不得分发）
doctor-rar-no = 无（官方构建：UnRAR 与 GPL 不兼容）
doctor-language = 语言：
doctor-modern = 新式菜单：
doctor-modern-ok = 正常（“{ $title }”）
doctor-modern-silent = 包存在，但处理程序没有响应——{ $error }
doctor-modern-missing = 未安装（zipmount shell-install --modern）
doctor-init = 初始化：
doctor-init-ok = 成功
doctor-init-failed = 失败
doctor-ready =
    一切就绪。挂载归档：
        zipmount mount <archive.zip> Z:
doctor-reason = 原因：{ $error }
doctor-no-winfsp-note =
    浏览、搜索和提取（ls、find、grep、verify）无需 WinFsp 也能工作——
    只有挂载驱动器才需要该驱动程序。

## Linux 和 macOS：没有驱动器号，也没有文件资源管理器

help-about-unix = 将归档作为文件夹：无需解压即可浏览、搜索和提取
help-notice-unix = 许可证 GPL-3.0-or-later，源代码：https://github.com/marlogg74mp/zipmount
help-prefix-unix = 输出路径的前缀，例如 "/home/me/ZipMount/logs/"
help-mount-mountpoint-unix = 用于挂载的空目录；省略时为 ~/ZipMount/<归档名>
help-mount-open-unix = 在文件管理器中打开挂载的文件夹
help-unmount-unix = 卸载归档
help-unmount-target-unix = 挂载目录或归档路径
help-language-code-unix = 语言代码（en、ru、zh-CN、ja、ko、pt-BR、es、de），或 auto 以跟随系统
language-source-system = 系统语言
language-hint-unix = 选择：zipmount language <代码>。重新跟随系统：zipmount language auto
language-follows-system = 语言重新跟随系统：{ $name }（{ $code }）。
err-language-unknown-unix = 未知语言“{ $code }”。可用：{ $available }；或 auto 以跟随系统
mount-err-fuse = 无法挂载到 { $mountpoint }：{ $error }
err-no-fuse = 此处无法使用 FUSE：缺少 { $missing }。请安装 fuse3 软件包，例如：  sudo apt install fuse3
err-mountpoint-not-dir = { $path } 不是目录
err-mountpoint-not-empty = { $path } 不是空目录；只能挂载到空目录
err-unmount-failed = 无法卸载 { $target }：{ $error }
err-mount-unsupported = 此系统暂不支持挂载
doctor-fuse = FUSE：
doctor-fuse-ok = 可用（/dev/fuse、fusermount3）
mount-err-nfs = 无法挂载到 { $mountpoint }：{ $error }
doctor-nfs-ok = 可用（系统自带的 NFS 客户端）
doctor-fuse-note =
    浏览、搜索和提取（ls、find、grep、verify）无需 FUSE 也能工作——
    只有挂载才需要它。可用包管理器安装：sudo apt install fuse3（Debian、
    Ubuntu）、sudo dnf install fuse3（Fedora）、sudo pacman -S fuse3（Arch）。
doctor-mount = 挂载：
doctor-mount-unsupported-note =
    浏览、搜索和提取（ls、find、grep、verify）在此系统上可以使用；
    挂载暂不可用。
doctor-ready-unix =
    一切就绪。挂载归档：
        zipmount mount <archive.zip>
