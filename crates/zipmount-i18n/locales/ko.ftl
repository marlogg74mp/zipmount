# ZipMount — 한국어. Machine-assisted translation; corrections are welcome.
#
# "WinFsp - Windows File System Proxy, Copyright (C) Bill Zissimopoulos"는 번역하지 마십시오:
# WinFsp 라이선스가 이 문구를 그대로 요구합니다.

## 숫자와 단위

decimal-separator = .
unit-b = B
unit-kb = KB
unit-mb = MB
unit-gb = GB
unit-tb = TB
seconds = { $value }초

## 아카이브 열기

core-open-failed = 아카이브 { $path }을(를) 열 수 없습니다
core-unknown-format = { $path }의 형식을 알 수 없습니다: zip, 7z, tar가 아닙니다
core-rar-not-built = { $path }은(는) RAR 아카이브이지만 이 빌드는 rar를 지원하지 않습니다: UnRAR 라이선스가 GPL과 호환되지 않으므로 rar는 직접 사용할 목적으로 소스에서 빌드할 때만 켤 수 있습니다(cargo build --release --features zipmount/rar)
core-unknown-encoding = 알 수 없는 인코딩 "{ $value }"; 사용 가능: auto, utf8, cp866, cp1251
core-deflate-corrupt = deflate 데이터가 손상되었습니다(zlib 코드 { $code })
core-password-missing = 아카이브가 암호화되어 있습니다: 암호가 필요합니다(-p 또는 --password-stdin)
core-password-wrong = 암호가 틀렸습니다

## 아카이브 읽기

core-mmap-failed = 아카이브를 메모리에 매핑할 수 없습니다
core-entry-out-of-bounds = 항목 데이터가 아카이브 끝을 넘어갑니다
core-zip-structure = zip 아카이브의 구조를 해석할 수 없습니다
core-zip-too-small = zip이라기에는 너무 작은 파일입니다: { $size }바이트
core-zip-no-eocd = 중앙 디렉터리 끝 레코드가 없습니다 — zip 아카이브가 아니거나 손상되었습니다
core-zip64-missing = 아카이브가 zip64로 표시되어 있지만 zip64 끝 레코드가 없습니다
core-zip-cd-out-of-bounds = 중앙 디렉터리가 파일 끝을 넘어갑니다
core-zip-bad-local-header = 오프셋 { $offset }의 로컬 헤더가 손상되었습니다
core-zip-method = 지원하지 않는 압축 방식 { $method }(stored와 deflate를 지원합니다)
core-deflate-failed = deflate 압축 해제 오류: { $error }
core-aes-unknown-strength = 항목의 AES 강도를 알 수 없습니다: { $strength }
core-encrypted-too-short = 암호화된 항목이 자신의 헤더 필드보다 짧습니다
core-aes-auth-failed = 인증 코드가 일치하지 않습니다: 항목이 손상되었습니다
core-7z-structure = 7z 아카이브 { $path }의 구조를 해석할 수 없습니다; 암호로 보호되어 있다면 -p로 암호를 입력하십시오
core-7z-structure-password = 7z 아카이브 { $path }의 구조를 해석할 수 없습니다: 암호가 틀렸거나 아카이브가 손상되었을 수 있습니다
core-7z-block-failed = 블록 { $block }을(를) 풀 수 없습니다
core-7z-block-error = 블록 { $block } 압축 해제 오류: { $error }
core-tar-structure = tar 아카이브의 구조를 해석할 수 없습니다
core-tar-bad-header = 첫 번째 tar 헤더가 체크섬 검사를 통과하지 못했습니다
core-gzip-not-tar = gzip이지만 안에 tar가 없습니다 — 이런 아카이브는 지원하지 않습니다
core-gzip-damaged = gzip 압축 해제 오류(아카이브가 손상되었거나 잘렸습니다)
core-targz-too-large = 풀어 놓은 tar.gz가 허용된 메모리에 들어가지 않습니다: 현재 { $used } GB, 상한 { $limit } GB. --cache-mb로 상한을 늘리십시오
core-rar-error = RAR 아카이브 읽기 오류: { $error }
core-grep-substring = 다음 부분 문자열 검색을 만들 수 없습니다: { $pattern }
core-grep-regex = 잘못된 정규식: { $pattern }
core-grep-block-errors = 블록을 탐색하는 중 오류: { $errors }
core-verify-short-read = { $offset }에서 읽기가 { $want }바이트 대신 { $got }바이트를 반환했습니다
core-verify-crc = CRC32 불일치: 실제 { $actual }, 예상 { $expected }
core-verify-block = <블록 { $block }>

