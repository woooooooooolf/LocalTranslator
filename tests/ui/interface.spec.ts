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
      let japanese = false;
      state.__TAURI_INTERNALS__ = {
        invoke: async (command: string, args: Record<string, unknown>) => {
          state.testCalls.push({ command, args });
          if (command === "engine_info") {
            const packages = [
              ...(installed
                ? [
                    { from: "zh", to: "en", version: "1.9" },
                    { from: "en", to: "zh", version: "1.9" },
                  ]
                : []),
              ...(japanese
                ? [
                    { from: "ja", to: "zh", version: "m2m100" },
                    { from: "zh", to: "ja", version: "m2m100" },
                  ]
                : []),
            ];
            const codes = [...new Set(packages.flatMap((p) => [p.from, p.to]))];
            return {
              data: {
                languages: codes.map((code) => ({
                  code,
                  name: code,
                  targets: packages
                    .filter((p) => p.from === code)
                    .map((p) => p.to),
                })),
                packages,
              },
              elapsed: 0,
            };
          }
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
              {
                id: "ja-zh",
                source: "ja",
                target: "zh",
                version: "m2m100",
                name: "M2M100 INT8",
                bidirectional: true,
                bytes: 500000000,
                present: japanese,
                installed: japanese,
              },
            ];
          if (command === "translate_text")
            return {
              data: {
                text:
                  args.target === "ja"
                    ? "こんにちは。"
                    : args.source === "ja"
                      ? "我一直喜欢你。"
                      : args.source === "zh"
                        ? "Hello, world!"
                        : "哈罗,世界!",
              },
              elapsed: 0.4,
            };
          if (command === "install_model") {
            if (args.id === "ja-zh") {
              japanese = true;
              return null;
            }
            throw "下载中断，请检查网络后重试。";
          }
          if (command === "remove_model" && args.id === "ja-zh") {
            japanese = false;
            return null;
          }
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
  await expect(page.locator("#source")).toHaveValue("en");
  await expect(page.locator("#target")).toHaveValue("zh");
  await page.locator("#input").fill("Hello, world!");
  await page.locator("#translate").click();
  await expect(page.locator("#output")).toHaveValue("哈罗,世界!");
  await page.locator("#locale-toggle").click();
  await expect(page.locator("html")).toHaveAttribute("lang", "en");
  await expect(page.locator("#source")).toHaveValue("zh");
  await expect(page.locator("#target")).toHaveValue("en");
  await expect(page.locator("#heading")).toHaveText("Translate");
  await expect(page.locator("#input")).toHaveValue("Hello, world!");
  await expect(page.locator("#output")).toHaveValue("哈罗,世界!");
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
  await expect(page.locator("#source")).toHaveValue("en");
  await expect(page.locator("#target")).toHaveValue("zh");
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
  await expect(page.locator("#source")).toHaveValue("en");
  await expect(page.locator("#target")).toHaveValue("zh");
});

test("English model screen, confirmation and native errors are localized", async ({
  page,
}) => {
  await setup(page);
  await page.locator("#locale-toggle").click();
  await page.locator('[data-view="models"]').click();
  await expect(page.locator("#heading")).toHaveText("Models");
  await expect(page.locator("#model-count")).toHaveText("2 / 3 installed");
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
  await expect(page.locator("#model-count")).toHaveText("2 / 3 已安装");
});

test("missing-model state retains requested direction and download errors use English", async ({
  page,
}) => {
  await setup(page, false);
  await page.locator("#locale-toggle").click();
  await expect(page.locator("#source")).toHaveValue("zh");
  await expect(page.locator("#target")).toHaveValue("en");
  await expect(page.locator("#translate")).toBeDisabled();
  await page
    .getByRole("button", {
      name: "Download Chinese (Simplified) → English model",
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
          (p: any) => p.from === "zh",
        );
        result.data.languages = [
          { code: "zh", name: "Chinese", targets: ["en"] },
          { code: "en", name: "English", targets: [] },
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
  await expect(page.locator("#source")).toHaveValue("zh");
  await expect(page.locator("#target")).toHaveValue("en");
  await expect(page.locator("#message")).toBeHidden();
  await expect(page.locator("#translate")).toBeEnabled();
});

test("one shared Japanese model enables both directions and is removed once", async ({
  page,
}) => {
  await setup(page);
  await page.locator("#source").selectOption("ja");
  await expect(page.locator("#target")).toHaveValue("zh");
  await expect(page.locator("#translate")).toBeDisabled();
  await page.locator('[data-view="models"]').click();
  await page
    .getByRole("button", { name: "下载日语 ↔ 简体中文模型", exact: true })
    .click();
  await expect(page.locator("#model-count")).toHaveText("3 / 3 已安装");
  await page.locator('[data-view="text"]').click();
  await page.locator("#input").fill("お前のこと、ずっと好きだった。");
  await page.locator("#translate").click();
  await expect(page.locator("#output")).toHaveValue("我一直喜欢你。");
  await page.locator("#swap").click();
  await expect(page.locator("#source")).toHaveValue("zh");
  await expect(page.locator("#target")).toHaveValue("ja");
  await page.locator("#input").fill("你好。");
  await page.locator("#translate").click();
  await expect(page.locator("#output")).toHaveValue("こんにちは。");
  await page.locator('[data-view="models"]').click();
  await page
    .getByRole("button", { name: "删除日语 ↔ 简体中文模型", exact: true })
    .click();
  await page.locator("#confirm-delete").click();
  await expect(page.locator("#model-count")).toHaveText("2 / 3 已安装");
  const calls = await page.evaluate(() => (window as any).testCalls);
  expect(
    calls.filter(
      (c: any) => c.command === "install_model" && c.args.id === "ja-zh",
    ),
  ).toHaveLength(1);
  expect(
    calls.filter(
      (c: any) => c.command === "remove_model" && c.args.id === "ja-zh",
    ),
  ).toHaveLength(1);
  expect(
    calls
      .filter((c: any) => c.command === "translate_text")
      .map((c: any) => [c.args.source, c.args.target]),
  ).toEqual([
    ["ja", "zh"],
    ["zh", "ja"],
  ]);
  await expect(page.locator('input[type="file"]')).toHaveCount(0);
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
  await page.locator("#input").fill("Hello, world!");
  await page.locator("#translate").click();
  await expect(page.locator("#output")).toHaveValue("哈罗,世界!");
  const directory = path.resolve("docs/images");
  fs.mkdirSync(directory, { recursive: true });
  await page.screenshot({ path: path.join(directory, "screenshot-zh.png") });
  await page.locator("#locale-toggle").click();
  await page.locator("#input").fill("你好，世界！");
  await page.locator("#translate").click();
  await expect(page.locator("#output")).toHaveValue("Hello, world!");
  await page.screenshot({ path: path.join(directory, "screenshot-en.png") });
});
