export type Locale = "zh" | "en";
let locale: Locale = "zh";
export const getLocale = () => locale;
export const setLocale = (next: Locale) => {
  locale = next;
};

const english: Record<string, string> = {
  静译: "LocalTranslator",
  静译首页: "LocalTranslator home",
  "静译 · 离线翻译": "LocalTranslator · Offline translation",
  主导航: "Main navigation",
  文字翻译: "Translate",
  本地模型: "Models",
  深色主题: "Dark theme",
  浅色主题: "Light theme",
  切换到深色主题: "Switch to dark theme",
  切换到浅色主题: "Switch to light theme",
  关于: "About",
  加载中: "Loading",
  源语言: "Source language",
  目标语言: "Target language",
  交换语言: "Swap languages",
  原文: "Source",
  译文: "Translation",
  清空文字: "Clear text",
  输入或粘贴文字: "Type or paste text",
  待翻译文字: "Text to translate",
  翻译结果: "Translated text",
  复制译文: "Copy translation",
  快速翻译: "Translate",
  中英模型: "Chinese / English models",
  刷新: "Refresh",
  准备下载: "Preparing download",
  取消: "Cancel",
  模型下载进度: "Model download progress",
  模型目录: "Model folder",
  "仅点击下载时联网。来源：Argos / MiniSBD 官方源；下载后可断网使用。":
    "Downloads use the official Argos / MiniSBD sources. Translation works offline.",
  关闭: "Close",
  "CPU 本地翻译": "Local CPU translation",
  第三方说明: "Third-party notices",
  重新检查引擎: "Check engine",
  关闭第三方说明: "Close third-party notices",
  第三方说明内容: "Third-party notices",
  删除模型: "Delete model",
  删除: "Delete",
  下载: "Download",
  已安装: "Installed",
  未安装: "Not installed",
  文件不完整: "Incomplete files",
  "窗口主题同步失败。": "Could not update the window theme.",
  "窗口语言同步失败。": "Could not update the window title.",
  "正在本机翻译，请稍候…": "Translating locally…",
  "原文或语言已更改，请重新翻译": "Text or language changed. Translate again.",
  翻译完成: "Translation complete",
  正在翻译: "Translating",
  正在加载引擎: "Loading engine",
  浏览器预览: "Browser preview",
  "这是界面预览。请运行桌面版以使用本地翻译。":
    "Interface preview. Run the desktop app to translate locally.",
  离线引擎已就绪: "Offline engine ready",
  未安装模型: "No model installed",
  "请在“本地模型”中下载需要的翻译方向。":
    "Download a translation direction in Models.",
  引擎需要检查: "Check the engine",
  "{seconds} 秒": "{seconds} s",
  "{installed} / {total} 已安装": "{installed} / {total} installed",
  "v{version} · 下载 {size}": "v{version} · {size} download",
  "{action}{direction}模型": "{action} {direction} model",
  "删除“{direction}”模型？此方向将暂时无法翻译，可随时重新下载。":
    "Delete the {direction} model? This direction will be unavailable until you download it again.",
  模型管理中: "Managing models",
  删除中: "Deleting",
  下载翻译模型: "Downloading translation model",
  下载分句模型: "Downloading sentence model",
  校验与解包: "Verifying and unpacking",
  安装中: "Installing",
  模型已删除: "Model deleted",
  "模型已安装，可离线翻译": "Model installed. Ready to translate offline.",
  "无法取消，请稍后重试。": "Could not cancel. Please try again.",
  译文已复制: "Translation copied",
  "复制失败，请选中译文后按 Ctrl+C。":
    "Could not copy. Select the translation and press Ctrl+C.",
  "第三方说明内置于桌面版。":
    "Third-party notices are included in the desktop app.",
  "无法显示第三方说明。": "Could not display third-party notices.",
  "请先下载“{direction}”模型。": "Download the {direction} model first.",
  "操作失败，请重试或重新检查引擎。":
    "Operation failed. Try again or check the engine.",
  "不支持的模型。": "Unsupported model.",
  "已有模型操作正在进行。": "A model operation is already in progress.",
  "下载已取消。": "Download cancelled.",
  "无法连接模型源，请检查网络后重试。":
    "Could not connect to the model source. Check your connection and try again.",
  "模型文件大小与内置清单不符，已停止下载。":
    "Unexpected model size. Download stopped.",
  "无法创建模型临时文件，请检查空间和写入权限。":
    "Could not create a temporary model file. Check disk space and folder permissions.",
  "下载中断，请检查网络后重试。":
    "Download interrupted. Check your connection and try again.",
  "模型超过预期大小，已停止下载。":
    "Model exceeds the expected size. Download stopped.",
  "模型写入失败，请检查磁盘空间。":
    "Could not write the model. Check disk space.",
  "模型写入失败。": "Could not write the model.",
  "模型 SHA-256 校验失败，未安装。请重新下载。":
    "SHA-256 verification failed. Model not installed. Download it again.",
  "无法创建模型临时目录。": "Could not create a temporary model folder.",
  "无法创建模型临时标记。": "Could not prepare the temporary model folder.",
  "无法准备模型。": "Could not prepare the model.",
  "无法准备分句模型。": "Could not prepare the sentence model.",
  "模型加载验证失败，未安装。":
    "Model loading validation failed. Model not installed.",
  "此模型目录已存在，请先删除后重新下载。":
    "This model folder already exists. Delete the model before downloading it again.",
  "分句模型正在使用，无法替换。":
    "The sentence model is in use and cannot be replaced.",
  "无法安装分句模型。": "Could not install the sentence model.",
  "无法安装模型，请检查目录权限。":
    "Could not install the model. Check folder permissions.",
  "模型已不存在，请刷新列表。": "The model no longer exists. Refresh the list.",
  "无法创建模型清理目录。": "Could not create the model cleanup folder.",
  "无法创建清理标记。": "Could not prepare model cleanup.",
  "模型正在使用或目录不可写，删除失败。":
    "Could not delete the model. It is in use or the folder is not writable.",
  "模型已停用，残留文件将在下次模型操作时清理。":
    "Model disabled. Remaining files will be cleaned up during the next model operation.",
  "无法初始化模型下载。": "Could not initialize the model download.",
  "模型包包含不安全路径、链接或超限内容。":
    "The archive contains unsafe paths, links or oversized content.",
  "模型包缺少必要文件。": "Required model files are missing.",
  "模型内容校验失败，未安装。":
    "Model file verification failed. Model not installed.",
  "模型语言或版本不匹配。": "Model language or version does not match.",
  "模型目录不可写，请将 EXE 放到有写入权限的文件夹。":
    "The model folder is not writable. Move the EXE to a writable folder.",
  "模型正在被其他操作使用，请稍后重试。":
    "Another operation is using the models. Try again later.",
  "模型管理不支持符号链接或目录联接。":
    "Model management does not support symbolic links or directory junctions.",
  "没有安装对应方向的本地模型。":
    "No local model is installed for this direction.",
  "请选择不同的源语言和目标语言。":
    "Choose different source and target languages.",
  "请输入 1 至 20,000 字文字。": "Enter between 1 and 20,000 characters.",
  "无法加载本地翻译模型，请检查模型文件。":
    "Could not load the local translation model. Check the model files.",
  "原生翻译失败，请检查输入及模型兼容性。":
    "Translation failed. Check the input and model compatibility.",
  "模型安装任务异常。": "The model installation task failed.",
  "模型删除任务异常。": "The model deletion task failed.",
  "翻译引擎状态异常。": "The translation engine is unavailable.",
  翻译引擎状态异常: "The translation engine is unavailable.",
};

