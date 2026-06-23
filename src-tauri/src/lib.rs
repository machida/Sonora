use serde::Serialize;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tauri::{AppHandle, Emitter, Manager};

/// mp3 変換に使う ffmpeg(GPL)を、配布元から各自の環境へ直接取得するための URL。
/// 同梱せずユーザー環境にダウンロードする運用にすることで、GPL バイナリの
/// 再配布義務を負わない。universal ビルドでは実行中スライスのアーキに合わせて
/// arm64 / x86_64 のどちらかを返す(`cfg!(target_arch)` はスライスごとに評価される)。
fn ffmpeg_url() -> &'static str {
    if cfg!(target_arch = "x86_64") {
        "https://github.com/eugeneware/ffmpeg-static/releases/download/b6.1.1/ffmpeg-darwin-x64"
    } else {
        "https://github.com/eugeneware/ffmpeg-static/releases/download/b6.1.1/ffmpeg-darwin-arm64"
    }
}

/// プレイリスト 1 件分の情報。
#[derive(Serialize)]
struct Entry {
    /// プレイリスト内の通し番号(1 始まり)。--playlist-items で使う。
    index: u64,
    title: String,
}

/// 同梱バイナリ(bin/ = yt-dlp)の場所を返す。
fn bin_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let res = app.path().resource_dir().map_err(|e| e.to_string())?;
    Ok(res.join("bin"))
}

/// ダウンロードした ffmpeg を置くディレクトリ(App Support 配下)。
/// 同梱しないため、ここに各自が取得した ffmpeg を保管する。
fn ffmpeg_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let data = app.path().app_data_dir().map_err(|e| e.to_string())?;
    Ok(data.join("bin"))
}

fn ffmpeg_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(ffmpeg_dir(app)?.join("ffmpeg"))
}

/// 配布先 Mac でも同梱/取得バイナリが起動できるよう、quarantine を外し実行権限を付ける。
fn prepare(dir: &Path) {
    // 公証していないので、ダウンロード由来の隔離属性が付いていると
    // 入れ子のバイナリ実行が Gatekeeper にブロックされる。先に外しておく。
    let _ = Command::new("/usr/bin/xattr")
        .args(["-dr", "com.apple.quarantine"])
        .arg(dir)
        .status();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for f in ["yt-dlp", "ffmpeg", "ffprobe"] {
            let p = dir.join(f);
            if let Ok(meta) = std::fs::metadata(&p) {
                let mut perm = meta.permissions();
                perm.set_mode(0o755);
                let _ = std::fs::set_permissions(&p, perm);
            }
        }
    }
}

/// "[download]   12.3% of ..." のような行から進捗(%)を取り出す。
fn parse_percent(line: &str) -> Option<f64> {
    let idx = line.find('%')?;
    let prefix = &line[..idx];
    let start = prefix.rfind(char::is_whitespace).map(|i| i + 1).unwrap_or(0);
    prefix[start..].trim().parse::<f64>().ok()
}

/// yt-dlp の出力から完成した音声ファイルの絶対パスを取り出す。
/// - "[ExtractAudio] Destination: /path/x.mp3"
/// - "[download] Destination: /path/x.m4a"
/// - "[download] /path/x.m4a has already been downloaded"
/// `exts` に含まれる拡張子のものだけ拾う(mp3 変換時は中間ファイルを無視するため)。
fn parse_dest(line: &str, exts: &[&str]) -> Option<String> {
    let path = if let Some(p) = line.split("] Destination: ").nth(1) {
        p.trim().to_string()
    } else if let Some(rest) = line.strip_prefix("[download] ") {
        rest.strip_suffix(" has already been downloaded")?
            .trim()
            .to_string()
    } else {
        return None;
    };
    exts.iter()
        .any(|e| path.to_lowercase().ends_with(&format!(".{e}")))
        .then_some(path)
}

/// Finder で対象ファイルを選択状態で表示する(macOS の `open -R`)。
#[tauri::command]
fn reveal_in_finder(path: String) -> Result<(), String> {
    Command::new("/usr/bin/open")
        .arg("-R")
        .arg(&path)
        .status()
        .map_err(|e| format!("Finder で開けませんでした: {e}"))?;
    Ok(())
}

/// ffmpeg が取得済みか(mp3 変換が使えるか)を返す。
#[tauri::command]
fn ffmpeg_ready(app: AppHandle) -> bool {
    ffmpeg_path(&app).map(|p| p.exists()).unwrap_or(false)
}

