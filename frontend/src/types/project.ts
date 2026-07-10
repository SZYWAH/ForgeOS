export const PROJECT_STATUSES = [
  "idea",
  "planned",
  "active",
  "paused",
  "maintenance",
  "archived",
] as const;

export const LIFECYCLE_STAGES = [
  "discovery",
  "definition",
  "design",
  "implementation",
  "validation",
  "release",
  "operation",
  "retrospective",
] as const;

export type ProjectStatus = (typeof PROJECT_STATUSES)[number];
export type LifecycleStage = (typeof LIFECYCLE_STAGES)[number];

export type Workspace = {
  id: string;
  name: string;
  timezone: string;
  createdAt: string;
  updatedAt: string;
};

export type Project = {
  id: string;
  workspaceId: string;
  slug: string;
  name: string;
  description: string;
  vision: string;
  targetUser: string;
  scope: string;
  nonGoals: string;
  techStack: string;
  status: ProjectStatus;
  lifecycleStage: LifecycleStage;
  priority: number;
  createdAt: string;
  updatedAt: string;
  version: number;
};

export type ProjectSummary = Pick<
  Project,
  | "id"
  | "slug"
  | "name"
  | "description"
  | "status"
  | "lifecycleStage"
  | "priority"
  | "updatedAt"
  | "version"
>;

export type CreateWorkspaceInput = {
  name: string;
  timezone: string;
};

export type CreateProjectInput = {
  slug: string;
  name: string;
  description: string;
  vision: string;
  targetUser: string;
  scope: string;
  nonGoals: string;
  techStack: string;
  status: ProjectStatus;
  lifecycleStage: LifecycleStage;
  priority: number;
};

export type UpdateProjectInput = CreateProjectInput & {
  expectedVersion: number;
};
