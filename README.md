# oterm

Rust 製のオリジナルターミナルソフト。
ratatui + portable-pty + 自作 vt100 レンダラで、tmux ライクなタブ・ペイン分割、SSH プロファイル、AI コマンド補完を備える TUI 端末多重化ツール。

## 機能

- 複数タブ・任意分割のペイン (左右/上下、何段でも)
- TOML ベースのプロファイル (ローカルシェル + SSH)
- カラーテーマ (HEX / ANSI 名)
- AI コマンド補完 (Anthropic Claude Haiku 4.5)
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

AI モーダル内: `Enter` で送信 / 結果挿入、`Esc` でキャンセル。

## 設定ファイル

- Windows: `%APPDATA%\oterm\config.toml`
- macOS: `~/Library/Application Support/oterm/config.toml`
- Linux: `~/.config/oterm/config.toml`

ファイルが無い場合は同梱の `src/config/default.toml` が使われる。

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

```toml
[ai]
enabled = true
model = "claude-haiku-4-5-20251001"
max_tokens = 256
# api_key = "sk-ant-..."     # または ANTHROPIC_API_KEY 環境変数
```

`Ctrl+Space` でモーダルを開き、自然言語 (例: 「カレントディレクトリ以下で 1MB 以上のファイルを大きい順に列挙」) を入力 → Enter で送信。返ってきたコマンドを `Enter` で確定するとアクティブペインに挿入される (改行は付かないので、内容を確認してから自分で `Enter` を押して実行)。

API キー未設定や `enabled = false` の場合はモーダルにエラーが出る。

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
- Phase 3: SSH プロファイル・AI 補完・OSS 公開 ✅ ← **現在ここ**
- 今後: russh ベースのアプリ内 SSH、コマンド履歴の AI ベース検索、プラグイン API
