const STATUS_LABELS: Record<string, string> = {
  idea: "Idea",
  planned: "Planned",
  active: "Active",
  paused: "Paused",
  maintenance: "Maintenance",
  archived: "Archived",
};

const LIFECYCLE_LABELS: Record<string, string> = {
  discovery: "Discovery",
  definition: "Definition",
  design: "Design",
  implementation: "Implementation",
  validation: "Validation",
  release: "Release",
  operation: "Operation",
  retrospective: "Retrospective",
};

export function statusLabel(value: string): string {
  return STATUS_LABELS[value] ?? value;
}

export function lifecycleLabel(value: string): string {
  return LIFECYCLE_LABELS[value] ?? value;
}

export function formatUpdatedAt(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }

  return new Intl.DateTimeFormat("zh-CN", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  }).format(date);
}

export function toProjectSlug(value: string): string {
  return value
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 64);
}
