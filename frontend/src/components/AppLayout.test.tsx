import { render, screen } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { describe, expect, it } from "vitest";

import { AppLayout } from "./AppLayout";

const workspace = {
  id: "workspace-1",
  name: "Personal",
  timezone: "Asia/Shanghai",
  createdAt: "2026-07-16T00:00:00Z",
  updatedAt: "2026-07-16T00:00:00Z",
};

describe("AppLayout navigation", () => {
  it("highlights only AI services on its nested settings route", () => {
    render(
      <MemoryRouter initialEntries={["/settings/ai-providers"]}>
        <Routes>
          <Route element={<AppLayout workspace={workspace} />}>
            <Route path="settings/ai-providers" element={<div>AI services page</div>} />
          </Route>
        </Routes>
      </MemoryRouter>,
    );

    expect(screen.getByRole("link", { name: "AI 服务" })).toHaveAttribute("aria-current", "page");
    expect(screen.getByRole("link", { name: "应用设置" })).not.toHaveAttribute("aria-current");
  });
});
