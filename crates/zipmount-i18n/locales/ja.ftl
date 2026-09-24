# ZipMount — 日本語. Machine-assisted translation; corrections are welcome.
#
# "WinFsp - Windows File System Proxy, Copyright (C) Bill Zissimopoulos" は翻訳しないこと：
# WinFsp のライセンスがこの表記をそのまま求めています。

## 数値と単位

decimal-separator = .
unit-b = B
unit-kb = KB
unit-mb = MB
unit-gb = GB
unit-tb = TB
seconds = { $value } 秒

## アーカイブを開く

core-open-failed = アーカイブ { $path } を開けません
core-unknown-format = { $path } の形式を判別できません：zip、7z、tar のいずれでもありません
core-rar-not-built = { $path } は RAR アーカイブですが、このビルドは rar に対応していません：UnRAR のライセンスは GPL と互換性がないため、rar は自分用にソースからビルドするときにのみ有効にできます（cargo build --release --features zipmount/rar）
core-unknown-encoding = 不明なエンコーディング「{ $value }」；使用可能な値：auto、utf8、cp866、cp1251
core-deflate-corrupt = deflate データが破損しています（zlib コード { $code }）
core-password-missing = アーカイブは暗号化されています：パスワードが必要です（-p または --password-stdin）
core-password-wrong = パスワードが違います

## アーカイブの読み込み

core-mmap-failed = アーカイブをメモリにマップできません
core-entry-out-of-bounds = エントリのデータがアーカイブの末尾を越えています
core-zip-structure = zip アーカイブの構造を解析できません
core-zip-too-small = zip としては小さすぎるファイルです：{ $size } バイト
core-zip-no-eocd = 中央ディレクトリ終端レコードがありません — zip アーカイブではないか、破損しています
core-zip64-missing = アーカイブは zip64 とされていますが、zip64 終端レコードがありません
core-zip-cd-out-of-bounds = 中央ディレクトリがファイルの末尾を越えています
core-zip-bad-local-header = オフセット { $offset } のローカルヘッダーが破損しています
core-zip-method = 未対応の圧縮方式 { $method }（stored と deflate に対応）
core-deflate-failed = deflate の展開エラー：{ $error }
core-aes-unknown-strength = エントリの AES 強度が不明です：{ $strength }
core-encrypted-too-short = 暗号化エントリが自身のヘッダー項目より短くなっています
core-aes-auth-failed = 認証コードが一致しません：エントリが破損しています
core-7z-structure = 7z アーカイブ { $path } の構造を解析できません；パスワード保護されている場合は -p でパスワードを指定してください
core-7z-structure-password = 7z アーカイブ { $path } の構造を解析できません：パスワードが違うか、アーカイブが破損している可能性があります
core-7z-block-failed = ブロック { $block } を展開できません
core-7z-block-error = ブロック { $block } の展開中にエラー：{ $error }
core-tar-structure = tar アーカイブの構造を解析できません
core-tar-bad-header = 最初の tar ヘッダーがチェックサム検査に失敗しました
core-gzip-not-tar = gzip ですが中身が tar ではありません — このようなアーカイブには対応していません
core-gzip-damaged = gzip の展開エラー（アーカイブが破損しているか途中で切れています）
core-targz-too-large = 展開した tar.gz が許可されたメモリに収まりません：現在 { $used } GB、上限 { $limit } GB。--cache-mb で上限を引き上げてください
core-rar-error = RAR アーカイブの読み込みエラー：{ $error }
core-grep-substring = 次の部分文字列検索を作成できません：{ $pattern }
core-grep-regex = 無効な正規表現：{ $pattern }
core-grep-block-errors = ブロックの走査中にエラー：{ $errors }
core-verify-short-read = { $offset } からの読み込みが { $want } バイトではなく { $got } バイトを返しました
core-verify-crc = CRC32 が一致しません：実際 { $actual }、期待値 { $expected }
core-verify-block = <ブロック { $block }>

## WinFsp によるマウント

mount-err-create = WinFsp ボリュームを作成できません：{ $error }
mount-err-mount = { $mountpoint } にマウントできません：{ $error }
mount-err-dispatcher = WinFsp ディスパッチャーを起動できません：{ $error }
mount-err-no-winfsp = WinFsp が見つかりません。次のコマンドでインストールしてください：  winget install WinFsp.WinFsp

