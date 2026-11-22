<h1 align="center">
  <img src="apps/docs/app/latch-logo.png" width="360" alt="Latch">
</h1>
<p align="center"><strong>코딩 에이전트가 작업 중에 피드백을 받게 하세요.</strong></p>
<p align="center">Latch는 에이전트가 작업하는 동안 프로젝트의 린터, 테스트 및 기타 검사를 실행하고 실패 내용을 세션으로 전달합니다.</p>
<p align="center">
  <a href="#플러그인-설치">플러그인 설치</a> ·
  <a href="#첫-검사-실행">첫 검사</a> ·
  <a href="#작동-방식">작동 방식</a> ·
  <a href="#호환성">호환성</a> ·
  <a href="#독립형-cli-설치">독립형 CLI</a> ·
  <a href="https://barakchamo.github.io/latch/">문서</a>
</p>
<p align="center">
  <a href="README.md">English</a> ·
  <a href="README_zh.md">简体中文</a> ·
  <a href="README_ja.md">日本語</a> ·
  <a href="README_ko.md">한국어</a> ·
  <a href="README_es.md">Español</a>
</p>

사람이 코드를 작성할 때 IDE는 작업 중에 피드백을 제공합니다. 구문 강조로 변경 내용을 읽기 쉬워지고, 저장할 때 린트와 포매팅을 실행하면 문제를 일찍 발견할 수 있습니다. 타입 검사를 하면 변경이 다른 코드에 미치는 영향도 알 수 있습니다. Latch는 코딩 에이전트에 같은 피드백을 제공합니다.

- **검사를 자동으로 실행합니다.** 지원되는 편집 직후나 작업 트리가 변경된 턴이 끝날 때, 실행 환경의 훅이 검사를 시작합니다. 에이전트가 어떤 검사를 언제 실행할지 기억할 필요가 없습니다.
- **컨텍스트에는 문제 해결에 필요한 정보만 담습니다.** 검사가 통과하면 기본적으로 출력하지 않습니다. 실패하면 에이전트가 조치하는 데 필요한 진단을 전달합니다.

## 작동 방식

Latch를 사용하면 편집 직후 린트 오류를 확인하거나 턴 종료 시 테스트 실패를 확인할 수 있습니다. 로컬이나 CI에서 이미 사용 중인 명령을 프로젝트에 두면, 팀원들이 서로 다른 에이전트를 사용해도 같은 검사를 실행할 수 있습니다.

플러그인은 각 실행 환경의 기본 훅을 Latch의 두 공통 이벤트에 연결합니다. 사용할 수 있는 이벤트는 실행 환경마다 다릅니다. 검사 명령은 프로젝트에 두고 모든 연동에서 공유할 수 있습니다.

| 이벤트 | 검사 실행 시점 |
| --- | --- |
| `after_edit` | 지원되는 파일 편집 도구가 작업 트리를 변경한 직후입니다. 린트처럼 빠른 검사에 적합합니다. |
| `turn_end` | 작업 트리를 변경한 턴이 끝날 때입니다. 셸 명령으로 바뀐 파일도 감지하므로 테스트 실행에 적합합니다. |

- **실제 변경이 없으면 건너뜁니다.** Latch는 편집이나 턴 전후에 Git에서 볼 수 있는 파일을 비교합니다. 작업 트리에 이미 커밋되지 않은 변경이 있어도, 실제 내용이 바뀌지 않은 편집과 읽기 전용 턴은 기본적으로 검사를 건너뜁니다.
- **성공한 검사는 조용히 처리합니다.** 기본적으로 성공 시 출력하지 않아 에이전트 세션에서 실패를 쉽게 찾을 수 있습니다.
- **변경에 맞는 검사만 실행합니다.** 경로나 확장자로 필터링할 수 있습니다. Markdown만 바뀌었을 때 Markdown 린트는 실행하고 TypeScript 검사는 건너뛸 수 있습니다.
- **실패 보고서를 간결하게 유지합니다.** 간결한 출력에는 오류나 어설션처럼 유용한 줄이 남습니다. 에이전트는 검사를 다시 실행하지 않고도 저장된 로그에서 자세한 내용을 확인할 수 있습니다.

## 플러그인 설치

macOS나 glibc를 사용하는 Linux(ARM64 또는 x64)에서 에이전트에 Latch를 설치합니다. 처음 사용하면 플러그인이 플랫폼에 맞는 Latch 바이너리를 다운로드하고 검증합니다. 설치 후에는 Git 프로젝트에서 새 세션을 여세요.

### Codex

```sh
codex plugin marketplace add BarakChamo/latch
codex plugin add latch@latch-tools
```

Codex가 다른 소스에서 `latch-tools`를 이미 추가했다고 표시하면 `codex plugin marketplace remove latch-tools`를 실행한 뒤 위의 두 명령을 다시 실행하세요.

