import { describe, expect, test } from "bun:test";
import {
  createAggregatedLog,
  getAggregatedMessageIds,
  mergeAggregatedLog,
} from "../src/aggregatedLog";

const entry = {
  messageId: "123",
  date: "2026-06-29",
  time: "21:34",
  authorName: "Alice",
  markdown: "yes",
};

describe("mergeAggregatedLog", () => {
  test("creates a daily log without per-message or date headings", () => {
    const result = mergeAggregatedLog(
      createAggregatedLog("daily", "2026-06-29"),
      [entry],
      {
        mode: "daily",
        showAuthorNames: false,
        showMessageTime: false,
      },
    );

    expect(result.addedCount).toBe(1);
    expect(result.content).toBe(
      [
        "<!-- discord-message-sender: daily-log -->",
        "# 2026-06-29",
        "",
        "<!-- discord-message-id: 123 -->",
        "yes",
        "",
      ].join("\n"),
    );
    expect(result.content).not.toContain("## ");
  });

  test("adds one date heading and optional discussion details", () => {
    const result = mergeAggregatedLog(
      createAggregatedLog("monthly", "2026-06"),
      [
        entry,
        {
          ...entry,
          messageId: "124",
          time: "21:40",
          authorName: "Bob",
          markdown: "I agree.",
        },
      ],
      {
        mode: "monthly",
        showAuthorNames: true,
        showMessageTime: true,
      },
    );

    expect(result.addedCount).toBe(2);
    expect(result.content.match(/^## 2026-06-29$/gm)).toHaveLength(1);
    expect(result.content).toContain(
      "<!-- discord-message-id: 123 -->\n**Alice** · 21:34\n\nyes",
    );
    expect(result.content).toContain(
      "<!-- discord-message-id: 124 -->\n**Bob** · 21:40\n\nI agree.",
    );
  });

  test("supports author-only and time-only display", () => {
    const authorOnly = mergeAggregatedLog(
      createAggregatedLog("weekly", "2026-W27"),
      [{ ...entry, authorName: "A*lice" }],
      {
        mode: "weekly",
        showAuthorNames: true,
        showMessageTime: false,
      },
    ).content;
    const timeOnly = mergeAggregatedLog(
      createAggregatedLog("weekly", "2026-W27"),
      [entry],
      {
        mode: "weekly",
        showAuthorNames: false,
        showMessageTime: true,
      },
    ).content;

    expect(authorOnly).toContain("**A\\*lice**\n\nyes");
    expect(timeOnly).toContain("21:34\n\nyes");
  });

  test("does not duplicate message IDs and preserves user edits", () => {
    const existing = [
      "<!-- discord-message-sender: monthly-log -->",
      "# 2026-06",
      "",
      "User note",
      "",
      "## 2026-06-29",
      "",
      "<!-- discord-message-id: 123 -->",
      "edited",
      "",
    ].join("\n");
    const result = mergeAggregatedLog(existing, [entry], {
      mode: "monthly",
      showAuthorNames: false,
      showMessageTime: false,
    });

    expect(result).toEqual({ content: existing, addedCount: 0 });
    expect(getAggregatedMessageIds(result.content)).toEqual(["123"]);
  });

  test("inserts a retried message into its existing date section", () => {
    const existing = [
      "<!-- discord-message-sender: monthly-log -->",
      "# 2026-06",
      "",
      "## 2026-06-29",
      "",
      "User note",
      "",
      "## 2026-06-30",
      "",
      "<!-- discord-message-id: 200 -->",
      "later",
      "",
    ].join("\n");
    const result = mergeAggregatedLog(existing, [entry], {
      mode: "monthly",
      showAuthorNames: false,
      showMessageTime: false,
    });

    expect(result.content.match(/^## 2026-06-29$/gm)).toHaveLength(1);
    expect(result.content.indexOf("User note")).toBeLessThan(
      result.content.indexOf("<!-- discord-message-id: 123 -->"),
    );
    expect(
      result.content.indexOf("<!-- discord-message-id: 123 -->"),
    ).toBeLessThan(result.content.indexOf("## 2026-06-30"));
  });
});