export function tr(
  text: string,
  values: Record<string, string | number> = {},
): string {
  const template = locale === "en" ? (english[text] ?? text) : text;
  return template.replace(/\{(\w+)\}/g, (_, key: string) =>
    String(values[key] ?? `{${key}}`),
  );
}

export function errorText(error: unknown): string {
  const text = String(error);
  if (locale === "zh") return text;
  if (english[text]) return english[text];
  const http = text.match(/^模型源返回 HTTP (\d+)，请稍后重试。$/);
  if (http)
    return `The model source returned HTTP ${http[1]}. Try again later.`;
  // Unknown native diagnostics are not leaked as untranslated technical strings.
  return /\p{Script=Han}/u.test(text)
    ? tr("操作失败，请重试或重新检查引擎。")
    : text;
}

export const preferredDirection = (value: Locale) =>
  value === "en"
    ? { source: "en", target: "zh" }
    : { source: "zh", target: "en" };
const languageNames: Record<string, [string, string]> = {
  zh: ["简体中文", "Chinese (Simplified)"],
  en: ["英语", "English"],
  ja: ["日语", "Japanese"],
  ko: ["韩语", "Korean"],
  fr: ["法语", "French"],
  de: ["德语", "German"],
  es: ["西班牙语", "Spanish"],
  ru: ["俄语", "Russian"],
  zt: ["繁体中文", "Chinese (Traditional)"],
};
export const languageName = (code: string) =>
  languageNames[code]?.[locale === "en" ? 1 : 0] ?? code;

const originals = new WeakMap<Node, string>();
const originalAttributes = new WeakMap<Element, Map<string, string>>();
// Only fixed UI copy is translated. Never walk user text or model/license text.
export function localizeDOM(root: HTMLElement) {
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
  while (walker.nextNode()) {
    const node = walker.currentNode;
    if (
      node.parentElement?.closest(
        "textarea, input, [contenteditable='true'], #model-path",
      )
    )
      continue;
    const original = originals.get(node) ?? node.textContent ?? "";
    originals.set(node, original);
    const key = original.trim();
    if (english[key]) node.textContent = original.replace(key, tr(key));
  }
  for (const element of Array.from(
    root.querySelectorAll("[aria-label], [title], [placeholder]"),
  )) {
    if (element.id === "locale-toggle" || element.id === "theme-toggle")
      continue;
    const attributes =
      originalAttributes.get(element) ?? new Map<string, string>();
    for (const attribute of ["aria-label", "title", "placeholder"]) {
      const original =
        attributes.get(attribute) ?? element.getAttribute(attribute);
      if (original != null) {
        attributes.set(attribute, original);
        element.setAttribute(attribute, tr(original));
      }
    }
    originalAttributes.set(element, attributes);
  }
}