## WinFsp를 통한 마운트

mount-err-create = WinFsp 볼륨을 만들 수 없습니다: { $error }
mount-err-mount = { $mountpoint }에 마운트할 수 없습니다: { $error }
mount-err-dispatcher = WinFsp 디스패처를 시작할 수 없습니다: { $error }
mount-err-no-winfsp = WinFsp를 찾을 수 없습니다. 다음 명령으로 설치하십시오:  winget install WinFsp.WinFsp

## 일반 오류

error-prefix = 오류
err-create-dir = { $path }을(를) 만들 수 없습니다
err-write = { $path }에 쓸 수 없습니다
err-read = { $path }을(를) 읽을 수 없습니다
err-read-password = 암호를 읽을 수 없습니다
err-read-password-stdin = 표준 입력에서 암호를 읽을 수 없습니다
err-spawn-background = 백그라운드 프로세스를 시작할 수 없습니다
err-run-failed = { $tool }을(를) 시작할 수 없습니다
err-tool-failed =
    { $tool } 실행 실패:
    { $output }

## 명령줄 도움말

help-about = 아카이브를 Windows 드라이브로: 압축을 풀지 않고 탐색, 검색, 추출
help-notice =
    라이선스 GPL-3.0-or-later, 소스: https://github.com/marlogg74mp/zipmount

    마운트는 WinFsp - Windows File System Proxy, Copyright (C) Bill Zissimopoulos — https://github.com/winfsp/winfsp 를 사용합니다
help-heading-usage = 사용법:
help-heading-commands = 명령
help-heading-arguments = 인수
help-heading-options = 옵션
help-flag-help = 도움말 표시
help-flag-version = 버전 표시
help-archive = 아카이브 경로
help-encoding = zip 안의 파일 이름 인코딩: auto, utf8, cp866, cp1251
help-password = 아카이브 암호를 묻기(입력 내용 숨김)
help-password-stdin = 표준 입력에서 암호 읽기(첫 줄)
help-prefix = 출력 경로의 접두사, 예: "Z:\"
help-mount = 아카이브를 드라이브로 마운트
help-mount-mountpoint = 드라이브 문자(Z:) 또는 빈 NTFS 디렉터리 경로; 생략하면 첫 번째 빈 드라이브 문자
help-mount-detach = 백그라운드에서 마운트하고 바로 반환
help-mount-open = 마운트한 드라이브를 파일 탐색기에서 열기
help-mount-label = 볼륨 레이블; 기본값은 아카이브 파일 이름
help-mount-cache-mb = 압축 해제한 데이터 캐시의 상한, MB(7z의 경우 solid 블록 캐시)
help-ls = 아카이브 안의 디렉터리 나열
help-ls-path = 아카이브 안의 경로; 기본값은 루트
help-ls-recursive = 재귀적으로 전체 트리
help-find = 이름 패턴으로 파일 찾기
help-find-pattern = 패턴, 예: "*.log"
help-grep = 아카이브 안 파일의 내용 검색
help-grep-pattern = 찾을 부분 문자열(--regex 사용 시 정규식)
help-grep-regex = 패턴을 정규식으로 취급
help-grep-files-only = 파일 경로만 표시하고 줄은 표시하지 않음
help-grep-count = 파일별 일치 횟수만 표시
help-grep-ignore-case = 대소문자 무시
help-grep-after-context = 일치 뒤에 보여 줄 문맥 줄 수
help-grep-before-context = 일치 앞에 보여 줄 문맥 줄 수
help-grep-context = 일치 앞뒤에 보여 줄 문맥 줄 수
help-grep-max-count = 파일당 최대 N개 일치
help-grep-max-total = 전체 출력 최대 N줄
help-grep-path = 아카이브의 이 분기 안에서만 검색
help-grep-glob = 이름 패턴으로 제한, 예: "*.log"
help-grep-copy-to = 일치한 파일을 이 디렉터리로 추출
help-info = 아카이브 요약
help-verify = 모든 항목의 읽기를 CRC32로 처음부터 끝까지 검증
help-verify-random = 순서대로가 아니라 섞인 순서로 읽기: 뒤로 탐색하는 경로를 검사
help-unmount = 드라이브 마운트 해제
help-unmount-target = 드라이브 문자(Z:) 또는 아카이브 경로
help-mounts = 마운트된 아카이브 표시
help-search = 아카이브에서 대화형 검색(상황에 맞는 메뉴에서 실행)
help-shell-install = 파일 탐색기의 상황에 맞는 메뉴에 항목 추가
help-shell-install-modern = "추가 옵션 표시" 없이 쓰는 Windows 11 기본 메뉴(관리자 권한을 한 번 요청)
help-shell-uninstall = 상황에 맞는 메뉴에서 항목 제거
help-language = 프로그램 언어 표시 또는 선택
help-language-code = 언어 코드(en, ru, zh-CN, ja, ko, pt-BR, es, de), 또는 Windows를 따르려면 auto
help-doctor = 마운트할 환경이 준비되었는지 확인

