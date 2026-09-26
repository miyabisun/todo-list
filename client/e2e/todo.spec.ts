import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { expect, test, type Page } from "@playwright/test";

// Fictional items only. Set E2E_EVIDENCE_DIR to keep screenshots.
const evidence = process.env.E2E_EVIDENCE_DIR;
if (evidence) mkdirSync(evidence, { recursive: true });

async function shoot(page: Page, name: string) {
  if (evidence) {
    await page.screenshot({
      path: join(evidence, `${name}.png`),
      fullPage: true,
    });
  }
}

function isoDay(offset: number): string {
  const day = new Date();
  day.setDate(day.getDate() + offset);
  const pad = (value: number) => String(value).padStart(2, "0");
  return `${day.getFullYear()}-${pad(day.getMonth() + 1)}-${pad(day.getDate())}`;
}

test.beforeAll(async ({ request }) => {
  const seed = [
    {
      title: "架空の申込書を提出する",
      url: "https://example.com/apply",
      due: isoDay(-2),
      source: "example-agent",
      body: "控えを保存する\n2行目のメモ",
    },
    {
      title: "架空の予約を確認する",
      url: "https://example.com/reserve",
      due: isoDay(0),
      source: "example-app",
    },
    {
      title: "架空の請求書を払う",
      url: "https://example.com/pay",
      due: isoDay(1),
    },
    { title: "L".repeat(120), due: isoDay(30) },
    ...Array.from({ length: 10 }, (_, i) => ({
      title: `架空の用事 ${i + 1}`,
      url: i % 2 ? `https://example.com/item/${i}` : undefined,
      source: i % 3 ? undefined : "example-agent",
    })),
  ];
  for (const data of seed) {
    const response = await request.post("/api/todos", { data });
    expect(response.status()).toBe(201);
  }
  const done = await request.post("/api/todos", {
    data: { title: "架空の完了済み", done: true },
  });
  expect(done.status()).toBe(201);
});

for (const viewport of [
  { width: 375, height: 812 },
  { width: 320, height: 640 },
  { width: 1280, height: 720 },
]) {
  for (const colorScheme of ["dark", "light"] as const) {
    test(`${viewport.width}x${viewport.height} ${colorScheme}: list flows with the page`, async ({
      page,
    }) => {
      await page.setViewportSize(viewport);
      await page.emulateMedia({ colorScheme });
      await page.goto("/");
      await expect(page.locator("[data-state=success]")).toBeVisible();
      const rows = page.locator("[data-state] > .todos > li");
      await expect(rows.first()).toBeVisible();
      const geometry = await page.evaluate(() => {
        const cards = [
          ...document.querySelectorAll("[data-state] > .todos > li"),
        ].map((li) => li.getBoundingClientRect());
        return {
          firstTop: cards[0].top,
          visible: cards.filter((r) => r.top >= 48 && r.bottom <= innerHeight)
            .length,
          documentWidth: document.documentElement.scrollWidth,
          background: getComputedStyle(document.body).backgroundColor,
        };
      });
      expect(geometry.documentWidth).toBe(viewport.width);
      expect(geometry.background).toBe(
        colorScheme === "dark" ? "rgb(25, 25, 25)" : "rgb(250, 246, 239)",
      );
      expect(geometry.firstTop).toBeLessThanOrEqual(200);
      expect(geometry.visible).toBeGreaterThanOrEqual(
        viewport.height >= 720 ? 6 : 4,
      );
      await expect(page.getByText(/^期限切れ /)).toHaveCSS(
        "color",
        colorScheme === "dark" ? "rgb(255, 107, 107)" : "rgb(156, 43, 29)",
      );
      await shoot(page, `list-${viewport.width}-${colorScheme}`);
      if (evidence) {
        writeFileSync(
          join(evidence, `list-${viewport.width}-${colorScheme}.json`),
          JSON.stringify(geometry),
        );
      }
    });
  }
}

