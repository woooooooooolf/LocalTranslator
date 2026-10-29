import { test, expect, type Page } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";

async function setup(page: Page, installed = true) {
  await page.addInitScript(
    ({ installed }) => {
      const state = window as unknown as {
        __TAURI_INTERNALS__: unknown;
        testCalls: { command: string; args: Record<string, unknown> }[];
      };
      state.testCalls = [];
      state.__TAURI_INTERNALS__ = {
        invoke: async (command: string, args: Record<string, unknown>) => {
          state.testCalls.push({ command, args });
          if (command === "engine_info")
            return {
              data: {
                languages: installed
                  ? [
                      { code: "zh", name: "Chinese", targets: ["en"] },
                      { code: "en", name: "English", targets: ["zh"] },
                    ]
                  : [],
                packages: installed
                  ? [
                      { from: "zh", to: "en", version: "1.9" },
                      { from: "en", to: "zh", version: "1.9" },
                    ]
                  : [],
              },
              elapsed: 0,
            };
          if (command === "runtime_location") return "C:\\LocalTranslator";
          if (command === "model_catalog")
            return [
              {
                id: "en-zh",
                source: "en",
                target: "zh",
                version: "1.9",
                bytes: 70931064,
                present: installed,
                installed,
              },
              {
                id: "zh-en",
                source: "zh",
                target: "en",
                version: "1.9",
                bytes: 75097844,
                present: installed,
                installed,
              },
            ];
          if (command === "translate_text")
            return {
              data: {
                text: args.source === "zh" ? "Hello, world!" : "哈罗,世界!",
              },
              elapsed: 0.4,
            };
          if (command === "install_model") throw "下载中断，请检查网络后重试。";
          if (command === "model_progress") return { active: false };
          return null;
        },
      };
    },
    { installed },
  );
  await page.goto("/");
  await expect(page.locator("#engine-label")).toHaveText(
    installed ? "离线引擎已就绪" : "未安装模型",
  );
}

test("defaults to Chinese and switches direction without rewriting translation content", async ({
  page,
}) => {
  await setup(page);
  await expect(page.locator("html")).toHaveAttribute("lang", "zh-CN");
  await expect(page.locator("#source")).toHaveValue("zh");
  await expect(page.locator("#target")).toHaveValue("en");
  await page.locator("#input").fill("你好，世界！");
  await page.locator("#translate").click();
  await expect(page.locator("#output")).toHaveValue("Hello, world!");
  await page.locator("#locale-toggle").click();
  await expect(page.locator("html")).toHaveAttribute("lang", "en");
  await expect(page.locator("#source")).toHaveValue("en");
  await expect(page.locator("#target")).toHaveValue("zh");
  await expect(page.locator("#heading")).toHaveText("Translate");
  await expect(page.locator("#input")).toHaveValue("你好，世界！");
  await expect(page.locator("#output")).toHaveValue("Hello, world!");
  await expect(page.locator("#result-label")).toHaveText(
    "Text or language changed. Translate again.",
  );
  await expect(page.locator("#input")).toHaveAttribute(
    "placeholder",
    "Type or paste text",
  );
  await page.locator("#theme-toggle").click();
  await expect(page.locator("#theme-toggle")).toHaveAttribute(
    "aria-label",
    "Switch to light theme",
  );
  await page.locator("#locale-toggle").click();
  await expect(page.locator("#source")).toHaveValue("zh");
  await expect(page.locator("#target")).toHaveValue("en");
  await expect(page.locator("#heading")).toHaveText("文字翻译");
  await expect(page.locator("#input")).toHaveAttribute(
    "placeholder",
    "输入或粘贴文字",
  );
  const state = await page.evaluate(async () => ({
    local: localStorage.length,
    session: sessionStorage.length,
    databases: (await indexedDB.databases()).length,
    cookie: document.cookie,
    commands: (window as any).testCalls.map((call: any) => call.command),
  }));
  expect(state.local).toBe(0);
  expect(state.session).toBe(0);
  expect(state.databases).toBe(0);
  expect(state.cookie).toBe("");
  expect(
    state.commands.filter((command: string) => command === "translate_text"),
  ).toHaveLength(1);
  expect(state.commands).not.toContain("install_model");
  await page.reload();
  await expect(page.locator("html")).toHaveAttribute("lang", "zh-CN");
  await expect(page.locator("#input")).toHaveValue("");
});

