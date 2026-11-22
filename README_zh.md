<h1 align="center">
  <img src="apps/docs/app/latch-logo.png" width="360" alt="Latch">
</h1>
<p align="center"><strong>让你的编程智能体获得及时反馈。</strong></p>
<p align="center">智能体工作时，Latch 会运行项目的 lint、测试和其他检查，并将失败信息带回会话。</p>
<p align="center">
  <a href="#安装插件">安装插件</a> ·
  <a href="#运行第一次检查">第一次检查</a> ·
  <a href="#工作原理">工作原理</a> ·
  <a href="#兼容性">兼容性</a> ·
  <a href="#安装独立-cli">独立 CLI</a> ·
  <a href="https://barakchamo.github.io/latch/">文档</a>
</p>
<p align="center">
  <a href="README.md">English</a> ·
  <a href="README_zh.md">简体中文</a> ·
  <a href="README_ja.md">日本語</a> ·
  <a href="README_ko.md">한국어</a> ·
  <a href="README_es.md">Español</a>
</p>

我们写代码时，IDE 会不断提供反馈。语法高亮让改动更易阅读，保存时的 lint 和格式化能及早发现问题，类型检查则能发现改动是否影响了其他代码。Latch 为编程智能体提供同样的反馈。

- **自动运行检查。** 在受支持的编辑之后，或在工作树发生变化的一轮对话结束时，运行环境的钩子会启动检查。智能体无需记住何时要运行哪些命令。
- **让上下文聚焦于问题。** 检查通过时默认不输出。检查失败时，Latch 提供可用于定位和修复问题的诊断信息。

## 工作原理

用 Latch 在编辑后立即发现 lint 错误，或在一轮对话结束时看到测试失败。直接复用已有的本地或 CI 命令，将它们保存在项目中，团队成员就能在不同智能体中运行同样的检查。

插件将各运行环境的原生钩子映射为两个通用的 Latch 事件。可触发的事件因运行环境而异，检查命令保存在项目中，适用于所有集成。

| 事件 | 何时运行检查 |
| --- | --- |
| `after_edit` | 受支持的文件编辑工具更改工作树后。适合 lint 等快速检查。 |
| `turn_end` | 智能体结束一轮更改了工作树的对话时。也能检测 shell 命令造成的修改，适合运行测试。 |

- **没有实际改动就跳过。** Latch 比较编辑或一轮对话前后 Git 能看到的文件。即使工作树原本已有未提交的改动，无实际修改的编辑和只读对话也默认跳过检查。
- **通过时保持安静。** 成功默认不输出，失败信息在会话中更容易被发现。
- **只运行与改动相关的检查。** 按路径或扩展名筛选。例如，只改 Markdown 时可运行 Markdown lint，并跳过 TypeScript 检查。
- **让失败报告简短易读。** 精简输出保留错误和断言等关键行。智能体可从保存的日志查看详情，无需重新运行检查。

## 安装插件

在 macOS 或使用 glibc 的 Linux（ARM64 或 x64）上，为你的编程智能体安装 Latch。插件首次使用时会下载并验证适合当前平台的 Latch 二进制文件。安装后，在 Git 项目中打开新会话。

### Codex

```sh
codex plugin marketplace add BarakChamo/latch
codex plugin add latch@latch-tools
```

如果 Codex 提示已从其他来源添加 `latch-tools`，先运行 `codex plugin marketplace remove latch-tools`，再运行上面的两条命令。

在新的 Codex 会话中，收到提示时选择 **Review hooks**，或打开 `/hooks`，然后批准 Latch 钩子。

### Claude Code

```sh
claude plugin marketplace add BarakChamo/latch
claude plugin install latch@latch-tools
```

在项目中打开新的 Claude Code 会话。

### GitHub Copilot CLI

```sh
copilot plugin marketplace add BarakChamo/latch
copilot plugin install latch@latch-tools
```

### Pi

```sh
pi install git:github.com/BarakChamo/latch
```

请使用 Pi 1.5.2 或更高版本。

### OpenCode

运行 `opencode --version` 查看版本。对于 v2.3.2 或更高版本，运行以下命令。

```sh
opencode plugin add 'github:BarakChamo/latch'
```

