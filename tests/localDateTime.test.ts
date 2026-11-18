import { describe, expect, test } from "bun:test";
import {
  possible_local_dates as getPossibleLocalDateTimes,
  local_date_time as toLocalDateTime,
} from "../pkg/dms_wasm.js";

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

  test("keeps whole-second file names for fractional and leap seconds", () => {
    expect(
      toLocalDateTime("1969-12-31t23:59:59.999999999123z", "UTC").fileTimestamp,
    ).toBe("19691231_235959");
    expect(
      toLocalDateTime("2017-01-01T08:59:60.25+09:00", "Asia/Tokyo")
        .fileTimestamp,
    ).toBe("20170101_085960");
    expect(
      getPossibleLocalDateTimes("2017-01-01T08:59:60.25+09:00")[1]
        ?.fileTimestamp,
    ).toBe("20161231_235960");
  });

  test("rejects invalid timestamps", () => {
    expect(() => toLocalDateTime("invalid", "UTC")).toThrow(
      'Invalid Discord message timestamp: "invalid".',
    );
  });

  test("covers every possible local date for cross-time-zone retries", () => {
    const dates = getPossibleLocalDateTimes("2026-07-01T00:30:00.000Z").map(
      ({ date }) => date,
    );

    expect(dates).toEqual(["2026-06-30", "2026-07-01", "2026-07-01"]);
  });
});
