export type AchievementCategory =
  | "feature"
  | "problem_solution"
  | "release"
  | "technical_highlight";
export type AchievementStatus = "draft" | "confirmed" | "archived";

export type ProjectAchievement = {
  id: string;
  projectId: string;
  category: AchievementCategory;
  title: string;
  problem: string;
  approach: string;
  result: string;
  validation: string;
  impact: string;
  technologies: string[];
  resumeBullet: string;
  status: AchievementStatus;
  sourceMode: "manual" | "pulse_rule";
  featured: boolean;
  sourcePulseId: string | null;
  confirmedAt: string | null;
  createdAt: string;
  updatedAt: string;
  version: number;
};

export type AchievementInput = Pick<
  ProjectAchievement,
  | "category"
  | "title"
  | "problem"
  | "approach"
  | "result"
  | "validation"
  | "impact"
  | "technologies"
  | "resumeBullet"
  | "featured"
>;