如果正在使用 OpenCode v1，首次安装时请升级到 v2。 [OpenCode 详细说明](https://barakchamo.github.io/latch/docs/plugins/#opencode)

### Grok Build

```sh
grok plugin install BarakChamo/latch --trust
```

已在 Grok Build 1.0.92 中验证。

### Kimi Code

在 Kimi Code 中运行以下命令。

```text
/plugins install https://github.com/BarakChamo/latch/tree/main
```

选择 **Trust and install**，然后运行 `/reload` 或启动新会话。

### OpenHands

在 Agent Canvas 中依次选择 **Customize → Plugins → Add plugin**，输入 `github:BarakChamo/latch` 作为来源，再点击 **Install**。启用后，插件会在新的本地会话中加载。
[OpenHands SDK 和 Canvas 安装说明](https://barakchamo.github.io/latch/docs/plugins/#openhands)

如果使用 OpenHands CLI，请[安装 Latch 独立二进制文件](https://barakchamo.github.io/latch/docs/cli/#install-the-cli)，并在 Git 项目中运行 `latch install openhands`。

[管理插件](https://barakchamo.github.io/latch/docs/plugins/)

## 运行第一次检查

**无论项目使用什么语言，Latch 都能运行项目命令。** 如果在 `package.json` 中定义了 `hook:turnend`，或在已打包的 uv 项目中定义了名为 `hook-turnend` 的控制台脚本，则无需创建 `latch.json`。其他命令可添加到 `latch.json`。
[脚本命名及自动发现](docs/usage.md#package-script-discovery)

### 配置检查

例如，如果项目中的 `make lint` 已经可以运行，请在运行该命令的目录中创建如下 `latch.json`。

```json
{
  "hooks": {
    "turn_end": {
      "commands": "make lint"
    }
  }
}
```

单条命令用字符串，多条命令用嵌套的参数数组。 [命令语法](docs/usage.md#command-syntax)

可以手动创建文件，也可以用[独立 CLI](#安装独立-cli)运行 `latch config init`。显式配置的命令可以与自动发现的脚本一起使用。

在项目中启动智能体，让它做一项会使检查失败的修改。如果运行环境在下一条提示中传递报告，请等本轮结束后在同一会话中发送一条后续消息。让智能体调用 `latch_status`，查看 Latch 是运行了检查还是跳过了检查。测试后撤销改动。通过的检查不会输出。
[快速入门](https://barakchamo.github.io/latch/docs/quickstart/#run-your-first-check) · [可运行的示例](examples/)

### 过滤条件与失败输出

以下示例在 `src/` 下的 `.ts` 或 `.tsx` 文件发生修改时，于一轮对话结束后运行现有的 `npm test` 脚本。同一个文件必须同时满足这两个过滤条件。

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

以下是 Vitest 运行失败时输出的精简报告片段。

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

打开保存的日志，即可查看完整输出，无需重新运行测试。`tail` 会显示标准输出和标准错误各自末尾的几行。不设置 `failure_output` 时，会使用默认的输出摘录。

[可运行的过滤示例](examples/filters/) · [失败输出模式](https://barakchamo.github.io/latch/docs/configuration/#failure-output-modes)

### 让智能体有机会处理失败

使用 Latch v0.1.3 或更高版本时，可在 `latch.json` 的顶层设置 `"turn_end_feedback": "continue_once"`。在受支持的运行环境中，Latch 会自动传递轮末检查的失败报告，给智能体一次响应机会。你无需再发送消息，Latch 也不会自动重跑检查。Grok Build 使用此选项需要 Latch v0.1.4 或更高版本。

[支持的智能体及其接收失败信息的方式](docs/usage.md#act-on-a-turn-end-failure)

## 安装独立 CLI

[下载适合当前平台的 Latch](https://barakchamo.github.io/latch/docs/cli/#install-the-cli)。解压后将 `latch` 放到 `PATH` 中的目录，再运行 `latch --version` 确认安装成功。

## 兼容性

除特别标明需启用的情况外，本地列展示智能体默认何时收到失败信息。

| 运行环境 | 编辑后 | 一轮对话结束时 | 托管云端 |
| --- | --- | --- | --- |
| Codex | 同一轮 | 下一条提示 | Latch CLI 或 CI |
| Claude Code | 同一轮 | 下一条提示 | 单仓库会话已验证 |
| GitHub Copilot CLI | 同一轮 | 下一条提示 | 仓库钩子，尚未验证 |
| OpenCode v1 和 v2 | 同一轮 | 下一条提示 | 尚未验证 |
| Pi | 同一轮 | 下一次模型请求 | 尚未验证 |
| OpenHands CLI 和 SDK | 下一条提示 | 下一条提示 | 工作区钩子，尚未验证 |
| OpenHands Agent Canvas | — | 手动启用后已验证 | 尚未验证 |
| Kimi Code CLI | — | 下一条提示 | 尚未验证 |
| Grok Build 1.0.92 | 同一轮 | 模型收不到反馈 | 尚未验证 |

「下一条提示」指你在同一会话、同一项目中发送的下一条消息。Kimi 2.7.2 在交互式会话和 ACP 会话中支持 `continue_once`。使用 `kimi -p` 时，请选择 `report`。

使用 Latch 0.1.6 或更高版本时，在本地检出的仓库中运行 `latch install cloud claude`、`latch install cloud copilot` 或 `latch install cloud openhands`，然后提交 Latch 更新的文件。现有设置会保留。在 Claude Code 云端，编辑后的检查失败会立即传给智能体。轮末的检查失败默认在下一条提示中传递，启用 `continue_once` 后则自动传递。

[钩子与反馈说明](https://barakchamo.github.io/latch/docs/hooks/) · [云端设置](https://barakchamo.github.io/latch/docs/cloud/)

## 文档

- [钩子生命周期与失败信息的显示位置](docs/usage.md#hooks-and-feedback)
- [脚本自动发现](https://barakchamo.github.io/latch/docs/project-scripts/)及[可运行的示例](examples/)
- [配置、过滤条件和 JSON Schema](docs/usage.md#configuration)
- [CLI 命令、插件工具与保存的日志](docs/usage.md#commands-and-saved-logs)
- [智能体集成指南](plugins/)、[云端设置](docs/usage.md#hosted-sessions)和[故障排查](docs/usage.md#troubleshooting)

## 许可证

Latch 采用 [Apache License 2.0](LICENSE)。
