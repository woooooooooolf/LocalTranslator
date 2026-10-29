import { invoke } from "@tauri-apps/api/core";
import {
  createIcons,
  Languages,
  ArrowRightLeft,
  Type,
  HardDrive,
  Copy,
  Trash2,
  ArrowRight,
  CircleHelp,
  FolderOpen,
  X,
  LoaderCircle,
  Sun,
  Moon,
  Download,
} from "lucide";
import "./style.css";

type Language = { code: string; name: string; targets: string[] };
type Package = { from: string; to: string; version: string };
type Info = { languages: Language[]; packages: Package[] };
type Reply<T> = { data: T; elapsed: number };
type Model = {
  id: string;
  source: string;
  target: string;
  version: string;
  bytes: number;
  installed: boolean;
  present: boolean;
};
type Progress = {
  active: boolean;
  id: string;
  phase: string;
  received: number;
  total: number;
  cancellable: boolean;
};
const desktop = "__TAURI_INTERNALS__" in window;
let info: Info | null = null;
let busy = false;
let modelBusy = false;
let models: Model[] = [];
let pendingDelete: Model | null = null;
let ready = false;
let view = "text";
let translatedSource = "";
let translatedDirection = "";
const names: Record<string, string> = {
  zh: "简体中文",
  en: "英语",
  ja: "日语",
  ko: "韩语",
  fr: "法语",
  de: "德语",
  es: "西班牙语",
  ru: "俄语",
  zt: "繁体中文",
};
const name = (code: string) =>
  names[code] || info?.languages.find((l) => l.code === code)?.name || code;