## 一般的なエラー

error-prefix = エラー
err-create-dir = { $path } を作成できません
err-write = { $path } に書き込めません
err-read = { $path } を読み込めません
err-read-password = パスワードを読み取れません
err-read-password-stdin = 標準入力からパスワードを読み取れません
err-spawn-background = バックグラウンドプロセスを起動できません
err-run-failed = { $tool } を起動できません
err-tool-failed =
    { $tool } が失敗しました：
    { $output }

## コマンドラインのヘルプ

help-about = アーカイブを Windows のドライブとして：展開せずに閲覧・検索・取り出し
help-notice =
    ライセンス GPL-3.0-or-later、ソース：https://github.com/marlogg74mp/zipmount

    マウントには WinFsp - Windows File System Proxy, Copyright (C) Bill Zissimopoulos — https://github.com/winfsp/winfsp を使用しています
help-heading-usage = 使い方：
help-heading-commands = コマンド
help-heading-arguments = 引数
help-heading-options = オプション
help-flag-help = ヘルプを表示
help-flag-version = バージョンを表示
help-archive = アーカイブのパス
help-encoding = zip 内のファイル名のエンコーディング：auto、utf8、cp866、cp1251
help-password = アーカイブのパスワードを尋ねる（入力は非表示）
help-password-stdin = 標準入力からパスワードを読み取る（最初の行）
help-prefix = 出力するパスの接頭辞、例："Z:\"
help-mount = アーカイブをドライブとしてマウント
help-mount-mountpoint = ドライブ文字（Z:）または空の NTFS ディレクトリのパス；省略時は最初の空きドライブ文字
help-mount-detach = バックグラウンドでマウントしてすぐに戻る
help-mount-open = マウントしたドライブをエクスプローラーで開く
help-mount-label = ボリュームラベル；既定はアーカイブのファイル名
help-mount-cache-mb = 展開済みデータのキャッシュ上限、MB（7z の場合は solid ブロックのキャッシュ）
help-ls = アーカイブ内のディレクトリを一覧表示
help-ls-path = アーカイブ内のパス；既定はルート
help-ls-recursive = 再帰的にツリー全体を表示
help-find = 名前のパターンでファイルを検索
help-find-pattern = パターン、例："*.log"
help-grep = アーカイブ内のファイルの内容を検索
help-grep-pattern = 探す部分文字列（--regex 指定時は正規表現）
help-grep-regex = パターンを正規表現として扱う
help-grep-files-only = ファイルのパスのみ、行は表示しない
help-grep-count = 各ファイルの一致数のみ
help-grep-ignore-case = 大文字と小文字を区別しない
help-grep-after-context = 一致の後に表示する文脈の行数
help-grep-before-context = 一致の前に表示する文脈の行数
help-grep-context = 一致の前後に表示する文脈の行数
help-grep-max-count = 1 ファイルあたり最大 N 件の一致
help-grep-max-total = 出力全体で最大 N 行
help-grep-path = アーカイブのこの枝の中だけを検索
help-grep-glob = 名前のパターンで絞り込む、例："*.log"
help-grep-copy-to = 一致したファイルをこのディレクトリに取り出す
help-info = アーカイブの概要
help-verify = 全エントリの読み込みを CRC32 でエンドツーエンドに検証
help-verify-random = 順番ではなくランダムな順序で読む：後方シークを検査
help-unmount = ドライブをアンマウント
help-unmount-target = ドライブ文字（Z:）またはアーカイブのパス
help-mounts = マウント中のアーカイブを表示
help-search = アーカイブ内を対話的に検索（コンテキストメニューから起動）
help-shell-install = エクスプローラーのコンテキストメニューに項目を追加
help-shell-install-modern = 「その他のオプションを表示」を経ない Windows 11 のメインメニュー（管理者権限を 1 回だけ要求）
help-shell-uninstall = コンテキストメニューから項目を削除
help-language = プログラムの言語を表示または選択
help-language-code = 言語コード（en、ru、zh-CN、ja、ko、pt-BR、es、de）、または Windows に従う場合は auto
help-doctor = マウントに必要な環境が整っているか確認

## ls、find、grep

