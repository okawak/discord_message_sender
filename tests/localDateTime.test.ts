import { describe, expect, test } from "bun:test";
import { toLocalDateTime } from "../src/localDateTime";

describe("toLocalDateTime", () => {
  test("uses the requested time zone at a UTC month boundary", () => {
    const timestamp = "2026-06-30T15:30:45.000Z";

    expect(toLocalDateTime(timestamp, "Asia/Tokyo")).toEqual({
      date: "2026-07-01",
      month: "2026-07",
      week: "2026-W27",
      time: "00:30",
      fileTimestamp: "20260701_003045",
    });
    expect(toLocalDateTime(timestamp, "America/New_York")).toEqual({
      date: "2026-06-30",
      month: "2026-06",
      week: "2026-W27",
      time: "11:30",
      fileTimestamp: "20260630_113045",
    });
  });

  test("uses the ISO week-year at a calendar year boundary", () => {
    expect(
      toLocalDateTime("2020-12-31T15:30:00.000Z", "Asia/Tokyo"),
    ).toMatchObject({
      date: "2021-01-01",
      month: "2021-01",
      week: "2020-W53",
    });
  });

  test("rejects invalid timestamps", () => {
    expect(() => toLocalDateTime("invalid", "UTC")).toThrow(
      'Invalid Discord message timestamp: "invalid".',
    );
  });
});
