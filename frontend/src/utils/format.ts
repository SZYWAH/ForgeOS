import i18n from "../i18n";

const STATUS_LABELS = {
  "zh-CN": { idea: "构想中", planned: "已规划", active: "进行中", paused: "已暂停", maintenance: "维护中", archived: "已归档" },
  "en-US": { idea: "Idea", planned: "Planned", active: "Active", paused: "Paused", maintenance: "Maintenance", archived: "Archived" },
} as const;

const LIFECYCLE_LABELS = {
  "zh-CN": { discovery: "探索", definition: "定义", design: "设计", implementation: "实现", validation: "验证", release: "发布", operation: "运行", retrospective: "复盘" },
  "en-US": { discovery: "Discovery", definition: "Definition", design: "Design", implementation: "Implementation", validation: "Validation", release: "Release", operation: "Operation", retrospective: "Retrospective" },
} as const;

export function currentLocale(): "zh-CN" | "en-US" {
  return i18n.resolvedLanguage === "en-US" ? "en-US" : "zh-CN";
}

export function statusLabel(value: string): string {
  return (STATUS_LABELS[currentLocale()] as Record<string, string>)[value] ?? value;
}

export function lifecycleLabel(value: string): string {
  return (LIFECYCLE_LABELS[currentLocale()] as Record<string, string>)[value] ?? value;
}

export function formatUpdatedAt(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }

  return new Intl.DateTimeFormat(currentLocale(), {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  }).format(date);
}

export function formatRelativeTime(value: string, now = new Date()): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  const seconds = Math.round((date.getTime() - now.getTime()) / 1000);
  const absolute = Math.abs(seconds);
  const formatter = new Intl.RelativeTimeFormat(currentLocale(), { numeric: "auto" });
  if (absolute < 60) return formatter.format(seconds, "second");
  if (absolute < 3_600) return formatter.format(Math.round(seconds / 60), "minute");
  if (absolute < 86_400) return formatter.format(Math.round(seconds / 3_600), "hour");
  return formatter.format(Math.round(seconds / 86_400), "day");
}

const DISPLAY_LABELS = {
  "zh-CN": {
    open: "进行中", completed: "已完成", abandoned: "已停止",
    pending: "待处理", accepted: "已接受", rejected: "已拒绝", archived: "已归档",
    draft: "草稿", proposed: "待审批", approved: "已批准",
    running: "进行中", success: "成功", failed: "失败", cancelled: "已取消", queued: "等待中", partial: "部分完成", succeeded: "已完成",
    unverified: "待确认", verified: "已确认", invalid: "无效",
    document: "文档", screenshot: "截图", link: "链接", test_result: "测试结果", note: "人工记录", artifact: "构建产物", git_commit: "Git 提交", git_status: "Git 状态", github_repository: "GitHub 仓库", github_commit: "GitHub 提交", github_release: "GitHub 发布版本",
    local_file: "本地文件", external_url: "外部链接", manual_note: "人工记录", github_api: "GitHub API",
    supports: "用于证明", relates_to: "与它相关",
    planning: "规划", coding: "编码", debugging: "问题排查", review: "检查", handoff: "交接",
    snapshot_update: "当前状态更新", session_summary: "开发记录摘要", evidence_suggestion: "项目资料建议", risk_finding: "风险提醒", general_note: "一般建议",
    reviewed: "已检查", imported: "已导入",
    generated: "已生成", sent_to_ai: "已发送给 AI",
  },
  "en-US": {
    open: "In progress", completed: "Completed", abandoned: "Stopped",
    pending: "Needs review", accepted: "Accepted", rejected: "Rejected", archived: "Archived",
    draft: "Draft", proposed: "Awaiting approval", approved: "Approved",
    running: "Running", success: "Success", failed: "Failed", cancelled: "Cancelled", queued: "Queued", partial: "Partially complete", succeeded: "Completed",
    unverified: "Needs review", verified: "Confirmed", invalid: "Invalid",
    document: "Document", screenshot: "Screenshot", link: "Link", test_result: "Test result", note: "Manual note", artifact: "Build artifact", git_commit: "Git commit", git_status: "Git state", github_repository: "GitHub repository", github_commit: "GitHub commit", github_release: "GitHub release",
    local_file: "Local file", external_url: "External link", manual_note: "Manual note", github_api: "GitHub API",
    supports: "Supports", relates_to: "Related to",
    planning: "Planning", coding: "Coding", debugging: "Debugging", review: "Review", handoff: "Handoff",
    snapshot_update: "Current state update", session_summary: "Development log summary", evidence_suggestion: "Project material suggestion", risk_finding: "Risk finding", general_note: "General suggestion",
    reviewed: "Reviewed", imported: "Imported",
    generated: "Generated", sent_to_ai: "Sent to AI",
  },
} as const;

export function displayLabel(value: string): string {
  return (DISPLAY_LABELS[currentLocale()] as Record<string, string>)[value] ?? value;
}

export function toProjectSlug(value: string): string {
  return value
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 64);
}