password-prompt = アーカイブのパスワード：
err-path-not-found = アーカイブ内にパスが見つかりません：{ $path }
find-summary = 見つかった件数：{ $count }
grep-copied = 取り出したファイル：{ $count } -> { $path }
grep-summary = 一致したファイル：{ $matched } | 走査：{ $scanned } | スキップ：{ $skipped } | 展開：{ $size }、{ $seconds } 秒（{ $speed } MB/s）
grep-truncated = 上限に達したため出力を打ち切りました（--max-total）
grep-regex-note = パターンは正規表現として解釈されました（--regex）

## info

info-archive = アーカイブ：
info-format = 形式：
info-size = アーカイブのサイズ：
info-files = ファイル：
info-dirs = ディレクトリ：
info-uncompressed = 展開後：
info-ratio = 圧縮率：
info-nodes = ツリーのノード：
info-in-memory = メモリ使用量：
info-in-memory-value = { $size }（tar.gz は開くときに全体を展開します）
info-solid = solid：
info-rar-solid-yes = はい（1 つのファイルを読むにはそれ以前のストリーム全体を通過する必要があります）
info-7z-solid-yes = はい（1 つのファイルを読むとそのブロック全体を展開します）
info-solid-no = いいえ（各ファイルは個別に展開されます）
info-headers = 目次：
info-headers-encrypted = 暗号化済み
info-blocks = solid ブロック：
info-encrypted = 暗号化済み：
info-encrypted-value = { $count } 件のエントリ
info-parse-time = 解析時間：

## verify

verify-failure = エラー  { $path }：{ $reason }
verify-summary = 検証したファイル：{ $ok } | エラー：{ $errors } | 読み込み { $size }、{ $seconds } 秒（{ $speed } MB/s）
verify-random = ランダムアクセス
verify-no-checksum = 比較するチェックサムなしで読み込み：{ $count } 件のエントリ
verify-no-checksum-tar =
    tar は内容のチェックサムを持たないため、比較対象がありません。
    確認したのは、すべてのエントリが最後まで読め、アーカイブの範囲内に収まることです。
verify-no-checksum-targz =
    tar は内容のチェックサムを持ちませんが、gzip ストリーム全体の CRC32 は
    展開時に確認済みです — アーカイブが破損していれば検出されたはずです。
verify-no-checksum-rar =
    これらは目次が暗号化されていない RAR5 の暗号化エントリです：この形式は
    パスワード推測に使われないよう、チェックサムを意図的に乱しています。
    比較対象はありませんが、データは正しく展開されています。
verify-no-checksum-zip =
    これらは WinZip AE-2 のエントリです：CRC 欄は意図的に空で、
    完全性は復号時に確認した HMAC が保証します。
err-verify-failed = 検証に失敗しました：{ $count } 件のエントリ

## mount、unmount、mounts

mount-already = マウント済み：{ $letter }
mount-done = マウントしました：{ $letter }
mount-done-stats = マウントしました：{ $letter }（ファイル { $files } 個、ディレクトリ { $dirs } 個、内容 { $size }）
mount-parse-time = アーカイブの解析に { $seconds } 秒かかりました。
mount-stop-hint = Ctrl+C または `zipmount unmount { $letter }` でアンマウントします。
mount-unmounting = アンマウントしています...
mount-finished = 完了。アーカイブは変更されていません。
err-no-free-letters = 空いているドライブ文字がありません
err-spawn-password = パスワードをバックグラウンドプロセスに渡せません
err-mount-failed-detached =
    { $path } をマウントできません。
    原因を確認するには --detach を付けずに実行してください。
err-letter-busy =
    ドライブ文字 { $letter } は別のドライブが使用中です。
    空き：{ $free }
letters-none = 空いているドライブ文字がありません
unmount-done = アンマウントしました：{ $letter }
err-unmount-not-responding = { $letter } のマウントプロセスが応答しません；記録を一覧から削除しました
err-unmount-timeout = { $letter } は時間内にアンマウントされませんでした：ファイルがまだ開いている可能性があります
err-not-mounted = { $target } はマウント済みとして記録されていません。一覧：zipmount mounts
mounts-none = 何もマウントされていません。

## コンテキストメニューからの検索

