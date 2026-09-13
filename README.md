# oterm

Rust 製のオリジナルターミナルソフト。
ratatui + portable-pty + 自作 vt100 レンダラで、tmux ライクなタブ・ペイン分割、SSH プロファイル、AI コマンド補完を備える TUI 端末多重化ツール。

## 機能

- 複数タブ・任意分割のペイン (左右/上下、何段でも)
- TOML ベースのプロファイル (ローカルシェル + SSH)
- カラーテーマ (HEX / ANSI 名)
- AI コマンド補完 (クラウドの Anthropic Claude / ローカルの Ollama を切替可能)
- コマンド履歴の AI 検索 (履歴は外部に送らず、手元で照合)
- Windows (ConPTY) / macOS / Linux

## 動作要件

- Rust 1.75+ (2021 edition)

## 起動

```bash
cargo run --release
```

## キーバインド

| Key | Action |
|---|---|
| `Ctrl+Q` | 終了 |
| `Ctrl+Shift+T` | 新タブ |
| `Ctrl+Shift+W` | アクティブペインを閉じる |
| `Ctrl+Tab` / `Ctrl+Shift+Tab` | タブ切替 |
| `Ctrl+Shift+D` | 左右にペイン分割 |
| `Ctrl+Shift+E` | 上下にペイン分割 |
| `Alt+矢印` | 隣接ペインへフォーカス移動 |
| `Ctrl+Space` | AI コマンド補完モーダル |
| `Ctrl+Shift+R` | コマンド履歴の AI 検索モーダル |

AI モーダル内: `Enter` で送信 / 結果挿入、`Tab` でプロバイダ切替、`Shift+Tab` で Ollama モデル切替、`Esc` でキャンセル。

### 外側のターミナルとのキーの衝突

oterm はターミナルの中で動くため、oterm を起動している外側のターミナルアプリが同じキーを自分のショートカットとして使っていると、そのキーは oterm に届かない。

- **WezTerm** では `Ctrl+Shift+R` が WezTerm 側に取られるため、履歴検索モーダルが開かない。Windows 標準のコンソール (Windows PowerShell を直接起動した画面) では問題なく動作することを確認済み。
- `Ctrl+Shift+T` / `Ctrl+Shift+W` などの他のショートカットも、外側のターミナルの既定の設定によっては同じように衝突しうる。

WezTerm で使う場合は、`wezterm.lua` で該当キーの既定の割り当てを無効にすると oterm に届くようになる。

```lua
config.keys = {
  { key = 'R', mods = 'CTRL|SHIFT', action = wezterm.action.DisableDefaultAssignment },
}
```

キー名の書き方 (`'R'` / `'r'`) や修飾キーの表記は、`wezterm show-keys --lua` で表示される既定の割り当てに合わせること。既に `config.keys` を定義している場合は、その表に行を追加する。

## 設定ファイル

- Windows: `%APPDATA%\oterm\config\config.toml`
- macOS: `~/Library/Application Support/oterm/config.toml`
- Linux: `~/.config/oterm/config.toml`

ファイルが無い場合は同梱の `src/config/default.toml` が使われる。
API キーは同じディレクトリの `secrets.toml` に分離して置く (後述)。

設定ファイルの記入例は `examples/config.toml` と `examples/secrets.toml.example` にある。

### プロファイル例

```toml
default_profile = "powershell"

[[profiles]]
name = "powershell"
type = "local"
command = "powershell.exe"
args = ["-NoLogo"]

[[profiles]]
name = "myserver"
type = "ssh"
host = "example.com"
user = "alice"
port = 22
extra_args = ["-o", "ServerAliveInterval=30"]
```

`type = "ssh"` は OS の `ssh` バイナリを呼び出すため、認証 (鍵/エージェント/パスワード) は通常の SSH と同じ仕組みで動く。

### AI 補完

クラウド (Anthropic) とローカル LLM (Ollama) の両方に対応し、モーダル内で切り替えられる。

```toml
[ai]
enabled = true
provider = "anthropic"   # 起動時の既定プロバイダ ("anthropic" または "ollama")
model = "claude-haiku-4-5-20251001"   # Anthropic 側のモデル
max_tokens = 256

[ai.ollama]
base_url = "http://localhost:11434"
models = ["qwen2.5-coder", "llama3.2"]   # Shift+Tab で順に切替
```

`Ctrl+Space` でモーダルを開き、自然言語 (例: 「カレントディレクトリ以下で 1MB 以上のファイルを大きい順に列挙」) を入力 → Enter で送信。返ってきたコマンドを `Enter` で確定するとアクティブペインに挿入される (改行は付かないので、内容を確認してから自分で `Enter` を押して実行)。