## ls, find, grep

password-prompt = 아카이브 암호:
err-path-not-found = 아카이브에서 경로를 찾을 수 없습니다: { $path }
find-summary = 찾은 항목: { $count }
grep-copied = 추출한 파일: { $count } -> { $path }
grep-summary = 일치한 파일: { $matched } | 검사: { $scanned } | 건너뜀: { $skipped } | 압축 해제: { $size }, { $seconds }초({ $speed } MB/s)
grep-truncated = 상한에 도달해 출력을 잘랐습니다(--max-total)
grep-regex-note = 패턴을 정규식으로 해석했습니다(--regex)

## info

info-archive = 아카이브:
info-format = 형식:
info-size = 아카이브 크기:
info-files = 파일:
info-dirs = 디렉터리:
info-uncompressed = 압축 해제 크기:
info-ratio = 압축률:
info-nodes = 트리 노드:
info-in-memory = 메모리 사용량:
info-in-memory-value = { $size }(tar.gz는 열 때 전체를 풉니다)
info-solid = solid:
info-rar-solid-yes = 예(파일 하나를 읽으려면 앞선 스트림 전체를 거쳐야 합니다)
info-7z-solid-yes = 예(파일 하나를 읽으면 그 블록 전체를 풉니다)
info-solid-no = 아니요(각 파일을 따로 풉니다)
info-headers = 목차:
info-headers-encrypted = 암호화됨
info-blocks = solid 블록:
info-encrypted = 암호화됨:
info-encrypted-value = 항목 { $count }개
info-parse-time = 해석 시간:

## verify

verify-failure = 오류  { $path }: { $reason }
verify-summary = 검증한 파일: { $ok } | 오류: { $errors } | 읽음 { $size }, { $seconds }초({ $speed } MB/s)
verify-random = 섞인 순서로 접근
verify-no-checksum = 비교할 체크섬 없이 읽음: 항목 { $count }개
verify-no-checksum-tar =
    tar는 내용의 체크섬을 저장하지 않으므로 비교할 대상이 없습니다.
    확인한 것: 모든 항목을 끝까지 읽을 수 있고 아카이브 범위를 벗어나지 않습니다.
verify-no-checksum-targz =
    tar는 내용의 체크섬을 저장하지 않지만, gzip 스트림 전체의 CRC32는
    압축 해제 중에 확인했습니다 — 아카이브가 손상되었다면 드러났을 것입니다.