document.querySelector<HTMLDivElement>("#app")!.innerHTML = `
  <aside class="sidebar">
    <a class="brand" href="#" aria-label="静译首页"><span class="brand-icon"><i data-lucide="languages"></i></span><span>静译</span></a>
    <nav aria-label="主导航">
      <button class="nav-item active" data-view="text"><i data-lucide="type"></i>文字翻译<span class="nav-dot"></span></button>
      <button class="nav-item" data-view="models"><i data-lucide="hard-drive"></i>本地模型</button>
    </nav>
    <div class="sidebar-bottom"><button class="nav-item" id="theme-toggle" aria-label="切换到深色主题"><i data-lucide="moon"></i><span id="theme-label">深色主题</span></button>
    <button class="about-button" id="about"><i data-lucide="circle-help"></i>关于<span>v0.4.0</span></button></div>
  </aside>
  <main>
    <header class="topbar"><h1 id="heading">文字翻译</h1><div class="engine-status" id="engine-status" role="status"><span class="status-dot loading"></span><span id="engine-label">加载中</span></div></header>
    <div class="workspace">
      <div class="toolbar" id="language-toolbar"><div class="language-pair"><label><span class="sr-only">源语言</span><select id="source" disabled><option value="zh">简体中文</option></select></label><button class="swap" id="swap" title="交换语言" aria-label="交换语言" disabled><i data-lucide="arrow-right-left"></i></button><label><span class="sr-only">目标语言</span><select id="target" disabled><option value="en">英语</option></select></label></div></div>
      <section id="text-view" class="view">
        <div class="translation-card"><div class="editor-pane"><div class="pane-header"><span>原文</span><button class="icon-button" id="clear" title="清空文字" aria-label="清空文字"><i data-lucide="trash-2"></i></button></div><textarea id="input" maxlength="20000" autocomplete="off" autocorrect="off" autocapitalize="off" spellcheck="false" placeholder="输入或粘贴文字" aria-label="待翻译文字"></textarea><div class="pane-footer"><span id="count">0 / 20,000</span></div></div>
        <div class="editor-pane output-pane"><div class="pane-header"><span>译文</span><span id="result-time"></span></div><div class="output-container"><div class="empty-result" id="empty-result"><i data-lucide="languages"></i><span>译文</span></div><textarea id="output" readonly autocomplete="off" autocorrect="off" autocapitalize="off" spellcheck="false" aria-label="翻译结果" hidden></textarea></div><div class="pane-footer output-tools"><span id="result-label"></span><div><button class="icon-button" id="copy" title="复制译文" aria-label="复制译文" disabled><i data-lucide="copy"></i></button></div></div></div></div>
        <div class="action-row"><span class="keyboard-hint"><kbd>Ctrl</kbd> + <kbd>Enter</kbd> 快速翻译</span><button class="primary" id="translate" disabled><span>翻译文字</span><i data-lucide="arrow-right"></i></button></div>
      </section>
      <section id="models-view" class="view" hidden><div class="models-header"><h2>中英模型</h2><span class="model-count" id="model-count">加载中</span><button class="secondary" id="refresh-models">刷新</button></div><div id="model-list" class="model-list"></div><div class="download-progress" id="download-progress" hidden role="status"><div><span id="download-label">准备下载</span><button class="secondary" id="cancel-download">取消</button></div><progress id="download-bar" max="100" value="0" aria-label="模型下载进度"></progress><span id="download-size"></span></div><div class="model-note"><i data-lucide="folder-open"></i><div><strong>模型目录</strong><p id="model-path">models</p><span>仅点击下载时联网。来源：Argos / MiniSBD 官方源；下载后可断网使用。</span></div></div></section>
      <div id="message" role="status" aria-live="polite" hidden></div>
    </div>
  </main>
  <dialog id="about-dialog"><button class="icon-button close-dialog" id="close-about" aria-label="关闭"><i data-lucide="x"></i></button><span class="brand-icon"><i data-lucide="languages"></i></span><h2>静译 <small>0.4.0</small></h2><div class="about-details">Tauri · CTranslate2 · MiniSBD<br>CPU 本地翻译</div><div class="about-actions"><button class="secondary" id="notices">第三方说明</button><button class="secondary" id="retry">重新检查引擎</button></div></dialog>
  <dialog id="notices-dialog" class="notices-dialog"><button class="icon-button close-dialog" id="close-notices" aria-label="关闭第三方说明"><i data-lucide="x"></i></button><h2>第三方说明</h2><textarea id="notices-text" readonly spellcheck="false" autocomplete="off" aria-label="第三方说明内容"></textarea></dialog>
  <dialog id="delete-dialog"><h2 class="compact-heading">删除模型</h2><p id="delete-description"></p><div class="about-actions"><button class="secondary" id="cancel-delete">取消</button><button class="secondary danger" id="confirm-delete">删除</button></div></dialog>
`;
const $ = <T extends HTMLElement = HTMLElement>(id: string) =>
  document.getElementById(id) as T;
const input = $<HTMLTextAreaElement>("input");
const output = $<HTMLTextAreaElement>("output");
const source = $<HTMLSelectElement>("source");
const target = $<HTMLSelectElement>("target");
const direction = () => `${source.value}-${target.value}`;
type Theme = "light" | "dark";
let theme: Theme = window.matchMedia("(prefers-color-scheme: dark)").matches
  ? "dark"
  : "light";
