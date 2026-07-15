import { fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { describe, expect, it, vi } from "vitest";
import {
  BoundedList,
  CollapsibleText,
  CompactIdentifier,
  ExclusiveExpansionGroup,
  RecordDisclosure,
} from "./ContentGovernance";
import { formatFactPreview } from "../../utils/contentPreview";

describe("content governance", () => {
  it("extracts the actual request from a legacy Codex wrapper", () => {
    const value = "# Files mentioned by the user:\n\n## image.jpg:\n[local path redacted]\n\n## My request for Codex:\n别人在安装的时候出现这个问题怎么解决？";
    expect(formatFactPreview(value)).toBe("别人在安装的时候出现这个问题怎么解决？");
  });

  it("expands a normalized preview without mutating the original text", () => {
    const value = "## My request for Codex:\n修复一个很长的问题";
    const { container } = render(<CollapsibleText previewMode="fact" value={value} />);
    expect(screen.getByText("修复一个很长的问题")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "展开全文" }));
    expect(container.querySelector(".collapsible-text__content")?.textContent).toBe(value);
    expect(screen.getByRole("button", { name: "收起" })).toHaveAttribute("aria-expanded", "true");
  });

  it("only exposes a toggle for genuinely overflowing plain content", () => {
    const scrollHeight = vi.spyOn(HTMLElement.prototype, "scrollHeight", "get").mockReturnValue(120);
    const clientHeight = vi.spyOn(HTMLElement.prototype, "clientHeight", "get").mockReturnValue(40);
    render(<CollapsibleText value={"长中文内容".repeat(200)} />);
    expect(screen.getByRole("button", { name: "展开全文" })).toHaveAttribute("aria-expanded", "false");
    scrollHeight.mockRestore();
    clientHeight.mockRestore();
  });

  it("keeps a ten-thousand-character document inside the governed expanded region", () => {
    const value = "文".repeat(10_000);
    const { container } = render(<CollapsibleText previewMode="fact" value={`## My request for Codex:\n${value}`} expandedMaxHeight={480} />);
    fireEvent.click(screen.getByRole("button", { name: "展开全文" }));
    expect(container.querySelector(".collapsible-text__content")).toHaveStyle({ "--expanded-max-height": "480px" });
  });

  it("resets expanded state when navigating to another value", () => {
    const { rerender } = render(<CollapsibleText previewMode="fact" value={"## My request for Codex:\n第一个目标"} />);
    fireEvent.click(screen.getByRole("button", { name: "展开全文" }));
    rerender(<CollapsibleText previewMode="fact" value={"## My request for Codex:\n第二个目标"} />);
    expect(screen.getByRole("button", { name: "展开全文" })).toHaveAttribute("aria-expanded", "false");
  });

  it("shows the remaining item count and can reveal the full list", () => {
    render(<BoundedList items={["一", "二", "三", "四", "五"]} />);
    expect(screen.queryByText("四")).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "查看其余 2 项" }));
    expect(screen.getByText("五")).toBeInTheDocument();
  });

  it("supports a controlled expanded state", () => {
    function ControlledText() {
      const [expanded, setExpanded] = useState(false);
      return <CollapsibleText expanded={expanded} onExpandedChange={setExpanded} previewMode="fact" value={"## My request for Codex:\n受控目标"} />;
    }
    render(<ControlledText />);
    fireEvent.click(screen.getByRole("button", { name: "展开全文" }));
    expect(screen.getByRole("button", { name: "收起" })).toHaveAttribute("aria-expanded", "true");
  });

  it("keeps only one long text expanded inside an exclusive group", () => {
    const { container } = render(<ExclusiveExpansionGroup>
      <CollapsibleText previewMode="fact" value={"## My request for Codex:\n目标一"} />
      <CollapsibleText previewMode="fact" value={"## My request for Codex:\n目标二"} />
    </ExclusiveExpansionGroup>);
    const toggles = screen.getAllByRole("button", { name: "展开全文" });
    fireEvent.click(toggles[0]);
    expect(container.querySelectorAll(".collapsible-text__content")[0]?.textContent).toBe("## My request for Codex:\n目标一");
    fireEvent.click(screen.getAllByRole("button", { name: "展开全文" })[0]);
    expect(container.querySelectorAll(".collapsible-text__content")[0]?.textContent).toBe("目标一");
    expect(container.querySelectorAll(".collapsible-text__content")[1]?.textContent).toBe("## My request for Codex:\n目标二");
  });

  it("applies the overview expansion limit to bounded list items", () => {
    const scrollHeight = vi.spyOn(HTMLElement.prototype, "scrollHeight", "get").mockReturnValue(180);
    const clientHeight = vi.spyOn(HTMLElement.prototype, "clientHeight", "get").mockReturnValue(40);
    const { container } = render(<BoundedList exclusiveExpansion expandedMaxHeight={280} items={["长内容".repeat(300)]} />);
    fireEvent.click(screen.getByRole("button", { name: "展开全文" }));
    expect(container.querySelector(".collapsible-text__content")).toHaveStyle({ "--expanded-max-height": "280px" });
    scrollHeight.mockRestore();
    clientHeight.mockRestore();
  });

  it("uses an externally controlled single-record disclosure state", () => {
    const onToggle = vi.fn();
    const { rerender } = render(<RecordDisclosure expanded={false} onToggle={onToggle} recordId="v1" summary="版本 1"><p>完整内容</p></RecordDisclosure>);
    fireEvent.click(screen.getByRole("button", { name: /版本 1/ }));
    expect(onToggle).toHaveBeenCalledWith("v1");
    rerender(<RecordDisclosure expanded onToggle={onToggle} recordId="v1" summary="版本 1"><p>完整内容</p></RecordDisclosure>);
    expect(screen.getByText("完整内容")).toBeInTheDocument();
  });

  it("allows a history list to keep only one record expanded", () => {
    function HistoryHarness() {
      const [expanded, setExpanded] = useState<string | null>(null);
      return <>
        <RecordDisclosure expanded={expanded === "one"} onToggle={(id) => setExpanded((current) => current === id ? null : id)} recordId="one" summary="记录一"><p>详情一</p></RecordDisclosure>
        <RecordDisclosure expanded={expanded === "two"} onToggle={(id) => setExpanded((current) => current === id ? null : id)} recordId="two" summary="记录二"><p>详情二</p></RecordDisclosure>
      </>;
    }
    render(<HistoryHarness />);
    fireEvent.click(screen.getByRole("button", { name: /记录一/ }));
    expect(screen.getByText("详情一")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /记录二/ }));
    expect(screen.queryByText("详情一")).not.toBeInTheDocument();
    expect(screen.getByText("详情二")).toBeInTheDocument();
  });

  it("copies a long identifier without exposing it as a wrapping label", async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.assign(navigator, { clipboard: { writeText } });
    const value = "C:\\very\\long\\path\\without\\a\\reasonable\\break\\artifact.txt?hash=" + "a".repeat(160);
    render(<CompactIdentifier value={value} />);
    fireEvent.click(screen.getByRole("button", { name: "复制标识" }));
    expect(writeText).toHaveBeenCalledWith(value);
  });
});
