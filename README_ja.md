<h1 align="center">
  <img src="apps/docs/app/latch-logo.png" width="360" alt="Latch">
</h1>
<p align="center"><strong>コーディングエージェントに、作業中のフィードバックを。</strong></p>
<p align="center">エージェントの作業に合わせて、Latch がプロジェクトのリンターやテストなどを実行し、失敗の内容をセッションに返します。</p>
<p align="center">
  <a href="#プラグインをインストールする">プラグインのインストール</a> ·
  <a href="#最初のチェックを実行する">最初のチェック</a> ·
  <a href="#仕組み">仕組み</a> ·
  <a href="#互換性">互換性</a> ·
  <a href="#スタンドアロン-cli-をインストールする">スタンドアロン CLI</a> ·
  <a href="https://barakchamo.github.io/latch/">ドキュメント</a>
</p>
<p align="center">
  <a href="README.md">English</a> ·
  <a href="README_zh.md">简体中文</a> ·
  <a href="README_ja.md">日本語</a> ·
  <a href="README_ko.md">한국어</a> ·
  <a href="README_es.md">Español</a>
</p>

人がコードを書くとき、IDE は作業中にフィードバックを返してくれます。シンタックスハイライトで変更を読みやすくし、保存時のリントやフォーマットで問題を早めに見つけ、型チェックで他のコードへの影響を確認できます。Latch は、コーディングエージェントにも同じ仕組みを提供します。

- **チェックを自動で実行。** 対応する編集の直後や、作業ツリーに変更があったターンの終了時に、実行環境のフックがチェックを起動します。どのチェックをいつ実行するかをエージェントが覚えておく必要はありません。
- **コンテキストを問題に集中させる。** 成功したチェックの出力は、既定では表示されません。失敗時には、エージェントが対処に使える診断情報を返します。

## 仕組み

編集直後にリントエラーを見つけたり、ターンの終了時にテストの失敗を確認したりできます。ローカルや CI ですでに使っているコマンドをプロジェクトに置けば、チームで異なるエージェントを使っていても同じチェックを実行できます。

プラグインは、各実行環境の標準フックを Latch 共通の二つのイベントに対応付けます。利用可能なイベントは環境ごとに異なります。チェックのコマンドはプロジェクト内に置き、どの連携でも共有できます。

| イベント | チェックが実行されるタイミング |
| --- | --- |
| `after_edit` | 対応するファイル編集ツールが作業ツリーを変更した直後。リントなどの短時間で終わるチェックに向いています。 |
| `turn_end` | 作業ツリーに変更を加えたターンが終了したとき。シェル経由の編集も対象になり、テストの実行に向いています。 |

- **実際に変更があった場合だけ実行。** Latch は編集やターンの前後で、Git から見えるファイルを比較します。作業ツリーに以前から未コミットの変更があっても、内容が変わらない編集や読み取り専用のターンでは、既定でチェックを省略します。
- **成功時は出力を省略。** 既定で成功時の出力を抑えるため、セッション内で失敗を見つけやすくなります。
- **変更に合ったチェックを実行。** パスや拡張子で絞り込めます。たとえば、Markdown だけを変更した場合は Markdown のリントを実行し、TypeScript のチェックを省略できます。
- **失敗の報告を簡潔に。** コンパクト出力はエラーやアサーションなどの重要な行を残します。エージェントはチェックを再実行せずに、保存済みログから詳細を取得できます。

## プラグインをインストールする

macOS または glibc を使う Linux（ARM64、x64）で、エージェントに Latch をインストールします。初回使用時に、プラグインが環境に合った Latch バイナリをダウンロードして検証します。インストール後は Git プロジェクトで新しいセッションを開いてください。

### Codex

```sh
codex plugin marketplace add BarakChamo/latch
codex plugin add latch@latch-tools
```

別のソースから `latch-tools` が追加済みだと表示されたら、`codex plugin marketplace remove latch-tools` を実行してから、上の二つのコマンドを再実行します。