async function applyTheme(next: Theme) {
  theme = next;
  document.documentElement.dataset.theme = theme;
  $("theme-toggle").innerHTML =
    `<i data-lucide="${theme === "dark" ? "sun" : "moon"}"></i><span>${theme === "dark" ? "浅色主题" : "深色主题"}</span>`;
  $("theme-toggle").setAttribute(
    "aria-label",
    `切换到${theme === "dark" ? "浅色" : "深色"}主题`,
  );
  icons();
  if (desktop) {
    try {
      await invoke("set_appearance", { theme });
    } catch {
      message("窗口主题同步失败。", true);
    }
  }
}
function icons() {
  createIcons({
    icons: {
      Languages,
      ArrowRightLeft,
      Type,
      HardDrive,
      Copy,
      Trash2,
      ArrowRight,
      CircleHelp,
      FolderOpen,
      X,
      LoaderCircle,
      Sun,
      Moon,
      Download,
    },
  });
}
function message(text: string, error = false) {
  const box = $("message");
  box.textContent = text;
  box.className = error ? "error" : "success";
  box.hidden = !text;
}
function update() {
  $("count").textContent = `${input.value.length.toLocaleString()} / 20,000`;
  for (const id of ["translate", "swap"])
    $<HTMLButtonElement>(id).disabled = busy || modelBusy || !ready;
  $<HTMLButtonElement>("translate").disabled ||= !input.value.trim();
  $<HTMLButtonElement>("clear").disabled =
    busy || (!input.value && !output.value);
  $<HTMLButtonElement>("swap").disabled ||= !info?.languages
    .find((language) => language.code === target.value)
    ?.targets.includes(source.value);
  source.disabled = target.disabled = busy || modelBusy || !ready;
  $<HTMLButtonElement>("copy").disabled = !output.value || busy;
  $("empty-result").hidden = !!output.value;
  output.hidden = !output.value;
  const stale =
    output.value &&
    (translatedSource !== input.value || translatedDirection !== direction());
  $("result-label").textContent = busy
    ? "正在本机翻译，请稍候…"
    : stale
      ? "原文或语言已更改，请重新翻译"
      : output.value
        ? "翻译完成"
        : "";
  $("translate").innerHTML = busy
    ? '<i data-lucide="loader-circle" class="spin"></i><span>正在翻译</span>'
    : '<span>翻译文字</span><i data-lucide="arrow-right"></i>';
  icons();
  document
    .querySelectorAll<HTMLButtonElement>("[data-model-action], #refresh-models")
    .forEach((button) => {
      button.disabled = busy || modelBusy || !desktop;
    });
}
function fillSelect(
  select: HTMLSelectElement,
  languages: Language[],
  preferred: string,
) {
  select.replaceChildren(
    ...languages.map((l) => new Option(name(l.code), l.code)),
  );
  if (languages.some((l) => l.code === preferred)) select.value = preferred;
}
function fillTargets(preferred = target.value) {
  const targets =
    info?.languages.find((l) => l.code === source.value)?.targets || [];
  fillSelect(
    target,
    info!.languages.filter((l) => targets.includes(l.code)),
    preferred,
  );
  ready = !!target.value;
  update();
}
function switchView(next: string) {
  view = next;
  for (const key of ["text", "models"]) $(`${key}-view`).hidden = key !== next;
  document
    .querySelectorAll<HTMLButtonElement>("[data-view]")
    .forEach((b) => b.classList.toggle("active", b.dataset.view === next));
  $("language-toolbar").hidden = next === "models";
  $("heading").textContent = next === "models" ? "本地模型" : "文字翻译";
  message("");
}
async function load() {
  ready = false;
  update();
  $("engine-label").textContent = desktop ? "正在加载引擎" : "浏览器预览";
  if (!desktop) {
    message("这是界面预览。请运行桌面版以使用本地翻译。");
    return;
  }
  $("engine-status").classList.remove("failed");
  $("engine-status").querySelector(".status-dot")?.classList.add("loading");
  try {
    const reply = await invoke<Reply<Info>>("engine_info");
    info = reply.data;

    fillSelect(
      source,
      info.languages.filter((l) => l.targets.length),
      source.value,
    );
    fillTargets(target.value);
    $("engine-label").textContent = "离线引擎已就绪";
    await loadModels();
    const root = await invoke<string>("runtime_location");
    $("model-path").textContent = `${root}\\models`;
    if (!info.languages.length || !info.packages.length) {
      ready = false;
      $("engine-label").textContent = "未安装模型";
      switchView("models");
      message("请在“本地模型”中下载需要的翻译方向。");
      return;
    }
    message("");
  } catch (e) {
    $("engine-label").textContent = "引擎需要检查";
    $("engine-status").classList.add("failed");
    message(String(e), true);
  } finally {
    $("engine-status")
      .querySelector(".status-dot")
      ?.classList.remove("loading");
    update();
  }
}
async function translateText() {
  if (busy || modelBusy || !ready || !input.value.trim()) return;
  busy = true;
  input.readOnly = true;
  message("");
  update();
  const text = input.value,
    dir = direction();
  try {
    const reply = await invoke<Reply<{ text: string }>>("translate_text", {
      text,
      source: source.value,
      target: target.value,
    });
    output.value = reply.data.text;
    translatedSource = text;
    translatedDirection = dir;
    $("result-time").textContent = `${reply.elapsed.toFixed(1)} 秒`;
  } catch (e) {
    message(String(e), true);
  } finally {
    busy = false;
    input.readOnly = false;
    update();
  }
}

