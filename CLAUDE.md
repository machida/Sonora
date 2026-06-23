# CLAUDE.md — Sonora 開発ガイド（AI 向け指示）

このファイルは Claude / AI エージェントがこのリポジトリで作業するときの指針です。
**「守るべき不変条件」は理由とともに必ず尊重すること。**

## プロジェクト概要

Sonora は YouTube の URL から音声をダウンロードする macOS デスクトップアプリ（Tauri 2 + バニラ HTML/CSS/JS）。
- 既定は **m4a**（yt-dlp で変換せずそのまま保存。ffmpeg 不要）
- **mp3 は任意**。アプリ内トグルを ON にした時点で、変換用 ffmpeg を実行時に自動取得する
- 配布は GitHub Releases の **universal `.dmg`**（公証なし）

## 守るべき不変条件（変更しないこと・理由つき）

1. **ffmpeg を配布物に同梱しない / 自前で再ホストしない。**
   ffmpeg は GPLv3。アプリ内（`ensure_ffmpeg`）で**配布元から各ユーザーが直接ダウンロード**する方式を維持する。
   同梱・再ホストすると「配布」にあたり GPLv3 の義務（ライセンス全文＋対応ソース提供）が発生する。
   → `tauri.conf.json` の `bundle.resources` に ffmpeg を加えない。`bin/` にコミットしない。
2. **yt-dlp / ffmpeg は別プロセスとして起動する（ライブラリ静的リンクしない）。**
   これによりコード本体は **MIT** を維持できる。`libav*` を静的リンクすると GPL が本体に及ぶ。
3. **`src-tauri/bin/`（yt-dlp）はリポジトリにコミットしない**（`.gitignore` 済み）。取得手順は README 参照。
4. **配布ビルドは universal（Intel + Apple Silicon）。**
   `npm run tauri build -- --target universal-apple-darwin`。
   ffmpeg のダウンロード URL はアーキ別（`ffmpeg_url()` が `cfg!(target_arch)` で出し分け）。yt-dlp も universal 版（`yt-dlp_macos`）を使う。
5. **既定フォーマットは m4a。** mp3 を既定にしない（ffmpeg 必須になり依存・GPL の話が常時絡むため）。

## リリース手順（重要）

1. バージョンを上げる（`src-tauri/tauri.conf.json` と `src-tauri/Cargo.toml` の `version`）。
2. universal でビルドする（上記コマンド）。
3. タグを打って push し、`gh release create` で `.dmg` を添付する。
4. **リリースノートは必ず [`.github/RELEASE_TEMPLATE.md`](.github/RELEASE_TEMPLATE.md) をベースに作成し、
   「⚠️ 免責事項・利用上の注意」セクションを毎回必ず含める。** これは削除・省略しないこと
   （非技術ユーザーは README を読まず Releases から直接 DL するため、注意喚起を配布の起点に必ず置く）。

## よく使うコマンド

```sh
npm run tauri dev                                   # 開発実行
npm run tauri build -- --target universal-apple-darwin   # 配布用ビルド
npx tauri icon app-icon.svg                         # アイコン再生成（元データは app-icon.svg）
```

## コードの要点

- バックエンドのコマンドは `src-tauri/src/lib.rs`：
  `fetch_playlist` / `start_download` / `ffmpeg_ready` / `ensure_ffmpeg` / `reveal_in_finder`。
  時間のかかる処理（一覧取得・ffmpeg 取得）は **async コマンド + `spawn_blocking`** で UI を固めない。
- フロントは `ui/`（`index.html` / `main.js` / `styles.css`）。DOM の id を変えると `main.js` と連動が切れるので注意。
- UI テーマはスレート＋パープル（`#7c3aed`）。配色トークンは `ui/styles.css` の `:root`。
