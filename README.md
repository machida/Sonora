# Sonora

YouTube の URL から音声をダウンロードする macOS 向けデスクトップアプリ。
[Tauri 2](https://tauri.app/) 製。既定では **m4a（変換なし・高音質）** で保存するため
`yt-dlp` だけで動作し、**ffmpeg は同梱していません**。mp3 が必要なときだけ、
オプションを ON にした時点で ffmpeg を各自の環境へ自動ダウンロードします。

> [!IMPORTANT]
> ### ⚠️ 免責事項・利用上の注意
>
> このソフトウェアは、**自分が権利を持つコンテンツ**や、**ダウンロードが許諾されている／
> 著作権の制限を受けないコンテンツ**（自作物・パブリックドメイン・クリエイティブ・コモンズ等）を
> 個人的に扱うことを目的とした学習・実験用のツールです。
>
> - 各動画サービスの**利用規約**（YouTube では原則、事前の許可なくダウンロードを禁止）を確認し、遵守してください。
> - **著作権者の許諾なく、著作物をダウンロード・複製・再配布する行為は法律で禁止されています。**
>   日本では、違法にアップロードされた著作物と知りながらダウンロードする行為は、私的使用目的であっても違法となる場合があります。
> - 本ツールの利用によって生じたいかなる損害・法的責任についても、**作者は一切の責任を負いません**（MIT ライセンスの無保証条項に基づく）。
> - 利用者は、自身の責任において、適用される法令および各サービスの規約の範囲内で本ツールを使用するものとします。
>
> *This is a personal/educational utility intended only for content you own or are
> otherwise permitted to download. Respect the terms of service of each platform and
> all applicable copyright laws. The author assumes no liability for any use of this software.*

## 機能

- YouTube の URL を入力して音声をダウンロード
  - **既定は m4a**（YouTube 配信の音声をそのまま保存。変換なし＝高速・高音質・ffmpeg 不要）
  - **「mp3 に変換する」トグル**を ON にすると mp3（最高音質）で保存。ON にした瞬間に
    ffmpeg（約 45MB）の取得確認が出る。取得後は次回以降そのまま使える
- 保存先フォルダを選択（既定はダウンロードフォルダ）
- ダウンロード進捗バーとログ表示
- **プレイリスト／ラジオの曲を個別に選択してダウンロード**
  - URL に `&list=...` や `&start_radio=1` が含まれる場合、URL を入力すると
    自動で曲一覧を取得して表示（`yt-dlp --flat-playlist`。実DLはしないので高速）
  - 各曲は**既定でチェック済み**。ダウンロードしたくない曲のチェックを外す
  - 「全選択 / 全解除」リンクで一括切り替え
  - ダウンロードはチェックした曲だけ取得（`--yes-playlist --playlist-items 1,3,5…`）
  - `list=` を含まない単体 URL は、そのまま 1 本だけ取得（`--no-playlist`）

## 入手・インストール

このアプリは**公証していない**ため、基本は **自分でビルドして使う**形です（→ [ビルド](#ビルド)）。
できあがった `Sonora.app` を **`/Applications` にコピー**すれば、以降は Launchpad / Spotlight から起動できます。
自分のMacでビルドしたものは隔離属性が付かないので、**警告なくそのまま起動**できます。

### 配布版（.dmg）を受け取って使う場合

他の人がビルドした `.dmg` を受け取った場合は、**ad-hoc 署名（公証なし）**のため初回に
Gatekeeper の警告が出ます。次のどちらかで起動してください（初回のみ。以降は通常起動）。

- `Sonora.app` を **右クリック →「開く」**
- もしくはターミナルで隔離属性を外す：
  ```sh
  xattr -dr com.apple.quarantine /Applications/Sonora.app
  ```

## 使い方

1. アプリを起動し、**YouTube の URL** を貼り付ける
2. **保存先**フォルダを選ぶ（既定はダウンロードフォルダ）
3. 形式を選ぶ
   - そのまま＝**m4a**（変換なし・高音質・すぐ落とせる）
   - **「mp3 に変換する」を ON**＝mp3（最高音質）。初回 ON のときだけ ffmpeg（約 45MB）の取得確認が出る（取得後は次回以降そのまま使える）
4. **「ダウンロード」**を押す。進捗バーとログで状況がわかる
5. 完了後、「ダウンロード済み」の項目をクリックすると Finder で表示される

### プレイリスト／ラジオ

URL に `&list=...` や `&start_radio=1` が含まれていると、自動で曲一覧が表示されます。
落としたい曲だけチェックを残して（既定は全選択）「ダウンロード」。
「全選択 / 全解除」で一括切り替えできます。

## 構成

```
Sonora/
├── ui/                 フロントエンド（HTML/CSS/JS、バニラ）
│   ├── index.html
│   ├── main.js         UI イベント → Rust コマンド呼び出し
│   └── styles.css      テーマ（スレート背景＋パープルのアクセント）
├── src-tauri/
│   ├── src/lib.rs      バックエンド本体（コマンド定義とイベント emit）
│   ├── src/main.rs     エントリポイント
│   ├── bin/            yt-dlp を置く場所（リポジトリには含めず取得する。ffmpeg は同梱しない）
│   ├── icons/          生成済みアプリアイコン（各サイズ）
│   ├── tauri.conf.json Tauri 設定
│   └── Cargo.toml
├── app-icon.svg        アイコンの元データ（ここを編集して再生成）
├── app-icon.png        app-icon.svg を 1024px に書き出したもの
├── package.json
└── dist/               ビルド済み成果物（.app / .dmg）の保管先
```

> mp3 用の ffmpeg は同梱せず、実行時に各ユーザーの App Support
> （`~/Library/Application Support/local.machida.sonora/bin/ffmpeg`）へ取得する。

### バックエンド（`src-tauri/src/lib.rs`）

- `fetch_playlist(url) -> Vec<Entry>` … `--flat-playlist` で曲一覧（番号＋タイトル）を取得。
  単体動画や取得不可のときは空 Vec を返し、フロント側で通常 DL に倒す。
  ラジオ等で取得に数十秒かかってもUIが固まらないよう **async コマンド**にし、
  本体（`fetch_playlist_blocking`）は `tauri::async_runtime::spawn_blocking` で
  別スレッドに逃がしてメインスレッド（UI）をブロックしない
- `start_download(url, outdir, items, mp3)` … `yt-dlp` を起動して音声取得。
  `mp3=false` は `-f bestaudio[ext=m4a]/bestaudio`（変換なし）、`mp3=true` は
  `-x --audio-format mp3 --audio-quality 0 --ffmpeg-location <App Support>/bin`。
  `items`（`"1,3,5"` 形式）があれば `--playlist-items` で選択分のみ、無ければ `--no-playlist`
- `ffmpeg_ready() -> bool` … mp3 変換用の ffmpeg が取得済みかを返す（UI 初期化に使用）
- `ensure_ffmpeg()` … ffmpeg を配布元から App Support 配下へ取得（既にあれば何もしない）。
  ダウンロード中は一時ファイルのサイズから算出した進捗を `ffmpeg-progress`(%) で emit。
  async コマンド＋`spawn_blocking` で UI を固めない
- `reveal_in_finder(path)` … 完成ファイルを Finder で選択表示（macOS の `open -R`）
- yt-dlp の場所は `resource_dir()/bin`、ffmpeg は `app_data_dir()/bin` から解決
- Gatekeeper 対策として、起動前にバイナリの quarantine 属性を外し実行権限を付与
- `yt-dlp` の標準出力を 1 行ずつ読み、`progress` / `file` / `log` / `done` イベントを emit

### デザイン・アイコン

- UI はスレート系のダークテーマにパープル（`#7c3aed`）のアクセント。配色トークンは
  `ui/styles.css` の `:root` にまとまっている
- アプリアイコンの元データは **`app-icon.svg`**（パープルのグラデ角丸スクエア＋
  白い「再生＋イコライザー」モチーフ）。macOS の慣習に合わせて squircle の周囲に
  透明余白を持たせている（`scale(0.8)`）

アイコンを変えたら、`app-icon.svg` を編集してから以下で全サイズを再生成する。

```sh
# 1024px の PNG を書き出し（任意。配布物に app-icon.png を残す用）
rsvg-convert -w 1024 -h 1024 app-icon.svg -o app-icon.png
# macOS / iOS / Android の全アイコン（icon.icns/.ico 含む）を src-tauri/icons へ生成
npx tauri icon app-icon.svg
```

> Dock やウィンドウのアイコンはビルド時に埋め込まれるため、再生成後は
> `npm run tauri build`（または dev 再起動）で反映される。

## セットアップ（開発・ビルド共通）

```sh
git clone https://github.com/machida/Sonora.git
cd Sonora
npm install

# yt-dlp はリポジトリに含めていないので取得する（macOS 用・実行権限付与まで）
mkdir -p src-tauri/bin
curl -L https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_macos \
  -o src-tauri/bin/yt-dlp
chmod +x src-tauri/bin/yt-dlp
```

> ffmpeg は同梱しません。アプリ内で mp3 オプションを ON にしたときに自動取得されます。

## 開発

```sh
npm run tauri dev      # 開発実行（ホットリロード）
```

## ビルド

```sh
npm run tauri build
```

成果物は `src-tauri/target/release/bundle/` に生成される。

- `bundle/macos/Sonora.app`
- `bundle/dmg/Sonora_<version>_aarch64.dmg`

> **注意**: 必ずプロジェクトルート（`Sonora/`）で実行すること。
> 別ディレクトリで `npm run tauri build` すると `package.json` が見つからずエラーになる。

### 署名について

ad-hoc 署名（公証なし）。自分の Mac では問題なく起動するが、他人の Mac に配布すると
Gatekeeper の警告が出る。正式配布には Apple Developer ID による署名＋公証が必要。

### DMG ビルドが失敗する場合

`bundle_dmg.sh` でエラーになるときは、前回の中断で一時 DMG がマウントされたまま残っている
ことが多い。以下で解除・削除してから再ビルドする。

```sh
hdiutil info | grep Sonora                 # マウント中の image-path を確認
hdiutil detach /dev/diskN -force           # 該当ディスクを解除
rm -f src-tauri/target/release/bundle/macos/rw.*.dmg
```

## 必要ツール

- Node.js / npm
- Rust（cargo）
- Tauri CLI（`devDependencies` に含まれる）
- librsvg（`rsvg-convert`）… アイコン再生成で SVG を PNG に書き出す場合のみ（`brew install librsvg`）

## ライセンス / License

### 本プロジェクトのコード

このプロジェクト（Sonora）の**ソースコードは [MIT ライセンス](./LICENSE)** で提供します。

### 同梱・取得する第三者バイナリ

| バイナリ | 役割 | ライセンス | 取り扱い |
| --- | --- | --- | --- |
| `yt-dlp` | 音声のダウンロード | Unlicense（パブリックドメイン相当） | `src-tauri/bin/` に同梱 |
| `ffmpeg` | mp3 への変換（任意機能） | GPLv3 | **同梱しない**。mp3 利用時に[配布元](https://github.com/eugeneware/ffmpeg-static)から各自の環境へ自動ダウンロード |

**ffmpeg(GPL) と本体ライセンスの関係**

- 本アプリは ffmpeg / yt-dlp を**ライブラリとしてリンクせず、別プロセスとして起動**して使います。
  GPL のコピーレフト（派生物への伝播）は静的/動的リンク等で生じるもので、コマンド実行による
  別プロセス連携には及びません。よって**コード本体は MIT のままで問題ありません**。
- さらに ffmpeg は**配布物に同梱せず**、mp3 を使うユーザーが**配布元から直接ダウンロード**する
  方式にしています。Sonora 側で GPL バイナリを「再配布」しないため、**GPLv3 の配布義務
  （ライセンス全文や対応ソースの提供）は発生しません**。
- 注意：ffmpeg バイナリを自前のサーバーやリリースに**再ホスト**したり、`.app`/`.dmg` に
  **同梱**したりすると、その時点で「配布」にあたり GPLv3 遵守が必要になります。現行の
  「実行時に配布元から取得」を維持してください。
- （※将来 `libav*` を自前バイナリへ**静的リンク**する構成に変えた場合は、GPL が本体にも及びます。）