test("English model screen, confirmation and native errors are localized", async ({
  page,
}) => {
  await setup(page);
  await page.locator("#locale-toggle").click();
  await page.locator('[data-view="models"]').click();
  await expect(page.locator("#heading")).toHaveText("Models");
  await expect(page.locator("#model-count")).toHaveText("2 / 2 installed");
  await page
    .getByRole("button", {
      name: "Delete English → Chinese (Simplified) model",
      exact: true,
    })
    .click();
  await expect(page.locator("#delete-dialog")).toBeVisible();
  await expect(page.locator("#delete-description")).toContainText(
    "Delete the English → Chinese (Simplified) model?",
  );
  await page.locator("#cancel-delete").click();
  expect(
    await page.evaluate(() =>
      (window as any).testCalls.some(
        (call: any) => call.command === "remove_model",
      ),
    ),
  ).toBe(false);
  await page.locator("#locale-toggle").click();
  await expect(page.locator("#model-count")).toHaveText("2 / 2 已安装");
});

test("missing-model state retains requested direction and download errors use English", async ({
  page,
}) => {
  await setup(page, false);
  await page.locator("#locale-toggle").click();
  await expect(page.locator("#source")).toHaveValue("en");
  await expect(page.locator("#target")).toHaveValue("zh");
  await expect(page.locator("#translate")).toBeDisabled();
  await page
    .getByRole("button", {
      name: "Download English → Chinese (Simplified) model",
      exact: true,
    })
    .click();
  await expect(page.locator("#message")).toHaveText(
    "Download interrupted. Check your connection and try again.",
  );
  await expect(page.locator("#download-progress")).toBeHidden();
});

test("an unavailable interface direction can be swapped back to an installed model", async ({
  page,
}) => {
  await setup(page);
  await page.evaluate(() => {
    const state = window as any;
    const invoke = state.__TAURI_INTERNALS__.invoke;
    state.__TAURI_INTERNALS__.invoke = async (command: string, args: any) => {
      const result = await invoke(command, args);
      if (command === "engine_info") {
        result.data.packages = result.data.packages.filter(
          (p: any) => p.from === "en",
        );
        result.data.languages = [
          { code: "en", name: "English", targets: ["zh"] },
          { code: "zh", name: "Chinese", targets: [] },
        ];
      }
      return result;
    };
  });
  await page.locator("#about").click();
  await page.locator("#retry").click();
  await expect(page.locator("#message")).toContainText("请先下载");
  await page.locator("#input").fill("Hello");
  await expect(page.locator("#translate")).toBeDisabled();
  await page.locator("#swap").click();
  await expect(page.locator("#source")).toHaveValue("en");
  await expect(page.locator("#target")).toHaveValue("zh");
  await expect(page.locator("#message")).toBeHidden();
  await expect(page.locator("#translate")).toBeEnabled();
});

test("editors fill the window and context menus remain limited to text", async ({
  page,
}) => {
  await setup(page);
  for (const viewport of [
    { width: 900, height: 650 },
    { width: 1600, height: 1000 },
  ]) {
    await page.setViewportSize(viewport);
    const layout = await page.evaluate(() => ({
      height: document.querySelector("#input")!.getBoundingClientRect().height,
      overflow:
        document.documentElement.scrollHeight > innerHeight ||
        document.documentElement.scrollWidth > innerWidth,
    }));
    expect(layout.height).toBeGreaterThan(viewport.height - 400);
    expect(layout.overflow).toBe(false);
  }
  expect(
    await page
      .locator("#input")
      .evaluate((element) =>
        element.dispatchEvent(
          new MouseEvent("contextmenu", { bubbles: true, cancelable: true }),
        ),
      ),
  ).toBe(true);
  expect(
    await page
      .locator("#heading")
      .evaluate((element) =>
        element.dispatchEvent(
          new MouseEvent("contextmenu", { bubbles: true, cancelable: true }),
        ),
      ),
  ).toBe(false);
});

test("capture bilingual interface previews with explicit synthetic samples", async ({
  page,
}) => {
  test.skip(
    process.env.CAPTURE_SCREENSHOTS !== "1",
    "Documentation capture is opt-in",
  );
  await setup(page);
  await page.locator("#input").fill("你好，世界！");
  await page.locator("#translate").click();
  await expect(page.locator("#output")).toHaveValue("Hello, world!");
  const directory = path.resolve("docs/images");
  fs.mkdirSync(directory, { recursive: true });
  await page.screenshot({ path: path.join(directory, "screenshot-zh.png") });
  await page.locator("#locale-toggle").click();
  await page.locator("#input").fill("Hello, world!");
  await page.locator("#translate").click();
  await expect(page.locator("#output")).toHaveValue("哈罗,世界!");
  await page.screenshot({ path: path.join(directory, "screenshot-en.png") });
});