/// mp3 変換用の ffmpeg を配布元から取得して App Support 配下へ置く。
/// 既にあれば何もしない。ダウンロード中は "ffmpeg-progress"(%) を emit する。
#[tauri::command]
async fn ensure_ffmpeg(app: AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || ensure_ffmpeg_blocking(&app))
        .await
        .map_err(|e| format!("ffmpeg 取得タスクの実行に失敗しました: {e}"))?
}

/// HTTP ヘッダから content-length を取り出す(リダイレクト後の最後の値)。
fn remote_size(url: &str) -> Option<u64> {
    let out = Command::new("/usr/bin/curl")
        .args(["-sIL", url])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    text.lines()
        .filter(|l| l.to_lowercase().starts_with("content-length:"))
        .filter_map(|l| l.split(':').nth(1)?.trim().parse::<u64>().ok())
        .last()
}

fn ensure_ffmpeg_blocking(app: &AppHandle) -> Result<(), String> {
    let dest = ffmpeg_path(app)?;
    if dest.exists() {
        return Ok(());
    }
    let dir = ffmpeg_dir(app)?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("保存先を作成できません: {e}"))?;
    let tmp = dir.join("ffmpeg.download");
    let _ = std::fs::remove_file(&tmp);

    let url = ffmpeg_url();
    let total = remote_size(url).unwrap_or(0);
    let _ = app.emit("ffmpeg-progress", 0.0_f64);

    // curl で配布元から直接ダウンロード(自前サーバーに再ホストしない = 再配布にならない)。
    let mut child = Command::new("/usr/bin/curl")
        .args(["-L", "--fail", "--silent", "--show-error", "-o"])
        .arg(&tmp)
        .arg(url)
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("ダウンロードを開始できません: {e}"))?;

    // 進捗は一時ファイルのサイズを定期的に見て算出する。
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {}
            Err(e) => return Err(format!("ダウンロードの監視に失敗しました: {e}")),
        }
        if total > 0 {
            if let Ok(meta) = std::fs::metadata(&tmp) {
                let pct = (meta.len() as f64 / total as f64 * 100.0).min(99.0);
                let _ = app.emit("ffmpeg-progress", pct);
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }

    let status = child.wait().map_err(|e| e.to_string())?;
    if !status.success() {
        let mut err = String::new();
        if let Some(mut s) = child.stderr.take() {
            use std::io::Read;
            let _ = s.read_to_string(&mut err);
        }
        let _ = std::fs::remove_file(&tmp);
        let msg = err.lines().last().unwrap_or("").trim();
        return Err(format!("ffmpeg のダウンロードに失敗しました: {msg}"));
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = std::fs::metadata(&tmp) {
            let mut perm = meta.permissions();
            perm.set_mode(0o755);
            let _ = std::fs::set_permissions(&tmp, perm);
        }
    }
    // 取得物にも quarantine が付くので外す。
    let _ = Command::new("/usr/bin/xattr")
        .args(["-d", "com.apple.quarantine"])
        .arg(&tmp)
        .status();

    std::fs::rename(&tmp, &dest).map_err(|e| format!("保存に失敗しました: {e}"))?;
    let _ = app.emit("ffmpeg-progress", 100.0_f64);
    Ok(())
}

/// ダウンロードせずにプレイリストの中身を一覧する(--flat-playlist なので高速)。
/// 単体動画 URL や一覧取得できないものは空 Vec を返し、フロント側で通常 DL に倒す。
/// 取得に数十秒かかることがある(ラジオ等)ため async コマンドにし、本体は
/// spawn_blocking で別スレッドへ逃がして UI(メインスレッド)を固めないようにする。
#[tauri::command]
async fn fetch_playlist(app: AppHandle, url: String) -> Result<Vec<Entry>, String> {
    tauri::async_runtime::spawn_blocking(move || fetch_playlist_blocking(&app, &url))
        .await
        .map_err(|e| format!("一覧取得タスクの実行に失敗しました: {e}"))?
}