search-archive = アーカイブ：{ $path }
search-stats = ファイル { $files } 個、展開後 { $size }。形式：{ $format }。
search-intro = ファイルの内容を検索します。空行で終了します。
search-prompt = 検索する文字列：
search-summary = 一致したファイル：{ $scanned } 個中 { $matched } 個、{ $seconds } 秒
search-truncated = （出力を打ち切りました）
search-error = 検索エラー：{ $error }
press-enter = Enter キーを押すと閉じます...

## コンテキストメニューの項目 — レジストリにも書き込まれます

menu-mount = ドライブとしてマウント
menu-mount-as = ドライブ文字を指定してマウント
menu-search = アーカイブ内を検索…
menu-unmount = アンマウント（ZipMount）
menu-unmount-letter = { $letter }: をアンマウント（ZipMount）

## shell-install、shell-uninstall

shell-installed = 次の種類にコンテキストメニューの項目を追加しました：{ $extensions }
shell-installed-items =
    { menu-mount } — 空きドライブ文字でマウントし、エクスプローラーで開く
    { menu-mount-as } — 選択用のサブメニュー
    { menu-search } — 内容で検索
    { menu-unmount } — マウントしたドライブ自体のメニュー
shell-installed-where =
    Windows 11 では、これらの項目は「その他のオプションを表示」の下にあります
    — または Shift+右クリックで直接表示されます。
shell-modern-hint = Shift なしのメインメニュー：zipmount shell-install --modern
shell-remove-hint = 削除：zipmount shell-uninstall
shell-removed = コンテキストメニューから項目を削除しました。

## モダンメニュー

modern-building = パッケージをビルドしています…
modern-trust-intro =
    管理者権限が必要な手順が 1 つ残っています。

    Windows がメインメニューに入れるのは署名済みパッケージだけで、
    証明書を信頼するかどうかはコンピューター全体に関わる判断のため、
    権限の昇格が必要です。証明書は自己署名で、ここにあります：
modern-trust-uac = まもなくユーザーアカウント制御のウィンドウが表示されます。
modern-registering = パッケージをインストールしています…
modern-done =
    完了。メインのコンテキストメニューの項目：

      { menu-mount } — アーカイブ上
      { menu-mount-as } — 空きドライブ文字だけのサブメニュー
      { menu-search } — 内容で検索
      { menu-unmount } — ドライブ内の何もない場所
modern-installed-to = プログラムのインストール先：{ $path }
modern-rebuild-hint = 再ビルドしたら、もう一度実行してください：zipmount shell-install --modern
modern-files-left = ファイルは { $path } に残ります
modern-cert-left = 証明書は信頼されたままです。削除するには（管理者が必要）：
err-sdk-tool-missing =
    { $tool } が見つかりません。Windows SDK に含まれています — インストールしてください。
    例：winget install Microsoft.WindowsSDK.10.0.26100
err-exe-busy =
    { $path } を更新できません：ファイルが使用中です。
    ドライブをアンマウント（zipmount unmount …）してから再試行してください。
err-shell-dll-missing =
    プログラムの隣に zipmount_shell.dll がありません。
    ビルドしてください：cargo build --release
err-bin-dir =
    { $path } を作成できません。
    ディレクトリがすでにある場合、別のアカウントの残りかもしれません —
    その場合は削除するか名前を変更してください。
err-cert-not-trusted =
    証明書が信頼されませんでした。
    これがないと Windows はパッケージを受け入れません。レジストリのメニューは
    管理者なしで使えます：zipmount shell-install
err-handler-create = ハンドラーの COM クラスを作成できません
err-handler-title = ハンドラーがタイトルを返しませんでした

## language

language-current = 言語：{ $name }（{ $code }）— { $source }
language-source-environment = ZIPMOUNT_LANG で設定
language-source-saved = zipmount language で選択
language-source-windows = Windows の表示言語
language-source-default = 既定
language-available = 使用可能：
language-hint = 選択：zipmount language <コード>。再び Windows に従う：zipmount language auto
language-set = 言語：{ $name }（{ $code }）。
language-follows-windows = 言語は再び Windows に従います：{ $name }（{ $code }）。
language-menu-updated =
    { $count ->
        [0] 書き換えるメニュー項目はレジストリにありません；モダンメニューは自動で言語を反映します。
       *[other] レジストリのメニュー項目 { $count } 種類をこの言語で書き換えました；モダンメニューは自動で反映します。
    }
