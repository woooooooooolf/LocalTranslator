import { invoke } from "@tauri-apps/api/core";
import {
  createIcons,
  Languages,
  ArrowRightLeft,
  FileText,
  Type,
  HardDrive,
  Copy,
  Download,
  Trash2,
  ArrowRight,
  CircleHelp,
  FolderOpen,
  X,
  LoaderCircle,
  Sun,
  Moon,
} from "lucide";
import "./style.css";

type Language = { code: string; name: string; targets: string[] };
type Package = { from: string; to: string; version: string };
type Info = { languages: Language[]; packages: Package[]; formats: string[] };
type Reply<T> = { data: T; elapsed: number };
const desktop = "__TAURI_INTERNALS__" in window;
let info: Info | null = null;
let busy = false;
let ready = false;
let view = "text";
let documentPath = "";
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
      <button class="nav-item" data-view="file"><i data-lucide="file-text"></i>文档翻译</button>
      <button class="nav-item" data-view="models"><i data-lucide="hard-drive"></i>本地模型</button>
    </nav>
    <div class="sidebar-bottom"><button class="nav-item" id="theme-toggle" aria-label="切换到深色主题"><i data-lucide="moon"></i><span id="theme-label">深色主题</span></button>
    <button class="about-button" id="about"><i data-lucide="circle-help"></i>关于<span>v0.2.0</span></button></div>
  </aside>
  <main>
    <header class="topbar"><h1 id="heading">文字翻译</h1><div class="engine-status" id="engine-status" role="status"><span class="status-dot loading"></span><span id="engine-label">加载中</span></div></header>
    <div class="workspace">
      <div class="toolbar" id="language-toolbar"><div class="language-pair"><label><span class="sr-only">源语言</span><select id="source" disabled><option value="zh">简体中文</option></select></label><button class="swap" id="swap" title="交换语言" aria-label="交换语言" disabled><i data-lucide="arrow-right-left"></i></button><label><span class="sr-only">目标语言</span><select id="target" disabled><option value="en">英语</option></select></label></div></div>
      <section id="text-view" class="view">
        <div class="translation-card"><div class="editor-pane"><div class="pane-header"><span>原文</span><button class="icon-button" id="clear" title="清空文字" aria-label="清空文字"><i data-lucide="trash-2"></i></button></div><textarea id="input" maxlength="20000" autocomplete="off" autocorrect="off" autocapitalize="off" spellcheck="false" placeholder="输入或粘贴文字" aria-label="待翻译文字"></textarea><div class="pane-footer"><span id="count">0 / 20,000</span></div></div>
        <div class="editor-pane output-pane"><div class="pane-header"><span>译文</span><span id="result-time"></span></div><div class="output-container"><div class="empty-result" id="empty-result"><i data-lucide="languages"></i><span>译文</span></div><textarea id="output" readonly autocomplete="off" autocorrect="off" autocapitalize="off" spellcheck="false" aria-label="翻译结果" hidden></textarea></div><div class="pane-footer output-tools"><span id="result-label"></span><div><button class="icon-button" id="copy" title="复制译文" aria-label="复制译文" disabled><i data-lucide="copy"></i></button><button class="icon-button" id="save" title="保存为文本" aria-label="保存为文本" disabled><i data-lucide="download"></i></button></div></div></div></div>
        <div class="action-row"><span class="keyboard-hint"><kbd>Ctrl</kbd> + <kbd>Enter</kbd> 快速翻译</span><button class="primary" id="translate" disabled><span>翻译文字</span><i data-lucide="arrow-right"></i></button></div>
      </section>
      <section id="file-view" class="view" hidden><div class="document-card"><div class="file-symbol"><i data-lucide="file-text"></i></div><h2 id="file-title">选择文档</h2><p id="file-description">译文另存为新文件</p><button class="secondary" id="pick-file"><i data-lucide="folder-open"></i>选择文档</button><div class="format-tags"><span>TXT</span><span>DOCX</span><span>PDF</span><span>SRT</span><span>等 9 种格式</span></div><div class="file-detail">最大 50 MB · PDF 需含文字层 · 译后请检查排版</div></div><div class="action-row"><span class="file-note">原文件保持不变</span><button class="primary" id="translate-file" disabled><span>翻译文档</span><i data-lucide="arrow-right"></i></button></div></section>
      <section id="models-view" class="view" hidden><div class="models-header"><h2>已安装</h2><span class="model-count" id="model-count">加载中</span></div><div id="model-list" class="model-list"></div><div class="model-note"><i data-lucide="folder-open"></i><div><strong>模型目录</strong><p id="model-path">runtime/packages</p><span>手动添加 Argos 模型后，重新检查引擎。</span></div></div></section>
      <div id="message" role="status" aria-live="polite" hidden></div>
    </div>
  </main>
  <dialog id="about-dialog"><button class="icon-button close-dialog" id="close-about" aria-label="关闭"><i data-lucide="x"></i></button><span class="brand-icon"><i data-lucide="languages"></i></span><h2>静译 <small>0.2.0</small></h2><div class="about-details">Tauri · Argos Translate · CTranslate2<br>CPU 本地翻译<br>第三方组件说明见 THIRD_PARTY_NOTICES.md。</div><button class="secondary" id="retry">重新检查引擎</button></dialog>
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
      FileText,
      Type,
      HardDrive,
      Copy,
      Download,
      Trash2,
      ArrowRight,
      CircleHelp,
      FolderOpen,
      X,
      LoaderCircle,
      Sun,
      Moon,
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
  for (const id of ["translate", "translate-file", "swap", "pick-file"])
    $<HTMLButtonElement>(id).disabled = busy || !ready;
  $<HTMLButtonElement>("translate").disabled ||= !input.value.trim();
  $<HTMLButtonElement>("clear").disabled =
    busy || (!input.value && !output.value);
  $<HTMLButtonElement>("translate-file").disabled ||= !documentPath;
  $<HTMLButtonElement>("swap").disabled ||= !info?.languages
    .find((language) => language.code === target.value)
    ?.targets.includes(source.value);
  source.disabled = target.disabled = busy || !ready;
  $<HTMLButtonElement>("copy").disabled = !output.value || busy;
  $<HTMLButtonElement>("save").disabled = !output.value || busy;
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
  $("translate-file").innerHTML = busy
    ? '<i data-lucide="loader-circle" class="spin"></i><span>正在处理</span>'
    : '<span>翻译文档</span><i data-lucide="arrow-right"></i>';
  icons();
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
  for (const key of ["text", "file", "models"])
    $(`${key}-view`).hidden = key !== next;
  document
    .querySelectorAll<HTMLButtonElement>("[data-view]")
    .forEach((b) => b.classList.toggle("active", b.dataset.view === next));
  $("language-toolbar").hidden = next === "models";
  $("heading").textContent =
    next === "file" ? "文档翻译" : next === "models" ? "本地模型" : "文字翻译";
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
    if (!info.languages.length || !info.packages.length)
      throw new Error("未找到本地语言包，请检查 runtime/packages。");
    fillSelect(
      source,
      info.languages.filter((l) => l.targets.length),
      source.value,
    );
    fillTargets(target.value);
    $("engine-label").textContent = "离线引擎已就绪";
    $("model-count").textContent = `${info.packages.length} 个语言模型`;
    $("model-list").replaceChildren(
      ...info.packages.map((p) => {
        const card = document.createElement("div");
        card.className = "model-card";
        const badge = document.createElement("span");
        badge.className = "model-badge";
        badge.textContent = p.from.toUpperCase();
        const title = document.createElement("h3");
        title.textContent = `${name(p.from)} → ${name(p.to)}`;
        const version = document.createElement("p");
        version.textContent = `Argos 模型 v${p.version} · CPU`;
        const details = document.createElement("div");
        details.append(title, version);
        const status = document.createElement("span");
        status.className = "installed";
        status.textContent = "已安装";
        card.append(badge, details, status);
        return card;
      }),
    );
    const root = await invoke<string>("runtime_location");
    $("model-path").textContent = `${root}\\runtime\\packages`;
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
  if (busy || !ready || !input.value.trim()) return;
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
$("save").onclick = async () => {
  try {
    if (await invoke<boolean>("save_text", { text: output.value }))
      message("译文已保存");
  } catch (e) {
    message(String(e), true);
  }
};
$("pick-file").onclick = async () => {
  try {
    const path = await invoke<string | null>("pick_document");
    if (path) {
      documentPath = path;
      $("file-title").textContent = path.split(/[\\/]/).pop()!;
      $("file-description").textContent = path;
      update();
      message("");
    }
  } catch (e) {
    message(String(e), true);
  }
};
$("translate-file").onclick = async () => {
  if (busy || !documentPath || !ready) return;
  busy = true;
  message("");
  update();
  try {
    const reply = await invoke<Reply<{ path: string }> | null>(
      "translate_document",
      { path: documentPath, source: source.value, target: target.value },
    );
    if (reply)
      message(
        `翻译完成，用时 ${reply.elapsed.toFixed(1)} 秒。已保存：${reply.data.path}`,
      );
  } catch (e) {
    message(String(e), true);
  } finally {
    busy = false;
    update();
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
$("retry").onclick = () => {
  if (!busy) {
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
  input.value =
    output.value =
    documentPath =
    translatedSource =
    translatedDirection =
      "";
  $("file-title").textContent = "选择文档";
  $("file-description").textContent = "译文另存为新文件";
  message("");
});
icons();
void applyTheme(theme);
void load();
