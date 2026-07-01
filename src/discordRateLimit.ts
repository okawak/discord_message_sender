export function getRateLimitDelay(
  headers: Record<string, string>,
  responseText: string,
): number {
  const header = readHeader(headers, "Retry-After");
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

export function getRateLimitResetDelay(
  headers: Record<string, string>,
): number {
  if (Number(readHeader(headers, "X-RateLimit-Remaining")) !== 0) {
    return 0;
  }
  return (
    secondsToMilliseconds(readHeader(headers, "X-RateLimit-Reset-After")) ?? 0
  );
}

function readHeader(
  headers: Record<string, string>,
  name: string,
): string | undefined {
  const normalizedName = name.toLowerCase();
  return Object.entries(headers).find(
    ([key]) => key.toLowerCase() === normalizedName,
  )?.[1];
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