/// fetch_playlist の本体。yt-dlp の起動と待機を行うブロッキング処理。
fn fetch_playlist_blocking(app: &AppHandle, url: &str) -> Result<Vec<Entry>, String> {
    let dir = bin_dir(app)?;
    prepare(&dir);

    let ytdlp = dir.join("yt-dlp");
    if !ytdlp.exists() {
        return Err(format!("yt-dlp が見つかりません: {}", ytdlp.display()));
    }

    let out = Command::new(&ytdlp)
        .args([
            "--flat-playlist",
            "--no-warnings",
            "--ignore-errors",
            "--print",
            "%(playlist_index)s\t%(title)s",
        ])
        .arg(url)
        .output()
        .map_err(|e| format!("起動に失敗しました: {e}"))?;

    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        let msg = err.lines().last().unwrap_or("").trim();
        return Err(format!("一覧の取得に失敗しました: {msg}"));
    }

    let text = String::from_utf8_lossy(&out.stdout);
    let mut entries = Vec::new();
    for line in text.lines() {
        let line = line.trim_end();
        if line.is_empty() {
            continue;
        }
        let mut parts = line.splitn(2, '\t');
        let idx = parts.next().unwrap_or("").trim();
        let title = parts.next().unwrap_or("").to_string();
        // 単体動画は playlist_index が "NA" になる。プレイリストとして扱わない。
        match idx.parse::<u64>() {
            Ok(index) => entries.push(Entry { index, title }),
            Err(_) => return Ok(Vec::new()),
        }
    }
    // 1 件だけならプレイリスト扱いせず通常 DL に倒す。
    if entries.len() <= 1 {
        return Ok(Vec::new());
    }
    Ok(entries)
}

#[tauri::command]
fn start_download(
    app: AppHandle,
    url: String,
    outdir: String,
    items: Option<String>,
    mp3: bool,
) -> Result<(), String> {
    let dir = bin_dir(&app)?;
    prepare(&dir);

    let ytdlp = dir.join("yt-dlp");
    if !ytdlp.exists() {
        return Err(format!("yt-dlp が見つかりません: {}", ytdlp.display()));
    }

    let ffdir = ffmpeg_dir(&app)?;
    if mp3 && !ffdir.join("ffmpeg").exists() {
        return Err("mp3 変換には ffmpeg が必要です。mp3 オプションを ON にして取得してください。".into());
    }
    prepare(&ffdir);

    let template = format!("{}/%(title)s.%(ext)s", outdir.trim_end_matches('/'));

    let mut cmd = Command::new(&ytdlp);
    cmd.arg("--newline");
    if mp3 {
        // ffmpeg で mp3 に変換(最高音質)。
        cmd.args(["-x", "--audio-format", "mp3", "--audio-quality", "0"]);
        cmd.arg("--ffmpeg-location").arg(&ffdir);
    } else {
        // 変換なし。配信されている音声(m4a 優先)をそのまま保存 = ffmpeg 不要。
        cmd.args(["-f", "bestaudio[ext=m4a]/bestaudio"]);
    }
    match items.as_deref() {
        // 選択された曲だけプレイリストから取得する(例: "1,3,5")。
        Some(it) if !it.is_empty() => {
            cmd.arg("--yes-playlist");
            cmd.arg("--playlist-items").arg(it);
        }
        // 選択なし = URL に &list=... が付いていても単体動画のみ取得する。
        _ => {
            cmd.arg("--no-playlist");
        }
    }
    cmd.arg("-o").arg(&template).arg(&url);

    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("起動に失敗しました: {e}"))?;

    let stdout = child.stdout.take().expect("stdout");
    let stderr = child.stderr.take().expect("stderr");

    // mp3 変換時は最終ファイル(.mp3)だけ拾う。それ以外は配信音声の拡張子を拾う。
    let exts: Vec<&'static str> = if mp3 {
        vec!["mp3"]
    } else {
        vec!["m4a", "webm", "opus", "aac", "mp4", "mp3"]
    };
    let exts_err = exts.clone();

    // 標準エラーはログとして流す(まれにこちらへ Destination が出るので拾う)
    let app_err = app.clone();
    std::thread::spawn(move || {
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            if let Some(path) = parse_dest(&line, &exts_err) {
                let _ = app_err.emit("file", path);
            }
            let _ = app_err.emit("log", line);
        }
    });

    // 標準出力はログ＋進捗＋完成ファイル、最後に完了通知
    let app_out = app.clone();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if let Some(p) = parse_percent(&line) {
                let _ = app_out.emit("progress", p);
            }
            if let Some(path) = parse_dest(&line, &exts) {
                let _ = app_out.emit("file", path);
            }
            let _ = app_out.emit("log", line);
        }
        let ok = child.wait().map(|s| s.success()).unwrap_or(false);
        let _ = app_out.emit("done", ok);
    });

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            start_download,
            fetch_playlist,
            reveal_in_finder,
            ffmpeg_ready,
            ensure_ffmpeg
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
