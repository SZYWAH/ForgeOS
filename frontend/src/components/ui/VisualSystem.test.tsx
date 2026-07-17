import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import {
  AppButton,
  EmptyState,
  ListDetailLayout,
  Notice,
  StatusBadge,
  Surface,
} from "./VisualSystem";

describe("AppButton", () => {
  it("applies its visual contract while preserving native button attributes", () => {
    const onClick = vi.fn();
    render(
      <AppButton
        aria-label="Create provider"
        className="feature-action"
        fullWidth
        onClick={onClick}
        size="prominent"
        variant="primary"
      >
        Create
      </AppButton>,
    );

    const button = screen.getByRole("button", { name: "Create provider" });
    expect(button).toHaveAttribute("type", "button");
    expect(button).toHaveClass(
      "app-button",
      "app-button--primary",
      "app-button--prominent",
      "app-button--full-width",
      "feature-action",
    );
    button.click();
    expect(onClick).toHaveBeenCalledOnce();
  });

  it("disables interaction and exposes busy state while loading", () => {
    render(
      <AppButton loading variant="secondary">
        Save
      </AppButton>,
    );

    const button = screen.getByRole("button", { name: "Save" });
    expect(button).toBeDisabled();
    expect(button).toHaveAttribute("aria-busy", "true");
    expect(button.querySelector(".app-button__spinner")).toHaveAttribute(
      "aria-hidden",
      "true",
    );
  });
});

describe("visual primitives", () => {
  it("renders surfaces with explicit elevation and padding", () => {
    render(
      <Surface data-testid="surface" padding="compact" variant="raised">
        Content
      </Surface>,
    );

    expect(screen.getByTestId("surface")).toHaveClass(
      "surface--raised",
      "surface--padding-compact",
    );
  });

  it("maps badge tone to a stable semantic class", () => {
    render(<StatusBadge tone="warning">Needs review</StatusBadge>);
    expect(screen.getByText("Needs review")).toHaveClass(
      "status-badge--warning",
    );
  });

  it("renders notice content and action without making informational notices alerts", () => {
    render(
      <Notice action={<button type="button">Retry</button>} title="Sync paused" tone="info">
        Codex is unavailable.
      </Notice>,
    );

    expect(screen.getByText("Sync paused")).toBeVisible();
    expect(screen.getByText("Codex is unavailable.")).toBeVisible();
    expect(screen.getByRole("button", { name: "Retry" })).toBeVisible();
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("announces danger notices as alerts", () => {
    render(<Notice tone="danger">Unable to save.</Notice>);
    expect(screen.getByRole("alert")).toHaveTextContent("Unable to save.");
  });

  it("supports compact empty states with an optional action", () => {
    render(
      <EmptyState
        action={<button type="button">Connect</button>}
        density="compact"
        description="Connect a repository to continue."
        title="No repository"
      />,
    );

    expect(screen.getByText("No repository").closest(".empty-state")).toHaveClass(
      "empty-state--compact",
    );
    expect(screen.getByRole("button", { name: "Connect" })).toBeVisible();
  });
});

describe("ListDetailLayout", () => {
  it("renders both list and detail regions when records exist", () => {
    render(
      <ListDetailLayout
        detail={<div>Provider details</div>}
        empty={false}
        list={<div>Provider list</div>}
      />,
    );

    expect(screen.getByText("Provider list")).toBeVisible();
    expect(screen.getByText("Provider details")).toBeVisible();
  });

  it("renders a single empty content surface and omits the list", () => {
    render(
      <ListDetailLayout
        detail={<EmptyState description="Create one to continue." title="No providers" />}
        empty
        list={<div>Provider list</div>}
      />,
    );

    expect(screen.queryByText("Provider list")).not.toBeInTheDocument();
    expect(screen.getByText("No providers")).toBeVisible();
    expect(document.querySelectorAll(".list-detail-layout__empty")).toHaveLength(1);
    expect(document.querySelector(".list-detail-layout__detail")).not.toBeInTheDocument();
  });
});