const megabytes = (bytes: number) => `${(bytes / 1024 / 1024).toFixed(1)} MB`;
async function loadModels() {
  models = await invoke<Model[]>("model_catalog");
  $("model-count").textContent =
    `${models.filter((m) => m.installed).length} / ${models.length} 已安装`;
  $("model-list").replaceChildren(
    ...models.map((model) => {
      const card = document.createElement("div");
      card.className = "model-card";
      const badge = document.createElement("span");
      badge.className = "model-badge";
      badge.textContent = model.source.toUpperCase();
      const title = document.createElement("h3");
      title.textContent = `${name(model.source)} → ${name(model.target)}`;
      const description = document.createElement("p");
      description.textContent = `v${model.version} · 下载 ${megabytes(model.bytes)}`;
      const details = document.createElement("div");
      details.className = "model-details";
      details.append(title, description);
      const status = document.createElement("span");
      status.className = "installed";
      status.textContent = model.installed
        ? "已安装"
        : model.present
          ? "文件不完整"
          : "未安装";
      const action = document.createElement("button");
      action.className = model.present ? "secondary danger" : "secondary";
      action.dataset.modelAction = model.id;
      action.setAttribute(
        "aria-label",
        `${model.present ? "删除" : "下载"}${title.textContent}模型`,
      );
      action.innerHTML = model.present
        ? '<i data-lucide="trash-2"></i>删除'
        : '<i data-lucide="download"></i>下载';
      action.onclick = () => {
        if (busy || modelBusy) return;
        if (model.present) {
          pendingDelete = model;
          $("delete-description").textContent =
            `删除“${title.textContent}”模型？此方向将暂时无法翻译，可随时重新下载。`;
          $<HTMLDialogElement>("delete-dialog").showModal();
          $("cancel-delete").focus();
        } else void manageModel(model, false);
      };
      card.append(badge, details, status, action);
      return card;
    }),
  );
  update();
}
async function manageModel(model: Model, remove: boolean) {
  if (busy || modelBusy || !desktop) return;
  modelBusy = true;
  $("engine-label").textContent = "模型管理中";
  message("");
  update();
  $("download-progress").hidden = false;
  $("download-label").textContent = remove ? "删除中" : "准备下载";
  $("download-size").textContent = "";
  $<HTMLProgressElement>("download-bar").value = 0;
  $<HTMLButtonElement>("cancel-download").disabled = remove;
  let polling = false;
  const poll = async () => {
    if (polling) return;
    polling = true;
    try {
      const progress = await invoke<Progress>("model_progress");
      if (!modelBusy || !progress.active) return;
      $("download-label").textContent =
        `${name(model.source)} → ${name(model.target)} · ${progress.phase}`;
      $("download-size").textContent = remove
        ? ""
        : `${megabytes(progress.received)} / ${megabytes(progress.total)}`;
      $<HTMLProgressElement>("download-bar").value = progress.total
        ? Math.min(100, (progress.received / progress.total) * 100)
        : 0;
      $<HTMLButtonElement>("cancel-download").disabled = !progress.cancellable;
    } catch {
      /* The operation itself reports IPC errors. */
    } finally {
      polling = false;
    }
  };
  const timer = window.setInterval(() => void poll(), 400);
  let result = remove ? "模型已删除" : "模型已安装，可离线翻译";
  let failed = false;
  try {
    await invoke(remove ? "remove_model" : "install_model", { id: model.id });
  } catch (error) {
    result = String(error);
    failed = true;
  } finally {
    window.clearInterval(timer);
    modelBusy = false;
    $("download-progress").hidden = true;
    await load();
    message(result, failed);
    update();
  }
}
$("refresh-models").onclick = () => {
  if (!busy && !modelBusy) void load();
};
$("cancel-download").onclick = async () => {
  $<HTMLButtonElement>("cancel-download").disabled = true;
  try {
    await invoke("cancel_model_download");
  } catch {
    message("无法取消，请稍后重试。", true);
  }
};
$("cancel-delete").onclick = () => {
  pendingDelete = null;
  $<HTMLDialogElement>("delete-dialog").close();
};
$("confirm-delete").onclick = () => {
  $<HTMLDialogElement>("delete-dialog").close();
  if (pendingDelete) void manageModel(pendingDelete, true);
  pendingDelete = null;
};
input.addEventListener("input", update);
source.addEventListener("change", () => fillTargets());
target.addEventListener("change", update);
$("swap").onclick = () => {
  const previous = source.value;
  source.value = target.value;
  fillTargets(previous);
  if (output.value) {
    input.value = output.value;
    output.value = "";
  }
  update();
};
$("translate").onclick = translateText;
$("clear").onclick = () => {
  input.value = output.value = translatedSource = translatedDirection = "";
  $("result-time").textContent = "";
  message("");
  update();
  input.focus();
};
$("copy").onclick = async () => {
  try {
    await navigator.clipboard.writeText(output.value);
    message("译文已复制");
  } catch {
    message("复制失败，请选中译文后按 Ctrl+C。", true);
  }
};

