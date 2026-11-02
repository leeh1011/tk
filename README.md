# tk

러스트로 만든 개인용 툴킷입니다. 메모, 할 일, 리마인더를 SQLite 파일 하나에 저장하고, 터미널(CLI)과 텔레그램 봇이 같은 코어를 공유합니다. 컴퓨터에서 `tk note add`로 넣은 메모를 폰에서 `/search`로 찾을 수 있고, 리마인더는 정해진 시각에 폰으로 알림이 옵니다.

## 주요 기능

- **메모**: 추가, 최근 목록, 키워드 검색, 삭제
- **할 일**: 추가, 목록, 완료 처리, 삭제
- **리마인더**: "30분 뒤", "내일 09:00" 형식으로 예약하고, 시각이 되면 텔레그램으로 알림
- **두 가지 입구**: 터미널(`tk`)과 텔레그램 봇이 같은 DB를 사용
- **재시작에도 안전**: 리마인더는 DB에 저장되므로 봇을 재시작해도 예약이 유지됩니다
- **안전한 DB 업그레이드**: 스키마 버전을 관리하는 마이그레이션으로, 기능이 추가돼도 기존 데이터가 보존됩니다

## 구조

```
tk/
  Cargo.toml          # 워크스페이스 설정
  crates/
    tk-core/          # 핵심 로직: Store, 마이그레이션, 메모/할 일/리마인더, Notifier/Job trait
    tk-cli/           # 터미널 입구 (바이너리 이름: tk)
    tk-bot/           # 텔레그램 입구 + 스케줄러
```

- **tk-core**: SQLite 접근과 비즈니스 로직 전부를 담당합니다. 에러는 `thiserror`로 종류를 구분해서 돌려줍니다.
- **tk-cli**: `clap`으로 인자를 받고 결과를 출력하는 일만 합니다. SQL은 알지 못합니다.
- **tk-bot**: `teloxide`로 명령을 받아 core 함수에 연결하고, 주기적으로 `ReminderJob`을 실행해 알림을 보냅니다.

### 설계 포인트

- **Store**: DB 연결을 비공개 필드로 감싸서, 바깥 코드가 SQL을 직접 다루지 못하게 합니다.
- **마이그레이션**: SQL 문자열 배열과 `PRAGMA user_version`으로 관리합니다. 각 항목은 트랜잭션 안에서 적용되어, 중간에 실패해도 반쯤 적용된 상태가 남지 않습니다.
- **Notifier / Job trait**: 메시지를 보내는 방법(`Notifier`)과 주기적으로 실행되는 작업(`Job`)을 분리해서, 새 알림 채널이나 작업을 기존 코드 수정 없이 추가할 수 있습니다.
- **시간 처리**: 저장은 항상 UTC, 표시할 때만 로컬 시간으로 변환합니다. 리마인더 로직은 현재 시각을 인자로 받아서 실제 시간을 기다리지 않고 테스트할 수 있습니다.
- **LIKE 검색 이스케이프**: 검색어의 `%`, `_`, `\`를 이스케이프해서 글자 그대로 검색합니다.

## 설치와 빌드

Rust 툴체인이 필요합니다.

```
git clone <저장소 주소>
cd tk
cargo build --release
```

CLI를 설치하려면:

```
cargo install --path crates/tk-cli
```

## CLI 사용법

```
# 메모
tk note add 우유 사기
tk note list
tk note list --limit 20
tk note search 우유
tk note rm 3

# 할 일
tk todo add 세금 신고
tk todo list
tk todo done 2
tk todo rm 2

# 리마인더
tk remind add 30분 뒤 약 먹기
tk remind add 내일 09:00 회의 준비
tk remind list
```

### DB 파일 위치

기본값은 OS 표준 데이터 디렉터리입니다. 환경변수 `TK_DB` 또는 `--db` 옵션으로 바꿀 수 있습니다.

```
TK_DB=/tmp/test.db tk note list
```

## 텔레그램 봇

### 명령어

| 명령어 | 동작 |
|---|---|
| `/memo 내용` | 메모 저장 |
| `/search 키워드` | 메모 검색 |
| `/todo 내용` | 할 일 추가 |
| `/done 번호` | 할 일 완료 |
| `/remind 시간 내용` | 리마인더 예약 |

허용된 `chat_id` 외의 메시지는 모두 무시합니다.

### 설정

BotFather에서 봇을 만들고 토큰을 발급받은 뒤, 환경변수로 전달합니다.

```
TELOXIDE_TOKEN=발급받은_토큰
TK_ALLOWED_CHAT_ID=내_chat_id
TK_DB=/data/tk.db
```

토큰은 코드나 저장소에 넣지 말고 환경변수 또는 `.env`(git 제외)로만 관리하세요.

### 실행

```
cargo run --release -p tk-bot
```

봇 프로세스가 30초~1분 간격으로 시각이 된 리마인더를 조회해 전송하고, 전송 후 `sent_at`을 기록합니다.

## 배포

Docker 이미지로 빌드해서 집 서버, 라즈베리파이, VPS 등에 올립니다.

```
docker build -t tk-bot .
docker run -d --restart always \
  -e TELOXIDE_TOKEN=... \
  -e TK_ALLOWED_CHAT_ID=... \
  -e TK_DB=/data/tk.db \
  -v tk-data:/data \
  tk-bot
```

- DB 파일은 볼륨(`/data`)에 두어 컨테이너를 교체해도 유지됩니다.
- `restart: always`로 상시 구동합니다.
- DB 파일을 날짜별로 복사하는 cron 백업을 권장합니다.

## 개발

```
cargo test --workspace
cargo clippy --workspace --all-targets
cargo fmt --all
```

- 단위 테스트는 `open_in_memory()`로 매번 독립된 DB를 사용합니다.
- CLI 통합 테스트는 `assert_cmd`와 `tempfile`로 임시 DB를 지정해 실제 바이너리를 실행합니다.

## 기술 스택

`rusqlite` · `chrono` · `thiserror` · `anyhow` · `clap` · `directories` · `tokio` · `teloxide` · `assert_cmd` · `tempfile`

## 추가할 만한 기능

구조상 기존 코드를 거의 건드리지 않고 붙일 수 있는 확장 후보입니다.

| 기능 | 붙이는 방법 |
|---|---|
| 아침 브리핑 (날씨, 오늘 할 일, 일정) | 새 `Job` 구현 |
| RSS 새 글 알림, 가격 추적 | 새 `Job` + 테이블 1개 |
| 서버/사이트 상태 감시 알림 | 새 `Job` |
| ntfy, Pushover 등 푸시 병행 | 새 `Notifier` 구현 |
| 아이폰 단축어 연동 (폰에서 버튼 한 번으로 메모 저장) | `tk-core`를 쓰는 작은 HTTP 입구 크레이트 추가 |
| 태그, 전문 검색(FTS5) | 마이그레이션 + `search_notes` 내부 교체 |
| 반복 리마인더 (매주 월요일 등) | `reminders`에 컬럼 추가 마이그레이션 |
| 자유로운 자연어 시간 입력 | 시간 파싱 함수 교체 |
| 할 일 마감일, 우선순위 | 마이그레이션으로 컬럼 추가 |
| 인라인 버튼 응답 ("완료", "1시간 뒤 다시") | 봇 핸들러 추가 |
| 쉘 자동완성 | `clap_complete` |
| 릴리스 바이너리 자동 빌드 | GitHub Actions, `cargo-dist` |
| TUI 화면 | `ratatui` 기반 새 입구 크레이트 |