新しい Codex セッションで、表示されたら **Review hooks** を選ぶか、`/hooks` を開いて Latch のフックを承認します。

### Claude Code

```sh
claude plugin marketplace add BarakChamo/latch
claude plugin install latch@latch-tools
```

プロジェクトで新しい Claude Code セッションを開きます。

### GitHub Copilot CLI

```sh
copilot plugin marketplace add BarakChamo/latch
copilot plugin install latch@latch-tools
```

### Pi

```sh
pi install git:github.com/BarakChamo/latch
```

Pi 1.5.2 以降を使用してください。

### OpenCode

`opencode --version` でバージョンを確認します。v2.3.2 以降では、次のコマンドを実行します。

```sh
opencode plugin add 'github:BarakChamo/latch'
```

OpenCode v1 をお使いの場合、新規インストールには v2 にアップグレードしてください。 [OpenCode の詳細](https://barakchamo.github.io/latch/docs/plugins/#opencode)

### Grok Build

```sh
grok plugin install BarakChamo/latch --trust
```

Grok Build 1.0.92 で動作を確認しています。

### Kimi Code

Kimi Code で次のコマンドを実行します。

```text
/plugins install https://github.com/BarakChamo/latch/tree/main
```

**Trust and install** を選び、`/reload` を実行するか、新しいセッションを開始します。

### OpenHands

Agent Canvas で **Customize → Plugins → Add plugin** を開き、ソースに `github:BarakChamo/latch` を入力して **Install** をクリックします。有効になったプラグインは、新しいローカル会話で読み込まれます。
[OpenHands SDK と Canvas のセットアップ](https://barakchamo.github.io/latch/docs/plugins/#openhands)

OpenHands CLI を使う場合は、[スタンドアロンの Latch バイナリをインストール](https://barakchamo.github.io/latch/docs/cli/#install-the-cli)し、Git プロジェクトで `latch install openhands` を実行します。

[プラグインの管理](https://barakchamo.github.io/latch/docs/plugins/)

## 最初のチェックを実行する

**Latch は、どの言語のプロジェクトでも既存のコマンドを実行できます。** `package.json` に `hook:turnend` を定義するか、パッケージ化された uv プロジェクトで `hook-turnend` をコンソールスクリプトとして定義すれば、`latch.json` は省略できます。他のコマンドは `latch.json` に追加します。
[スクリプト名と自動検出](docs/usage.md#package-script-discovery)

### チェックを設定する

たとえば、プロジェクトで `make lint` がすでに使えるなら、そのコマンドを実行するディレクトリに次の `latch.json` を置きます。

```json
{
  "hooks": {
    "turn_end": {
      "commands": "make lint"
    }
  }
}
```

単一のコマンドには文字列を使い、複数のコマンドには引数配列を入れ子にして指定します。 [コマンドの書式](docs/usage.md#command-syntax)

ファイルは手動で作成できます。[スタンドアロン CLI](#スタンドアロン-cli-をインストールする)で `latch config init` を実行して作成することもできます。明示的に指定したコマンドと自動検出したスクリプトは併用できます。

プロジェクトでエージェントを起動し、チェックが失敗する編集を依頼します。次のプロンプトで報告が届く実行環境では、ターンの終了を待ち、同じセッションで追加のメッセージを送ります。エージェントに `latch_status` を呼び出してもらうと、チェックが実行されたか、省略されたかを確認できます。確認後はテスト用の編集を元に戻してください。チェックの成功時には出力がありません。
[クイックスタート](https://barakchamo.github.io/latch/docs/quickstart/#run-your-first-check) · [実行できるサンプル](examples/)

### フィルターと失敗時の出力

次の例は、`src/` 配下の `.ts` または `.tsx` ファイルが変更されたとき、ターン終了時に既存の `npm test` スクリプトを実行します。同じファイルが両方の条件に一致する必要があります。

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

Vitest が失敗した場合のコンパクトな報告の抜粋です。

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

保存済みログを開けば、テストを再実行せずに全出力を確認できます。`tail` を指定すると、標準出力と標準エラー出力のそれぞれで末尾の行を表示します。`failure_output` を省略した場合は、既定の抜粋を表示します。

[実行できるフィルターの例](examples/filters/) · [失敗時の出力モード](https://barakchamo.github.io/latch/docs/configuration/#failure-output-modes)

### 失敗に対処する機会をエージェントに与える

Latch v0.1.3 以降では、`latch.json` のトップレベルに `"turn_end_feedback": "continue_once"` を設定できます。対応するエージェントにターン終了時の失敗報告が自動で届き、一度だけ応答する機会が与えられます。追加メッセージは不要で、Latch による自動再チェックもありません。Grok Build でこのオプションを使うには、Latch v0.1.4 以降が必要です。

[対応するエージェントと失敗報告の受け取り方](docs/usage.md#act-on-a-turn-end-failure)

## スタンドアロン CLI をインストールする

[環境に合った Latch をダウンロード](https://barakchamo.github.io/latch/docs/cli/#install-the-cli)します。アーカイブを展開し、`latch` を `PATH` が通っているディレクトリに置きます。`latch --version` を実行してインストールを確認してください。

## 互換性

ローカルの列は、エージェントに失敗報告が届く既定のタイミングを示します。別途有効化が必要な場合は、その旨を記載しています。

| 実行環境 | 編集後 | ターン終了時 | マネージドクラウド |
| --- | --- | --- | --- |
| Codex | 同じターン | 次のプロンプト | Latch CLI または CI |
| Claude Code | 同じターン | 次のプロンプト | 単一リポジトリで確認済み |
| GitHub Copilot CLI | 同じターン | 次のプロンプト | リポジトリのフック · 未検証 |
| OpenCode v1 と v2 | 同じターン | 次のプロンプト | 未検証 |
| Pi | 同じターン | 次のモデルリクエスト | 未検証 |
| OpenHands CLI と SDK | 次のプロンプト | 次のプロンプト | ワークスペースのフック · 未検証 |
| OpenHands Agent Canvas | — | オプションの有効化時に確認済み | 未検証 |
| Kimi Code CLI | — | 次のプロンプト | 未検証 |
| Grok Build 1.0.92 | 同じターン | モデルへの通知なし | 未検証 |

「次のプロンプト」は、同じプロジェクトの同じセッションに送る次のメッセージです。Kimi 2.7.2 は対話セッションと ACP セッションで `continue_once` に対応しています。`kimi -p` では `report` を使用します。

Latch 0.1.6 以降を使い、チェックアウトしたリポジトリで `latch install cloud claude`、`latch install cloud copilot`、または `latch install cloud openhands` を実行して、Latch が更新したファイルをコミットします。既存の設定は保持されます。Claude Code クラウドでは、編集後の失敗は即時にエージェントへ届きます。ターン終了時の失敗は既定で次のプロンプトに届き、`continue_once` では自動で届きます。

[フックとフィードバックの詳細](https://barakchamo.github.io/latch/docs/hooks/) · [クラウドのセットアップ](https://barakchamo.github.io/latch/docs/cloud/)

## ドキュメント

- [フックのライフサイクルと失敗報告の表示場所](docs/usage.md#hooks-and-feedback)
- [パッケージスクリプトの自動検出](https://barakchamo.github.io/latch/docs/project-scripts/)と[実行できるサンプル](examples/)
- [設定、フィルター、JSON Schema](docs/usage.md#configuration)
- [CLI コマンド、プラグインツール、保存済みログ](docs/usage.md#commands-and-saved-logs)
- [エージェントとの連携](plugins/)、[クラウドのセットアップ](docs/usage.md#hosted-sessions)、[トラブルシューティング](docs/usage.md#troubleshooting)

## ライセンス

Latch は [Apache License 2.0](LICENSE) の下で公開されています。
