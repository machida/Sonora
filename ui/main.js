const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const dialog = window.__TAURI__.dialog;
const tpath = window.__TAURI__.path;

const $ = (id) => document.getElementById(id);

// 初期保存先 = ダウンロードフォルダ
(async () => {
  try {
    $("dir").value = await tpath.downloadDir();
  } catch (_) {
    $("dir").value = "";
  }
})();

function log(line) {
  const el = $("log");
  el.textContent += line + "\n";
  el.scrollTop = el.scrollHeight;
}

$("choose").addEventListener("click", async () => {
  const sel = await dialog.open({
    directory: true,
    defaultPath: $("dir").value || undefined,
  });
  if (sel) $("dir").value = sel;
});

listen("log", (e) => log(e.payload));
listen("progress", (e) => {
  $("fill").style.width = e.payload + "%";
});

// ---- mp3 変換（ffmpeg）---------------------------------------------------
// 既定は m4a（変換なし・ffmpeg 不要）。mp3 を選んだときだけ ffmpeg を取得する。

let ffmpegReady = false; // ffmpeg 取得済みか
let ffmpegBusy = false; // 取得処理中か

(async () => {
  try {
    ffmpegReady = await invoke("ffmpeg_ready");
  } catch (_) {
    ffmpegReady = false;
  }
})();

listen("ffmpeg-progress", (e) => {
  if (!ffmpegBusy) return;
  $("ffstatus").textContent = `ffmpeg を準備中… ${Math.round(e.payload)}%`;
});

$("mp3").addEventListener("change", async () => {
  const box = $("mp3");
  if (!box.checked) {
    $("ffstatus").textContent = "既定は m4a（高音質・変換なし）";
    return;
  }
  if (ffmpegReady) {
    $("ffstatus").textContent = "mp3 に変換します";
    return;
  }
  // 未取得なら確認のうえダウンロード（配布元から直接取得）
  const ok = confirm(
    "mp3 変換には ffmpeg（約 45MB）が必要です。今すぐダウンロードしますか？"
  );
  if (!ok) {
    box.checked = false;
    return;
  }
  ffmpegBusy = true;
  box.disabled = true;
  $("go").disabled = true;
  $("ffstatus").textContent = "ffmpeg を準備中… 0%";
  try {
    await invoke("ensure_ffmpeg");
    ffmpegReady = true;
    $("ffstatus").textContent = "✅ mp3 に変換します";
  } catch (err) {
    box.checked = false;
    $("ffstatus").textContent = "⚠️ ffmpeg を取得できませんでした";
    log("❌ " + err);
  } finally {
    ffmpegBusy = false;
    box.disabled = false;
    $("go").disabled = false;
  }
});

// ---- ダウンロード済みファイル -------------------------------------------

const seenFiles = new Set();

function baseName(p) {
  return p.split("/").pop();
}

listen("file", (e) => {
  const path = e.payload;
  if (!path || seenFiles.has(path)) return;
  seenFiles.add(path);

  const row = document.createElement("button");
  row.className = "file";
  row.type = "button";
  row.title = path;
  row.innerHTML = `<span class="ficon">📁</span><span class="fname"></span>`;
  row.querySelector(".fname").textContent = baseName(path);
  row.addEventListener("click", async () => {
    try {
      await invoke("reveal_in_finder", { path });
    } catch (err) {
      log("❌ Finder で開けませんでした: " + err);
    }
  });
  $("files").append(row);
  $("donewrap").hidden = false;
});
listen("done", (e) => {
  $("go").disabled = false;
  if (e.payload) {
    $("fill").style.width = "100%";
    log("✅ 完了しました。");
  } else {
    log("❌ 失敗しました。ログを確認してください。");
  }
});

// ---- プレイリスト一覧 ----------------------------------------------------

let lastUrl = ""; // 取得済み URL（同じ URL の二重取得を防ぐ）

function clearList() {
  $("listwrap").hidden = true;
  $("list").innerHTML = "";
  $("count").textContent = "";
  lastUrl = "";
}

function renderList(entries) {
  const list = $("list");
  list.innerHTML = "";
  for (const e of entries) {
    const row = document.createElement("label");
    row.className = "item";
    const cb = document.createElement("input");
    cb.type = "checkbox";
    cb.checked = true; // 既定で全部チェック
    cb.dataset.index = e.index;
    const span = document.createElement("span");
    span.textContent = `${e.index}. ${e.title}`;
    row.append(cb, span);
    list.append(row);
  }
  $("count").textContent = `このリストには ${entries.length} 曲あります`;
  $("listwrap").hidden = false;
}

async function fetchList(url) {
  if (!url || url === lastUrl) return;
  // プレイリスト/ラジオでなければ取りに行かない（単体DLを遅くしない）
  if (!url.includes("list=")) {
    clearList();
    return;
  }
  lastUrl = url;
  $("count").textContent = "一覧を取得中…";
  $("list").innerHTML = "";
  $("listwrap").hidden = false;
  try {
    const entries = await invoke("fetch_playlist", { url });
    if (!entries || entries.length === 0) {
      clearList(); // 単体動画扱い
      return;
    }
    if (url !== $("url").value.trim()) return; // 取得中に URL が変わっていたら破棄
    renderList(entries);
  } catch (err) {
    $("count").textContent = "⚠️ 一覧を取得できませんでした（単体としてDLします）";
    $("list").innerHTML = "";
  }
}

let urlTimer = null;
$("url").addEventListener("input", () => {
  clearTimeout(urlTimer);
  const url = $("url").value.trim();
  if (!url.includes("list=")) {
    clearList();
    return;
  }
  urlTimer = setTimeout(() => fetchList(url), 600);
});

function setAll(checked) {
  $("list")
    .querySelectorAll("input[type=checkbox]")
    .forEach((cb) => (cb.checked = checked));
}
$("all").addEventListener("click", (e) => {
  e.preventDefault();
  setAll(true);
});
$("none").addEventListener("click", (e) => {
  e.preventDefault();
  setAll(false);
});

// 選択された曲番号を "1,3,5" 形式で返す。リスト非表示なら null（=単体DL）。
function selectedItems() {
  if ($("listwrap").hidden) return null;
  const idxs = [...$("list").querySelectorAll("input[type=checkbox]:checked")].map(
    (cb) => cb.dataset.index
  );
  return idxs.length ? idxs.join(",") : "";
}

// ---- ダウンロード --------------------------------------------------------

$("go").addEventListener("click", async () => {
  const url = $("url").value.trim();
  const dir = $("dir").value.trim();
  if (!url) {
    alert("URL を入力してください。");
    return;
  }
  if (!dir) {
    alert("保存先を選んでください。");
    return;
  }
  const items = selectedItems();
  if (items === "") {
    alert("ダウンロードする曲を 1 つ以上選んでください。");
    return;
  }
  const mp3 = $("mp3").checked;
  $("go").disabled = true;
  $("fill").style.width = "0%";
  log("開始: " + url);
  log(mp3 ? "形式: mp3（変換あり）" : "形式: m4a（変換なし）");
  if (items) log(`選択: ${items.split(",").length} 曲`);
  try {
    await invoke("start_download", { url, outdir: dir, items, mp3 });
  } catch (err) {
    log("❌ " + err);
    $("go").disabled = false;
  }
});
