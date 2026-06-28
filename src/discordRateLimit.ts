export function getRateLimitDelay(
  headers: Record<string, string>,
  responseText: string,
): number {
  const header = headers["Retry-After"] ?? headers["retry-after"];
  const headerDelay = secondsToMilliseconds(header);
  if (headerDelay !== undefined) {
    return headerDelay;
  }

  try {
    const payload: unknown = JSON.parse(responseText);
    if (isRecord(payload)) {
      const bodyDelay = secondsToMilliseconds(payload.retry_after);
      if (bodyDelay !== undefined) {
        return bodyDelay;
      }
    }
  } catch {
    // Fall back to one second below.
  }

  return 1000;
}

function secondsToMilliseconds(value: unknown): number | undefined {
  if (typeof value !== "number" && typeof value !== "string") {
    return undefined;
  }
  const seconds = Number(value);
  return Number.isFinite(seconds) && seconds >= 0
    ? Math.ceil(seconds * 1000)
    : undefined;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return !!value && typeof value === "object" && !Array.isArray(value);
}
