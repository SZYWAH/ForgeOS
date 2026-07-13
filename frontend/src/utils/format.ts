const STATUS_LABELS: Record<string, string> = {
  idea: "构想中",
  planned: "已规划",
  active: "进行中",
  paused: "已暂停",
  maintenance: "维护中",
  archived: "已归档",
};

const LIFECYCLE_LABELS: Record<string, string> = {
  discovery: "探索",
  definition: "定义",
  design: "设计",
  implementation: "实现",
  validation: "验证",
  release: "发布",
  operation: "运行",
  retrospective: "复盘",
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