test("phone: add with a link, open it, complete, then reopen from the done list", async ({
  page,
  context,
  request,
}) => {
  await page.setViewportSize({ width: 375, height: 812 });
  await context.route("https://example.com/**", (route) =>
    route.fulfill({ contentType: "text/html", body: "<title>stub</title>" }),
  );
  await page.goto("/");
  await expect(page.locator("[data-state=success]")).toBeVisible();

  await page
    .getByRole("textbox", { name: "新しい ToDo" })
    .fill("架空の書類を受け取る");
  await page.getByText("リンク・期日・メモ").click();
  await page.getByLabel("リンク").fill("https://example.com/pickup");
  await page.getByLabel("期日").fill(isoDay(3));
  await shoot(page, "add-form-375");
  await page.getByRole("button", { name: "追加" }).click();

  const link = page.getByRole("link", { name: "架空の書類を受け取る" });
  await expect(link).toHaveAttribute("href", "https://example.com/pickup");
  await expect(link).toHaveAttribute("target", "_blank");
  await expect(page.getByRole("textbox", { name: "新しい ToDo" })).toHaveValue(
    "",
  );
  const box = await link.boundingBox();
  expect(box!.width).toBeGreaterThan(200);
  expect(box!.height).toBeGreaterThanOrEqual(24);

  const [popup] = await Promise.all([
    context.waitForEvent("page"),
    link.click(),
  ]);
  await expect(popup).toHaveURL("https://example.com/pickup");
  await popup.close();

  const checkbox = page.getByRole("checkbox", {
    name: "完了: 架空の書類を受け取る",
  });
  await checkbox.check();
  const row = page.locator("li", { has: checkbox });
  await expect(row).toHaveAttribute("data-done", "true");
  await shoot(page, "completed-in-place-375");
  const done = await (await request.get("/api/todos?done=true")).json();
  expect(done.map((t: { title: string }) => t.title)).toContain(
    "架空の書類を受け取る",
  );

  await page.reload();
  await expect(
    page.getByRole("link", { name: "架空の書類を受け取る" }),
  ).toBeHidden();
  await page.getByText(/^完了済み \d+件$/).click();
  const again = page.getByRole("checkbox", {
    name: "完了: 架空の書類を受け取る",
  });
  await expect(again).toBeChecked();
  await shoot(page, "done-list-375");
  await again.uncheck();
  await expect(page.locator("li", { has: again })).toHaveAttribute(
    "data-done",
    "false",
  );
  const open = await (await request.get("/api/todos?done=false")).json();
  expect(open.map((t: { title: string }) => t.title)).toContain(
    "架空の書類を受け取る",
  );
});

test("keyboard adds an item and focus stays visible", async ({ page }) => {
  await page.setViewportSize({ width: 375, height: 812 });
  await page.goto("/");
  const input = page.getByRole("textbox", { name: "新しい ToDo" });
  await input.focus();
  await page.keyboard.type("架空の電話をかける");
  await page.keyboard.press("Tab");
  const add = page.getByRole("button", { name: "追加" });
  await expect(add).toBeFocused();
  await expect(add).toHaveCSS("outline-style", "solid");
  await input.focus();
  await page.keyboard.press("Enter");
  await expect(page.getByText("架空の電話をかける")).toBeVisible();
  await expect(input).toHaveValue("");
});

test("a rejected link keeps the typed text and explains why", async ({
  page,
}) => {
  await page.setViewportSize({ width: 320, height: 640 });
  await page.goto("/");
  await page
    .getByRole("textbox", { name: "新しい ToDo" })
    .fill("架空のリンク違い");
  await page.getByText("リンク・期日・メモ").click();
  await page.getByLabel("リンク").fill("ftp://example.com/file");
  await page.getByRole("button", { name: "追加" }).click();
  // The browser blocks the submit first; the item is not added.
  expect(
    await page
      .getByLabel("リンク")
      .evaluate((el: HTMLInputElement) => el.validity.valid),
  ).toBe(false);
  await expect(page.getByText("架空のリンク違い", { exact: true })).toHaveCount(
    0,
  );
  await expect(page.getByRole("textbox", { name: "新しい ToDo" })).toHaveValue(
    "架空のリンク違い",
  );
});

test("loading, empty and error states recover", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 640 });
  let mode: "loading" | "empty" | "error" | "real" = "loading";
  let release!: () => void;
  const pending = new Promise<void>((resolve) => (release = resolve));
  await page.route("**/api/todos", async (route) => {
    if (route.request().method() !== "GET" || mode === "real")
      return route.fallback();
    if (mode === "loading") await pending;
    await route.fulfill(
      mode === "error"
        ? { status: 500, json: { error: "io: stub", code: "io" } }
        : { json: [] },
    );
  });
  await page.goto("/");
  await expect(page.getByText("読み込み中…")).toBeVisible();
  mode = "empty";
  release();
  await expect(page.getByText("やることはありません")).toBeVisible();
  await shoot(page, "empty-320");
  mode = "error";
  await page.reload();
  await expect(page.getByText("読み込みに失敗しました")).toBeVisible();
  await shoot(page, "error-320");
  mode = "real";
  await page.getByRole("button", { name: "再試行" }).click();
  await expect(page.locator("[data-state=success]")).toBeVisible();
});

test("a failed completion is reverted and explained on its row", async ({
  page,
}) => {
  await page.setViewportSize({ width: 375, height: 812 });
  await page.route("**/api/todos/*", (route) =>
    route.request().method() === "PATCH"
      ? route.fulfill({ status: 500, json: { error: "io: stub", code: "io" } })
      : route.fallback(),
  );
  await page.goto("/");
  const checkbox = page.getByRole("checkbox", {
    name: "完了: 架空の用事 1",
    exact: true,
  });
  await checkbox.click();
  await expect(
    page.locator("li", { has: checkbox }).getByRole("alert"),
  ).toContainText("更新できませんでした");
  await expect(checkbox).not.toBeChecked();
  await shoot(page, "complete-failed-375");
});
