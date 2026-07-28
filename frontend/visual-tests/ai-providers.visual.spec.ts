import { expect, test, type Locator, type Page } from "@playwright/test";

async function openScenario(page: Page, scenario: "empty" | "detail") {
  await page.goto(`/?scenario=${scenario}`);
  await expect(page.locator(".ai-provider-page")).toBeVisible();
}

async function visibleCount(locator: Locator) {
  return locator.evaluateAll((elements) =>
    elements.filter((element) => {
      const rect = element.getBoundingClientRect();
      const style = getComputedStyle(element);
      return (
        rect.width > 0 &&
        rect.height > 0 &&
        style.display !== "none" &&
        style.visibility !== "hidden"
      );
    }).length,
  );
}

async function expectNoHorizontalOverflow(page: Page) {
  const overflow = await page.evaluate(() => ({
    document: document.documentElement.scrollWidth - document.documentElement.clientWidth,
    harness: (() => {
      const harness = document.querySelector<HTMLElement>(".visual-harness");
      return harness ? harness.scrollWidth - harness.clientWidth : 0;
    })(),
  }));
  expect(overflow.document).toBeLessThanOrEqual(1);
  expect(overflow.harness).toBeLessThanOrEqual(1);
}

async function expectButtonsInsideSurfaces(page: Page) {
  const failures = await page.locator(".app-button:visible").evaluateAll((buttons) =>
    buttons.flatMap((button) => {
      const parent = button.closest<HTMLElement>(".panel, .provider-empty-surface");
      if (!parent) return [];
      const buttonBox = button.getBoundingClientRect();
      const parentBox = parent.getBoundingClientRect();
      const tolerance = 1;
      const inside =
        buttonBox.left >= parentBox.left - tolerance &&
        buttonBox.right <= parentBox.right + tolerance &&
        buttonBox.top >= parentBox.top - tolerance &&
        buttonBox.bottom <= parentBox.bottom + tolerance;
      return inside
        ? []
        : [{
          text: button.textContent?.trim() ?? "",
          button: {
            left: buttonBox.left,
            right: buttonBox.right,
            top: buttonBox.top,
            bottom: buttonBox.bottom,
          },
          parent: {
            left: parentBox.left,
            right: parentBox.right,
            top: parentBox.top,
            bottom: parentBox.bottom,
          },
        }];
    }),
  );
  expect(failures).toEqual([]);
}

async function expectOnePrimaryAction(page: Page) {
  await expect.poll(() => visibleCount(page.locator(".app-button--primary"))).toBe(1);
}

async function expectUniformControlHeights(page: Page) {
  const heights = await page
    .locator(
      ".provider-create-form input.form-control:not([type=checkbox]):not([type=radio]), " +
      ".provider-create-form select.form-control",
    )
    .evaluateAll((controls) =>
      controls
        .filter((control) => {
          const box = control.getBoundingClientRect();
          return box.width > 0 && box.height > 0;
        })
        .map((control) => control.getBoundingClientRect().height),
    );
  expect(heights.length).toBeGreaterThan(1);
  expect(Math.max(...heights) - Math.min(...heights)).toBeLessThanOrEqual(1);
}

async function expectCheckboxGeometry(page: Page) {
  const checkbox = page.locator(".checkbox-field input[type=checkbox]");
  const label = page.locator(".checkbox-field__label");
  await expect(checkbox).toBeVisible();
  await expect(label).toBeVisible();
  const [checkboxBox, labelBox] = await Promise.all([
    checkbox.boundingBox(),
    label.boundingBox(),
  ]);
  expect(checkboxBox).not.toBeNull();
  expect(labelBox).not.toBeNull();
  expect(checkboxBox!.width).toBeCloseTo(18, 0);
  expect(checkboxBox!.height).toBeCloseTo(18, 0);
  expect(Math.abs(checkboxBox!.y - labelBox!.y)).toBeLessThanOrEqual(2);
}

async function expectSharedGeometry(page: Page) {
  await expectNoHorizontalOverflow(page);
  await expectButtonsInsideSurfaces(page);
  await expectOnePrimaryAction(page);
}

async function prepareFullVisualCapture(page: Page) {
  await page.evaluate(() => {
    document.documentElement.classList.add("visual-capture");
    document
      .querySelector(".visual-harness")
      ?.classList.add("visual-harness--capture");
  });
}

test("AI service workbench passes the nine viewport and scale geometry checks", async ({ page }) => {
  await openScenario(page, "empty");
  await expectSharedGeometry(page);
  await expect(page.locator(".provider-detail-empty")).toHaveCount(0);

  await page.getByRole("button", { name: "创建 AI 服务" }).click();
  await expect(page.locator(".provider-create-form")).toBeVisible();
  await expectSharedGeometry(page);
  await expectUniformControlHeights(page);
  await expectCheckboxGeometry(page);

  await page.locator("#create-provider-preset").selectOption({ label: "DeepSeek" });
  await expect(page.locator("#create-provider-environment-variable")).toBeVisible();
  await expect(page.locator(".checkbox-field")).toHaveCount(0);
  await expectSharedGeometry(page);
  await expectUniformControlHeights(page);

  await page.locator("#create-provider-preset").selectOption({ label: "Ollama" });
  await expect(
    page.locator(".provider-create-form .notice__title").filter({ hasText: "无需密钥" }),
  ).toBeVisible();
  await expect(page.locator("#create-provider-api-key")).toHaveCount(0);
  await expect(page.locator("#create-provider-environment-variable")).toHaveCount(0);
  await expectSharedGeometry(page);
  await expectUniformControlHeights(page);

  await openScenario(page, "detail");
  await expect(page.locator(".provider-list-panel")).toBeVisible();
  await expect(page.locator(".provider-editor")).toBeVisible();
  await expectSharedGeometry(page);
});

test("provider list and detail become a single column below 900px", async ({ page }, testInfo) => {
  test.skip(testInfo.project.name !== "960x720-scale-100");
  await page.setViewportSize({ width: 860, height: 720 });
  await openScenario(page, "detail");
  const [listBox, detailBox] = await Promise.all([
    page.locator(".provider-list-panel").boundingBox(),
    page.locator(".provider-detail-stack").boundingBox(),
  ]);
  expect(listBox).not.toBeNull();
  expect(detailBox).not.toBeNull();
  expect(detailBox!.y).toBeGreaterThanOrEqual(listBox!.y + listBox!.height - 1);
  expect(Math.abs(detailBox!.x - listBox!.x)).toBeLessThanOrEqual(1);
  await expectSharedGeometry(page);
});

test("custom provider creation visual baseline", async ({ page }, testInfo) => {
  test.skip(testInfo.project.name !== "1280x820-scale-125");
  await openScenario(page, "empty");
  await page.getByRole("button", { name: "创建 AI 服务" }).click();
  await expect(page.locator(".provider-create-form")).toBeVisible();
  await prepareFullVisualCapture(page);
  await expect(page.locator(".provider-create-form")).toHaveScreenshot(
    "ai-provider-custom-create.png",
  );
});

test("configured provider detail visual baseline", async ({ page }, testInfo) => {
  test.skip(testInfo.project.name !== "1280x820-scale-125");
  await openScenario(page, "detail");
  await expect(page.locator(".provider-editor")).toBeVisible();
  await prepareFullVisualCapture(page);
  await expect(page.locator(".provider-workbench")).toHaveScreenshot(
    "ai-provider-configured-detail.png",
  );
});
