import { describe, expect, it } from "vitest";

const styleModules = import.meta.glob("./*.css", {
  eager: true,
  import: "default",
  query: "?raw",
}) as Record<string, string>;

describe("visual system CSS contract", () => {
  it("keeps hard-coded colors inside the token or compatibility layer", () => {
    const violations = Object.entries(styleModules)
      .filter(([path]) => !path.endsWith("tokens.css") && !path.endsWith("legacy.css"))
      .flatMap(([path, css]) => {
        const matches = css.match(/#[0-9a-f]{3,8}\b|rgba?\(/gi) ?? [];
        return matches.map((value) => `${path}: ${value}`);
      });
    expect(violations).toEqual([]);
  });

  it("keeps every active stylesheet in the declared cascade", () => {
    expect(Object.keys(styleModules).sort()).toEqual([
      "./base.css",
      "./components.css",
      "./legacy.css",
      "./pages.css",
      "./patterns.css",
      "./shell.css",
      "./tokens.css",
      "./utilities.css",
    ]);
  });
});
