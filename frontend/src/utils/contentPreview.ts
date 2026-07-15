const REQUEST_MARKER = /(?:^|\n)\s*#{0,6}\s*My request for Codex\s*:\s*/i;
const FILES_MARKER = /(?:^|\n)\s*#{0,6}\s*Files mentioned by the user\s*:\s*/i;

export function formatFactPreview(value: string): string {
  const normalized = value.replace(/\r\n?/g, "\n").trim();
  if (!normalized) return "";

  const requestIndex = normalized.search(REQUEST_MARKER);
  let candidate = requestIndex >= 0
    ? normalized.slice(requestIndex).replace(REQUEST_MARKER, "")
    : normalized;

  if (requestIndex < 0 && FILES_MARKER.test(candidate)) {
    candidate = candidate.replace(FILES_MARKER, "");
  }

  const cleaned = candidate
    .replace(/^\s*#{1,6}\s*/gm, "")
    .replace(/^\s*\[local path redacted\]\s*$/gim, "")
    .replace(/`{1,3}/g, "")
    .replace(/\s+/g, " ")
    .trim();

  return cleaned || normalized.replace(/\s+/g, " ").trim();
}
