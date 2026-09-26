import { describe, expect, it } from "vitest";

import { dueLabel, localDate, newTodoFields, safeUrl } from "./todo";

describe("dueLabel", () => {
  const today = "2026-09-26";

  it("names today and tomorrow", () => {
    expect(dueLabel("2026-09-26", today)).toEqual({
      text: "今日まで",
      overdue: false,
    });
    expect(dueLabel("2026-09-27", today)).toEqual({
      text: "明日まで",
      overdue: false,
    });
  });

  it("shows month and day this year and the year otherwise", () => {
    expect(dueLabel("2026-10-03", today)).toEqual({
      text: "10/3まで",
      overdue: false,
    });
    expect(dueLabel("2027-01-05", today)).toEqual({
      text: "2027/1/5まで",
      overdue: false,
    });
  });

  it("marks past dates as overdue in words, not only color", () => {
    expect(dueLabel("2026-09-25", today)).toEqual({
      text: "期限切れ 9/25",
      overdue: true,
    });
    expect(dueLabel("2025-12-31", today)).toEqual({
      text: "期限切れ 2025/12/31",
      overdue: true,
    });
  });

  it("counts tomorrow across month and year ends", () => {
    expect(dueLabel("2026-10-01", "2026-09-30").text).toBe("明日まで");
    expect(dueLabel("2027-01-01", "2026-12-31").text).toBe("明日まで");
  });
});

describe("localDate", () => {
  it("formats the local calendar day", () => {
    expect(localDate(new Date(2026, 0, 5, 23, 59))).toBe("2026-01-05");
  });
});

describe("safeUrl", () => {
  it("keeps only http and https links", () => {
    expect(safeUrl("https://example.com/a")).toBe("https://example.com/a");
    expect(safeUrl("HTTP://example.com")).toBe("HTTP://example.com");
    for (const url of [null, "", "javascript:alert(1)", "//example.com"]) {
      expect(safeUrl(url)).toBeNull();
    }
  });
});

describe("newTodoFields", () => {
  it("trims the title and omits blank optional fields", () => {
    expect(
      newTodoFields({ title: "  架空の用事 ", url: " ", due: "", body: "" }),
    ).toEqual({ title: "架空の用事" });
  });

  it("sends filled optional fields", () => {
    expect(
      newTodoFields({
        title: "t",
        url: " https://example.com ",
        due: "2026-10-01",
        body: "memo\n",
      }),
    ).toEqual({
      title: "t",
      url: "https://example.com",
      due: "2026-10-01",
      body: "memo\n",
    });
  });
});