verify-no-checksum-rar =
    목차가 암호화되지 않은 RAR5의 암호화 항목입니다: 이 형식은 암호를
    추측하는 데 쓰이지 않도록 체크섬을 일부러 흐트러뜨립니다.
    비교할 대상은 없지만 데이터는 올바르게 풀렸습니다.
verify-no-checksum-zip =
    WinZip AE-2 항목입니다: CRC 필드는 일부러 비어 있고,
    무결성은 복호화 중에 확인한 HMAC이 보장합니다.
err-verify-failed = 검증 실패: 항목 { $count }개

## mount, unmount, mounts

mount-already = 이미 마운트됨: { $letter }
mount-done = 마운트함: { $letter }
mount-done-stats = 마운트함: { $letter }(파일 { $files }개, 디렉터리 { $dirs }개, 내용 { $size })
mount-parse-time = 아카이브 해석에 { $seconds }초 걸렸습니다.
mount-stop-hint = 마운트를 해제하려면 Ctrl+C 또는 `zipmount unmount { $letter }`.
mount-unmounting = 마운트 해제 중...
mount-finished = 완료. 아카이브는 변경되지 않았습니다.
err-no-free-letters = 빈 드라이브 문자가 없습니다
err-spawn-password = 백그라운드 프로세스에 암호를 전달할 수 없습니다
err-mount-failed-detached =
    { $path }을(를) 마운트할 수 없습니다.
    원인을 보려면 --detach 없이 실행하십시오.
err-letter-busy =
    드라이브 문자 { $letter }은(는) 다른 드라이브가 사용 중입니다.
    비어 있음: { $free }
letters-none = 빈 드라이브 문자 없음
unmount-done = 마운트 해제함: { $letter }
err-unmount-not-responding = { $letter }의 마운트 프로세스가 응답하지 않습니다; 목록에서 기록을 제거했습니다
err-unmount-timeout = { $letter }이(가) 제시간에 마운트 해제되지 않았습니다: 파일이 아직 열려 있을 수 있습니다
err-not-mounted = { $target }은(는) 마운트된 목록에 없습니다. 목록: zipmount mounts
mounts-none = 마운트된 것이 없습니다.

## 상황에 맞는 메뉴에서 검색

search-archive = 아카이브: { $path }
search-stats = 파일 { $files }개, 압축 해제 시 { $size }. 형식: { $format }.
search-intro = 파일 내용을 검색합니다. 빈 줄을 입력하면 끝납니다.
search-prompt = 검색할 내용:
search-summary = 일치한 파일: 검사한 { $scanned }개 중 { $matched }개, { $seconds }초
search-truncated = (출력을 잘랐습니다)
search-error = 검색 오류: { $error }
press-enter = 닫으려면 Enter 키를 누르십시오...

## 상황에 맞는 메뉴 항목 — 레지스트리에도 기록됩니다

menu-mount = 드라이브로 마운트
menu-mount-as = 드라이브 문자로 마운트
menu-search = 아카이브에서 검색…
menu-unmount = 마운트 해제(ZipMount)
menu-unmount-letter = { $letter }: 마운트 해제(ZipMount)

## shell-install, shell-uninstall

shell-installed = 다음 형식에 상황에 맞는 메뉴 항목을 추가했습니다: { $extensions }
shell-installed-items =
    { menu-mount } — 빈 드라이브 문자로 마운트하고 파일 탐색기에서 열기
    { menu-mount-as } — 선택할 수 있는 하위 메뉴
    { menu-search } — 내용으로 검색
    { menu-unmount } — 마운트한 드라이브 자체의 메뉴
shell-installed-where =
    Windows 11에서 이 항목들은 "추가 옵션 표시" 아래에 있습니다
    — 또는 Shift+오른쪽 클릭으로 바로 볼 수 있습니다.
shell-modern-hint = Shift 없는 기본 메뉴: zipmount shell-install --modern
shell-remove-hint = 제거: zipmount shell-uninstall
shell-removed = 상황에 맞는 메뉴에서 항목을 제거했습니다.

## 최신 메뉴

