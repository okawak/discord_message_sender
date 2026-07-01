export interface LocalDateTime {
  date: string;
  month: string;
  week: string;
  time: string;
  fileTimestamp: string;
}

export function getLocalTimeZone(): string {
  return Intl.DateTimeFormat().resolvedOptions().timeZone || "UTC";
}

export function toLocalDateTime(
  timestamp: string,
  timeZone: string,
): LocalDateTime {
  const date = new Date(timestamp);
  if (Number.isNaN(date.getTime())) {
    throw new Error(`Invalid Discord message timestamp: "${timestamp}".`);
  }

  const parts = new Intl.DateTimeFormat("en-US-u-nu-latn", {
    timeZone,
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hourCycle: "h23",
  }).formatToParts(date);
  const year = readPart(parts, "year");
  const month = readPart(parts, "month");
  const day = readPart(parts, "day");
  const hour = readPart(parts, "hour");
  const minute = readPart(parts, "minute");
  const second = readPart(parts, "second");
  const localDate = `${year}-${month}-${day}`;

  return {
    date: localDate,
    month: `${year}-${month}`,
    week: getIsoWeek(Number(year), Number(month), Number(day)),
    time: `${hour}:${minute}`,
    fileTimestamp: `${year}${month}${day}_${hour}${minute}${second}`,
  };
}

function readPart(
  parts: Intl.DateTimeFormatPart[],
  type: Intl.DateTimeFormatPartTypes,
): string {
  const value = parts.find((part) => part.type === type)?.value;
  if (!value) {
    throw new Error(`Could not resolve local date part "${type}".`);
  }
  return value;
}

function getIsoWeek(year: number, month: number, day: number): string {
  const date = new Date(Date.UTC(year, month - 1, day));
  const weekday = date.getUTCDay() || 7;
  date.setUTCDate(date.getUTCDate() + 4 - weekday);
  const weekYear = date.getUTCFullYear();
  const yearStart = new Date(Date.UTC(weekYear, 0, 1));
  const weekNumber = Math.ceil(
    ((date.getTime() - yearStart.getTime()) / 86_400_000 + 1) / 7,
  );
  return `${weekYear}-W${weekNumber.toString().padStart(2, "0")}`;
}