language-env-overrides = 注意：ZIPMOUNT_LANG={ $value } が設定されているため、このコンソールでは引き続きそれが言語を決めます。
err-language-unknown = 不明な言語「{ $code }」。使用可能：{ $available }；または Windows に従う auto
err-language-save = 言語の選択を保存できません

## doctor

doctor-winfsp = WinFsp ライブラリ：
doctor-winfsp-found = 見つかりました（{ $path }）
doctor-winfsp-missing = 見つかりません
doctor-rar = rar 対応：
doctor-rar-yes = あり（--features rar による個人用ビルド。配布は禁止）
doctor-rar-no = なし（公式ビルド：UnRAR は GPL と互換性がありません）
doctor-language = 言語：
doctor-modern = モダンメニュー：
doctor-modern-ok = 動作中（「{ $title }」）
doctor-modern-silent = パッケージはありますが、ハンドラーが応答しません — { $error }
doctor-modern-missing = 未インストール（zipmount shell-install --modern）
doctor-init = 初期化：
doctor-init-ok = 成功
doctor-init-failed = 失敗
doctor-ready =
    準備完了。アーカイブをマウントするには：
        zipmount mount <archive.zip> Z:
doctor-reason = 原因：{ $error }
doctor-no-winfsp-note =
    閲覧・検索・取り出し（ls、find、grep、verify）は WinFsp がなくても
    動作します — ドライバーが必要なのはドライブのマウントだけです。

## Linux と macOS：ドライブ文字もエクスプローラーもない環境

help-about-unix = アーカイブをフォルダーとして：展開せずに閲覧・検索・取り出し
help-notice-unix = ライセンス GPL-3.0-or-later、ソース：https://github.com/marlogg74mp/zipmount
help-prefix-unix = 出力するパスの接頭辞、例："/home/me/ZipMount/logs/"
help-mount-mountpoint-unix = マウント先の空のディレクトリ。省略時は ~/ZipMount/<アーカイブ名>
help-mount-open-unix = マウントしたフォルダーをファイルマネージャーで開く
help-unmount-unix = アーカイブをアンマウント
help-unmount-target-unix = マウント先のディレクトリまたはアーカイブのパス
help-language-code-unix = 言語コード（en、ru、zh-CN、ja、ko、pt-BR、es、de）、またはシステムに従う場合は auto
language-source-system = システムの言語
language-hint-unix = 選択：zipmount language <コード>。再びシステムに従う：zipmount language auto
language-follows-system = 言語は再びシステムに従います：{ $name }（{ $code }）。
err-language-unknown-unix = 不明な言語「{ $code }」。使用可能：{ $available }；またはシステムに従う auto
mount-err-fuse = { $mountpoint } にマウントできません：{ $error }
err-no-fuse = ここでは FUSE を使用できません：{ $missing } がありません。fuse3 パッケージをインストールしてください。例：  sudo apt install fuse3
err-mountpoint-not-dir = { $path } はディレクトリではありません
err-mountpoint-not-empty = { $path } は空ではありません。空のディレクトリにマウントしてください
err-unmount-failed = { $target } をアンマウントできません：{ $error }
err-unmount-in-use = { $target } をアンマウントできません：{ $programs } がファイルを開いています。閉じてから再試行してください
err-mount-unsupported = このシステムではまだマウントできません
doctor-fuse = FUSE：
doctor-fuse-ok = 使用可能（/dev/fuse、fusermount3）
mount-err-nfs = { $mountpoint } にマウントできません：{ $error }
doctor-nfs-ok = 使用可能（システム標準の NFS クライアント）
doctor-fuse-note =
    閲覧・検索・取り出し（ls、find、grep、verify）は FUSE がなくても
    動作します — 必要なのはマウントだけです。パッケージマネージャーで
    インストールできます：sudo apt install fuse3（Debian、Ubuntu）、
    sudo dnf install fuse3（Fedora）、sudo pacman -S fuse3（Arch）。
doctor-mount = マウント：
doctor-mount-unsupported-note =
    閲覧・検索・取り出し（ls、find、grep、verify）はこのシステムで
    動作しますが、マウントはまだ使用できません。
doctor-ready-unix =
    準備完了。アーカイブをマウントするには：
        zipmount mount <archive.zip>