document
  .querySelectorAll<HTMLButtonElement>("[data-view]")
  .forEach(
    (button) => (button.onclick = () => switchView(button.dataset.view!)),
  );
document.querySelector<HTMLAnchorElement>(".brand")!.onclick = (e) => {
  e.preventDefault();
  switchView("text");
};
$("about").onclick = () => $<HTMLDialogElement>("about-dialog").showModal();
$("theme-toggle").onclick = () => {
  void applyTheme(theme === "light" ? "dark" : "light");
};
$("close-about").onclick = () => $<HTMLDialogElement>("about-dialog").close();
$("notices").onclick = async () => {
  try {
    $<HTMLTextAreaElement>("notices-text").value = desktop
      ? await invoke<string>("third_party_notices")
      : "第三方说明内置于桌面版。";
    $<HTMLDialogElement>("about-dialog").close();
    $<HTMLDialogElement>("notices-dialog").showModal();
  } catch {
    message("无法显示第三方说明。", true);
  }
};
$("close-notices").onclick = () =>
  $<HTMLDialogElement>("notices-dialog").close();
$("retry").onclick = () => {
  if (!busy && !modelBusy) {
    $<HTMLDialogElement>("about-dialog").close();
    void load();
  }
};
document.addEventListener("keydown", (e) => {
  if ((e.ctrlKey || e.metaKey) && e.key === "Enter" && view === "text") {
    e.preventDefault();
    void translateText();
  }
});
document.addEventListener("contextmenu", (event) => {
  if (
    !(event.target instanceof Element) ||
    !event.target.closest("textarea, input, [contenteditable='true']")
  ) {
    event.preventDefault();
  }
});
window.addEventListener("pagehide", () => {
  input.value = output.value = translatedSource = translatedDirection = "";
  message("");
});
icons();
void applyTheme(theme);
void load();