modern-building = 패키지를 빌드하는 중…
modern-trust-intro =
    관리자 권한이 필요한 단계가 하나 남았습니다.

    Windows는 서명된 패키지만 기본 메뉴에 넣으며, 인증서를 신뢰할지는
    컴퓨터 전체에 관한 결정이므로 권한 상승이 필요합니다.
    인증서는 자체 서명이며 여기에 있습니다:
modern-trust-uac = 곧 사용자 계정 컨트롤 창이 나타납니다.
modern-registering = 패키지를 설치하는 중…
modern-done =
    완료. 기본 상황에 맞는 메뉴의 항목:

      { menu-mount } — 아카이브에서
      { menu-mount-as } — 빈 드라이브 문자만 있는 하위 메뉴
      { menu-search } — 내용으로 검색
      { menu-unmount } — 드라이브 안의 빈 곳에서
modern-installed-to = 프로그램 설치 위치: { $path }
modern-rebuild-hint = 다시 빌드한 뒤에는 한 번 더 실행하십시오: zipmount shell-install --modern
modern-files-left = 파일은 { $path }에 남아 있습니다
modern-cert-left = 인증서는 계속 신뢰됩니다. 제거하려면(관리자 필요):
err-sdk-tool-missing =
    { $tool }을(를) 찾을 수 없습니다. Windows SDK에 포함되어 있습니다 — 설치하십시오.
    예: winget install Microsoft.WindowsSDK.10.0.26100
err-exe-busy =
    { $path }을(를) 업데이트할 수 없습니다: 파일이 사용 중입니다.
    드라이브를 마운트 해제(zipmount unmount …)한 뒤 다시 시도하십시오.
err-shell-dll-missing =
    프로그램 옆에 zipmount_shell.dll이 없습니다.
    빌드하십시오: cargo build --release
err-bin-dir =
    { $path }을(를) 만들 수 없습니다.
    디렉터리가 이미 있다면 다른 계정이 남긴 것일 수 있습니다 —
    그렇다면 삭제하거나 이름을 바꾸십시오.
err-cert-not-trusted =
    인증서가 신뢰되지 않았습니다.
    인증서 없이는 Windows가 패키지를 받아들이지 않습니다. 레지스트리 메뉴는
    관리자 없이도 작동합니다: zipmount shell-install
err-handler-create = 처리기의 COM 클래스를 만들 수 없습니다
err-handler-title = 처리기가 제목을 반환하지 않았습니다

## language

language-current = 언어: { $name }({ $code }) — { $source }
language-source-environment = ZIPMOUNT_LANG으로 설정됨
language-source-saved = zipmount language로 선택됨
language-source-windows = Windows 표시 언어
language-source-default = 기본값
language-available = 사용 가능:
language-hint = 선택: zipmount language <코드>. 다시 Windows 따르기: zipmount language auto
language-set = 언어: { $name }({ $code }).
language-follows-windows = 언어가 다시 Windows를 따릅니다: { $name }({ $code }).
language-menu-updated =
    { $count ->
        [0] 레지스트리에 다시 쓸 메뉴 항목이 없습니다; 최신 메뉴는 언어를 자동으로 반영합니다.
       *[other] 레지스트리 메뉴 항목 { $count }종을 이 언어로 다시 썼습니다; 최신 메뉴는 자동으로 반영합니다.
    }
language-env-overrides = 참고: ZIPMOUNT_LANG={ $value }이(가) 설정되어 있어 이 콘솔에서는 여전히 그것이 언어를 결정합니다.
err-language-unknown = 알 수 없는 언어 "{ $code }". 사용 가능: { $available }; 또는 Windows를 따르는 auto
err-language-save = 언어 선택을 저장할 수 없습니다

## doctor

