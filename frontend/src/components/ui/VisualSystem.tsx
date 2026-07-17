import type {
  ButtonHTMLAttributes,
  HTMLAttributes,
  ReactNode,
} from "react";

function mergeClassNames(...classNames: Array<string | false | undefined>) {
  return classNames.filter(Boolean).join(" ");
}

export interface AppButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant: "primary" | "secondary" | "quiet" | "danger";
  size?: "compact" | "default" | "prominent";
  loading?: boolean;
  fullWidth?: boolean;
}

export function AppButton({
  variant,
  size = "default",
  loading = false,
  fullWidth = false,
  className,
  children,
  disabled,
  type,
  ...props
}: AppButtonProps) {
  return (
    <button
      {...props}
      aria-busy={loading || undefined}
      className={mergeClassNames(
        "app-button",
        `app-button--${variant}`,
        `app-button--${size}`,
        fullWidth && "app-button--full-width",
        loading && "app-button--loading",
        className,
      )}
      disabled={disabled || loading}
      type={type ?? "button"}
    >
      {loading ? <span aria-hidden="true" className="app-button__spinner" /> : null}
      <span className="app-button__label">{children}</span>
    </button>
  );
}

export interface SurfaceProps extends HTMLAttributes<HTMLDivElement> {
  variant?: "plain" | "outlined" | "raised";
  padding?: "none" | "compact" | "default";
}

export function Surface({
  variant = "plain",
  padding = "default",
  className,
  ...props
}: SurfaceProps) {
  return (
    <div
      {...props}
      className={mergeClassNames(
        "surface",
        `surface--${variant}`,
        `surface--padding-${padding}`,
        className,
      )}
    />
  );
}

export interface StatusBadgeProps extends HTMLAttributes<HTMLSpanElement> {
  tone: "neutral" | "info" | "success" | "warning" | "danger";
}

export function StatusBadge({ tone, className, ...props }: StatusBadgeProps) {
  return (
    <span
      {...props}
      className={mergeClassNames(
        "status-badge",
        `status-badge--${tone}`,
        className,
      )}
    />
  );
}

export interface NoticeProps extends HTMLAttributes<HTMLElement> {
  tone: "info" | "success" | "warning" | "danger";
  title?: string;
  action?: ReactNode;
}

export function Notice({
  tone,
  title,
  action,
  className,
  children,
  role,
  ...props
}: NoticeProps) {
  return (
    <section
      {...props}
      className={mergeClassNames("notice", `notice--${tone}`, className)}
      role={role ?? (tone === "danger" ? "alert" : undefined)}
    >
      <div className="notice__content">
        {title ? <strong className="notice__title">{title}</strong> : null}
        <div className="notice__body">{children}</div>
      </div>
      {action ? <div className="notice__action">{action}</div> : null}
    </section>
  );
}

export interface EmptyStateProps extends HTMLAttributes<HTMLDivElement> {
  title: string;
  description: string;
  action?: ReactNode;
  density?: "compact" | "default";
}

export function EmptyState({
  title,
  description,
  action,
  density = "default",
  className,
  ...props
}: EmptyStateProps) {
  return (
    <div
      {...props}
      className={mergeClassNames(
        "empty-state",
        `empty-state--${density}`,
        className,
      )}
    >
      <strong className="empty-state__title">{title}</strong>
      <p className="empty-state__description">{description}</p>
      {action ? <div className="empty-state__action">{action}</div> : null}
    </div>
  );
}

export interface ListDetailLayoutProps extends HTMLAttributes<HTMLDivElement> {
  list: ReactNode;
  detail: ReactNode;
  empty: boolean;
}

export function ListDetailLayout({
  list,
  detail,
  empty,
  className,
  ...props
}: ListDetailLayoutProps) {
  if (empty) {
    return (
      <div
        {...props}
        className={mergeClassNames(
          "list-detail-layout",
          "list-detail-layout--empty",
          className,
        )}
      >
        <div className="list-detail-layout__empty">{detail}</div>
      </div>
    );
  }

  return (
    <div
      {...props}
      className={mergeClassNames("list-detail-layout", className)}
    >
      <div className="list-detail-layout__list">{list}</div>
      <div className="list-detail-layout__detail">{detail}</div>
    </div>
  );
}