새 Codex 세션에서 메시지가 표시되면 **Review hooks**를 선택하거나 `/hooks`를 열고 Latch 훅을 승인하세요.

### Claude Code

```sh
claude plugin marketplace add BarakChamo/latch
claude plugin install latch@latch-tools
```

프로젝트에서 새 Claude Code 세션을 엽니다.

### GitHub Copilot CLI

```sh
copilot plugin marketplace add BarakChamo/latch
copilot plugin install latch@latch-tools
```

### Pi

```sh
pi install git:github.com/BarakChamo/latch
```

Pi 1.5.2 이상을 사용하세요.

### OpenCode

`opencode --version`으로 버전을 확인하세요. v2.3.2 이상에서는 다음 명령을 실행합니다.

```sh
opencode plugin add 'github:BarakChamo/latch'
```

OpenCode v1을 사용 중이라면 새로 설치하기 전에 v2로 업그레이드하세요. [OpenCode 상세 안내](https://barakchamo.github.io/latch/docs/plugins/#opencode)

### Grok Build

```sh
grok plugin install BarakChamo/latch --trust
```

Grok Build 1.0.92에서 검증했습니다.

### Kimi Code

Kimi Code에서 다음 명령을 실행합니다.

```text
/plugins install https://github.com/BarakChamo/latch/tree/main
```

**Trust and install**을 선택한 뒤 `/reload`를 실행하거나 새 세션을 시작하세요.

### OpenHands

Agent Canvas에서 **Customize → Plugins → Add plugin**으로 이동합니다. 소스에 `github:BarakChamo/latch`를 입력한 다음 **Install**을 클릭하세요. 활성화된 플러그인은 새 로컬 대화에서 로드됩니다.
[OpenHands SDK 및 Canvas 설치 안내](https://barakchamo.github.io/latch/docs/plugins/#openhands)

OpenHands CLI를 사용하는 경우 [독립형 Latch 바이너리를 설치](https://barakchamo.github.io/latch/docs/cli/#install-the-cli)하고 Git 프로젝트에서 `latch install openhands`를 실행하세요.

[플러그인 관리](https://barakchamo.github.io/latch/docs/plugins/)

## 첫 검사 실행

**Latch는 어떤 언어의 프로젝트 명령이든 실행할 수 있습니다.** `package.json`에 `hook:turnend`를 정의하거나, 패키징된 uv 프로젝트에서 `hook-turnend`를 콘솔 스크립트로 정의하면 `latch.json`을 생략할 수 있습니다. 다른 명령은 `latch.json`에 추가하세요.
[스크립트 이름과 자동 검색](docs/usage.md#package-script-discovery)

### 검사 설정

예를 들어 프로젝트에서 이미 `make lint`를 실행할 수 있다면, 그 명령을 실행하는 디렉터리에 다음과 같이 `latch.json`을 작성하세요.

```json
{
  "hooks": {
    "turn_end": {
      "commands": "make lint"
    }
  }
}
```

명령 하나는 문자열로, 여러 명령은 인수 배열을 중첩해 지정하세요. [명령 문법](docs/usage.md#command-syntax)

파일을 직접 만들거나 [독립형 CLI](#독립형-cli-설치)에서 `latch config init`을 실행하세요. 명시적으로 지정한 명령과 자동으로 찾은 스크립트는 함께 사용할 수 있습니다.

프로젝트에서 에이전트를 시작하고 검사에 실패할 만한 편집을 요청하세요. 사용하는 환경에서 다음 프롬프트에 보고서를 전달한다면, 턴이 끝난 뒤 같은 세션에서 후속 메시지를 보내세요. 에이전트에 `latch_status`를 호출하도록 요청해 Latch가 검사를 실행했는지, 건너뛰었는지 확인하세요. 확인 후에는 테스트용 편집을 되돌립니다. 통과한 검사는 출력하지 않습니다.
[빠른 시작](https://barakchamo.github.io/latch/docs/quickstart/#run-your-first-check) · [실행 가능한 예제](examples/)

### 필터와 실패 출력

아래 예제는 `src/` 아래의 `.ts` 또는 `.tsx` 파일이 변경되었을 때 턴 종료 시 기존 `npm test` 스크립트를 실행합니다. 같은 파일이 두 필터를 모두 만족해야 합니다.

```json
{
  "hooks": {
    "turn_end": {
      "paths": ["src/**"],
      "extensions": ["ts", "tsx"],
      "commands": [["npm", "test"]],
      "failure_output": "compact"
    }
  }
}
```

아래는 Vitest 테스트에 실패했을 때의 간결한 보고서 일부입니다.

```text
Latch checks failed.
latch: command 1 ("npm") failed: exit status: 1
stdout:
Tests  1 failed | 36 passed (37)
[output compacted from 3175 bytes]
stderr:
FAIL  tests/message.test.ts > greets the caller by name
AssertionError: expected 'Hi, Latch!' to be 'Hello, Latch!'
Saved log: …/r-kgMl3D6EnjseUc7Nal1Wgf8Z.log
```

저장된 로그를 열면 테스트를 다시 실행하지 않고 전체 출력을 확인할 수 있습니다. `tail`은 표준 출력과 표준 오류 각각의 마지막 줄을 보여 줍니다. `failure_output`을 지정하지 않으면 기본 발췌본을 표시합니다.

[실행 가능한 필터 예제](examples/filters/) · [실패 출력 모드](https://barakchamo.github.io/latch/docs/configuration/#failure-output-modes)

### 에이전트가 실패에 대응할 기회 주기

Latch v0.1.3 이상에서는 `latch.json` 최상위에 `"turn_end_feedback": "continue_once"`를 설정할 수 있습니다. 지원되는 에이전트에 턴 종료 시의 실패 보고서를 자동으로 전달해 한 번 대응할 기회를 줍니다. 사용자가 메시지를 다시 보낼 필요가 없으며 검사를 자동으로 다시 실행하지 않습니다. Grok Build에서 이 옵션을 사용하려면 Latch v0.1.4 이상이 필요합니다.

[지원되는 에이전트와 실패 보고서 전달 방식](docs/usage.md#act-on-a-turn-end-failure)

## 독립형 CLI 설치

[플랫폼에 맞는 Latch를 다운로드](https://barakchamo.github.io/latch/docs/cli/#install-the-cli)합니다. 압축을 풀고 `latch`를 `PATH`에 포함된 디렉터리에 둡니다. `latch --version`을 실행해 설치를 확인하세요.

## 호환성

별도로 활성화가 필요한 경우를 제외하면, 로컬 열은 에이전트가 기본적으로 언제 실패 보고서를 받는지 보여 줍니다.

| 실행 환경 | 편집 후 | 턴 종료 | 관리형 클라우드 |
| --- | --- | --- | --- |
| Codex | 같은 턴 | 다음 프롬프트 | Latch CLI 또는 CI |
| Claude Code | 같은 턴 | 다음 프롬프트 | 단일 저장소에서 검증 |
| GitHub Copilot CLI | 같은 턴 | 다음 프롬프트 | 저장소 훅 · 미검증 |
| OpenCode v1 및 v2 | 같은 턴 | 다음 프롬프트 | 미검증 |
| Pi | 같은 턴 | 다음 모델 요청 | 미검증 |
| OpenHands CLI 및 SDK | 다음 프롬프트 | 다음 프롬프트 | 작업 공간 훅 · 미검증 |
| OpenHands Agent Canvas | — | 옵션 활성화 시 검증 | 미검증 |
| Kimi Code CLI | — | 다음 프롬프트 | 미검증 |
| Grok Build 1.0.92 | 같은 턴 | 모델에 전달되지 않음 | 미검증 |

‘다음 프롬프트’는 같은 세션과 프로젝트에서 보내는 다음 메시지입니다. Kimi 2.7.2는 대화형 세션과 ACP 세션에서 `continue_once`를 지원합니다. `kimi -p`에서는 `report`를 사용하세요.

Latch 0.1.6 이상에서는 체크아웃한 저장소에서 `latch install cloud claude`, `latch install cloud copilot` 또는 `latch install cloud openhands`를 실행한 뒤 Latch가 업데이트한 파일을 커밋하세요. 기존 설정은 유지됩니다. Claude Code 클라우드에서는 편집 후 실패가 에이전트에 즉시 전달됩니다. 턴 종료 시의 실패는 기본적으로 다음 프롬프트에 전달되며, `continue_once`를 사용하면 자동으로 전달됩니다.

[훅과 피드백 상세 안내](https://barakchamo.github.io/latch/docs/hooks/) · [클라우드 설정](https://barakchamo.github.io/latch/docs/cloud/)

## 문서

- [훅의 수명 주기와 실패 보고서 표시 위치](docs/usage.md#hooks-and-feedback)
- [패키지 스크립트 자동 검색](https://barakchamo.github.io/latch/docs/project-scripts/) 및 [실행 가능한 예제](examples/)
- [설정, 필터 및 JSON 스키마](docs/usage.md#configuration)
- [CLI 명령, 플러그인 도구 및 저장된 로그](docs/usage.md#commands-and-saved-logs)
- [에이전트 연동 안내](plugins/), [클라우드 설정](docs/usage.md#hosted-sessions) 및 [문제 해결](docs/usage.md#troubleshooting)

## 라이선스

Latch는 [Apache License 2.0](LICENSE)을 따릅니다.