doctor-winfsp = WinFsp 라이브러리:
doctor-winfsp-found = 찾음({ $path })
doctor-winfsp-missing = 찾을 수 없음
doctor-rar = rar 지원:
doctor-rar-yes = 있음(--features rar로 만든 개인 빌드; 배포하면 안 됩니다)
doctor-rar-no = 없음(공식 빌드: UnRAR은 GPL과 호환되지 않습니다)
doctor-language = 언어:
doctor-modern = 최신 메뉴:
doctor-modern-ok = 작동함("{ $title }")
doctor-modern-silent = 패키지는 있지만 처리기가 응답하지 않습니다 — { $error }
doctor-modern-missing = 설치되지 않음(zipmount shell-install --modern)
doctor-init = 초기화:
doctor-init-ok = 성공
doctor-init-failed = 실패
doctor-ready =
    준비 완료. 아카이브 마운트:
        zipmount mount <archive.zip> Z:
doctor-reason = 원인: { $error }
doctor-no-winfsp-note =
    탐색, 검색, 추출(ls, find, grep, verify)은 WinFsp 없이도 작동합니다
    — 드라이버는 드라이브를 마운트할 때만 필요합니다.

## Linux와 macOS: 드라이브 문자도 파일 탐색기도 없는 환경

help-about-unix = 아카이브를 폴더로: 압축을 풀지 않고 탐색, 검색, 추출
help-notice-unix = 라이선스 GPL-3.0-or-later, 소스: https://github.com/marlogg74mp/zipmount
help-prefix-unix = 출력 경로의 접두사, 예: "/home/me/ZipMount/logs/"
help-mount-mountpoint-unix = 마운트할 빈 디렉터리. 생략하면 ~/ZipMount/<아카이브 이름>
help-mount-open-unix = 마운트한 폴더를 파일 관리자에서 열기
help-unmount-unix = 아카이브 마운트 해제
help-unmount-target-unix = 마운트 디렉터리 또는 아카이브 경로
help-language-code-unix = 언어 코드(en, ru, zh-CN, ja, ko, pt-BR, es, de), 또는 시스템을 따르려면 auto
language-source-system = 시스템 언어
language-hint-unix = 선택: zipmount language <코드>. 다시 시스템 따르기: zipmount language auto
language-follows-system = 언어가 다시 시스템을 따릅니다: { $name }({ $code }).
err-language-unknown-unix = 알 수 없는 언어 "{ $code }". 사용 가능: { $available }; 또는 시스템을 따르는 auto
mount-err-fuse = { $mountpoint }에 마운트할 수 없습니다: { $error }
err-no-fuse = 여기서는 FUSE를 사용할 수 없습니다: { $missing }이(가) 없습니다. fuse3 패키지를 설치하세요. 예:  sudo apt install fuse3
err-mountpoint-not-dir = { $path }은(는) 디렉터리가 아닙니다
err-mountpoint-not-empty = { $path }이(가) 비어 있지 않습니다. 빈 디렉터리에 마운트하세요
err-unmount-failed = { $target }을(를) 마운트 해제할 수 없습니다: { $error }
err-unmount-in-use = { $target }을(를) 마운트 해제할 수 없습니다: { $programs }에서 파일이 열려 있습니다. 닫은 후 다시 시도하세요
err-mount-unsupported = 이 시스템에서는 아직 마운트를 사용할 수 없습니다
doctor-fuse = FUSE:
doctor-fuse-ok = 사용 가능(/dev/fuse, fusermount3)
mount-err-nfs = { $mountpoint }에 마운트할 수 없습니다: { $error }
doctor-nfs-ok = 사용 가능(시스템 내장 NFS 클라이언트)
doctor-fuse-note =
    탐색, 검색, 추출(ls, find, grep, verify)은 FUSE 없이도 작동합니다
    — FUSE는 마운트할 때만 필요합니다. 패키지 관리자로 설치하세요:
    sudo apt install fuse3(Debian, Ubuntu), sudo dnf install fuse3(Fedora),
    sudo pacman -S fuse3(Arch).
doctor-mount = 마운트:
doctor-mount-unsupported-note =
    탐색, 검색, 추출(ls, find, grep, verify)은 이 시스템에서 작동하지만
    마운트는 아직 사용할 수 없습니다.
doctor-ready-unix =
    준비 완료. 아카이브 마운트:
        zipmount mount <archive.zip>