モーダル上部に現在のプロバイダとモデルが表示される。`Tab` で anthropic ⇄ ollama、`Shift+Tab` で `[ai.ollama].models` を順に切り替える。切替結果はモーダルを閉じても保持される (恒久的に変えたい場合は `config.toml` の `provider` を編集する)。

Ollama を使う場合はローカルで `ollama serve` が動作し、指定モデルが `ollama pull` 済みである必要がある。未起動やモデル未取得の場合は自動的にクラウドへ切り替わることはなく、モーダルにエラーが表示される。

思考 (thinking) を行うモデル (例: gemma4) は、思考に出力の上限を使い切って応答が空になるのを避けるため、思考を無効にして呼び出す。

#### API キー (`secrets.toml`)

Anthropic のキーは `config.toml` と同じディレクトリの `secrets.toml` に置く。設定本体を共有・dotfile 管理してもキーが混ざらないようにするため。

```toml
# secrets.toml
anthropic_api_key = "sk-ant-..."
```

キーの探索順は `secrets.toml` → `ANTHROPIC_API_KEY` 環境変数 → `config.toml` の `[ai].api_key` (旧方式)。Ollama はキー不要。`secrets.toml` が壊れている場合は警告ログを出して「キー無し」として起動を続行する。

### コマンド履歴の AI 検索

`Ctrl+Shift+R` で履歴検索モーダルを開き、過去に実行したコマンドを自然言語 (例: 「先週 docker のログを見たコマンド」) で探せる。`↑↓` で選んで `Enter` を押すとアクティブペインに挿入される (AI 補完と同じく改行は付かないので、確認してから自分で実行する)。

仕組み:

- LLM に送るのは質問文だけ。LLM が返した検索語 (例: `docker`、`logs`) で oterm が手元の履歴を照合する。**履歴そのものは Ollama にも Anthropic にも送らない。**
- 質問文に含まれる英数字の語 (例: `deploy.ps1`) もそのまま照合に使う。
- 一致した検索語の数が多い順、同数なら新しい順に最大 20 件を表示する。同じコマンドは最新の 1 件にまとめる。
- `[ai].enabled = false` の場合は AI を使わず、質問文中の語だけで照合する。
- プロバイダの切替 (`Tab` / `Shift+Tab`) は AI 補完モーダルと共通。

対象の履歴 (アクティブなペインのプロファイルから判定):

- PowerShell: PSReadLine の履歴 (`%APPDATA%\Microsoft\Windows\PowerShell\PSReadLine\ConsoleHost_history.txt`、macOS / Linux は `~/.local/share/powershell/PSReadLine/ConsoleHost_history.txt`)
- bash: `$HISTFILE`、未設定なら `~/.bash_history`

制約:

- PowerShell の履歴には日時が記録されないため、「先週」のような時期の指定は効かない (新しい順に並ぶだけ)。
- bash は通常シェルの終了時に履歴を書き込むため、開いているセッションの直近のコマンドは出てこない。
- 複数行にまたがるコマンドは対象外 (挿入すると途中の行が実行されてしまうため)。
- SSH・cmd のペインや、ペインの中で後から起動したシェルの履歴は対象外。

### テーマ

```toml
[theme]
status_fg = "#000000"
status_bg = "#00d7ff"
focus_border = "#ffaf00"
inactive_border = "#444444"
tab_active_fg = "#000000"
tab_active_bg = "#00d7ff"
tab_inactive_fg = "#bbbbbb"
tab_inactive_bg = "#262626"
```

色は `#rrggbb` または ANSI 名 (`black`/`red`/`green`/`yellow`/`blue`/`magenta`/`cyan`/`white`/`gray`/`darkgray`)。

## ログ

`%TEMP%\oterm.log` (Windows) / `/tmp/oterm.log` (Unix) に書き出される。レベルは `RUST_LOG=debug` などで上書き可能。

## ライセンス

MIT OR Apache-2.0 のデュアル。詳細は `LICENSE-MIT` / `LICENSE-APACHE`。

## ロードマップ

- Phase 1: 単一シェルを画面内で動かす ✅
- Phase 2: タブ・ペイン分割・テーマ・プロファイル設定 ✅
- Phase 3: SSH プロファイル・AI 補完・OSS 公開 ✅
- Phase 4: ローカル LLM (Ollama) 対応・プロバイダのモーダル内切替・`secrets.toml` への API キー分離 ✅
- Phase 5: コマンド履歴の AI 検索 ✅ ← **現在ここ**

見送った項目 (導入コストに対して得られる効果が小さいため、実装しない):

- russh ベースのアプリ内 SSH — OS 標準の `ssh` で Windows / macOS / Linux とも既に動作しており、置き換える利点が小さい。加えて非同期ランタイム (tokio) の導入が必要になる。
- プラグイン API — 具体的な使い道がないまま拡張点を設計することになる。
