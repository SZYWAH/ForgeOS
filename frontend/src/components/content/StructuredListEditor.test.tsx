import { fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { describe, expect, it } from "vitest";
import { StructuredListEditor } from "./StructuredListEditor";

function Fixture({ initial = ["第一项", "第二项"], maxItems = 3 }: { initial?: string[]; maxItems?: number }) {
  const [items, setItems] = useState(initial);
  return (
    <StructuredListEditor
      addLabel="添加"
      emptyLabel="暂无内容"
      items={items}
      label="接下来做什么"
      maxItems={maxItems}
      moveDownLabel="下移"
      moveUpLabel="上移"
      onChange={setItems}
      removeLabel="删除"
    />
  );
}

describe("StructuredListEditor", () => {
  it("edits, reorders and removes each item independently", () => {
    render(<Fixture />);
    fireEvent.change(screen.getByRole("textbox", { name: "接下来做什么 1" }), { target: { value: "更新后的第一项" } });
    expect(screen.getByRole("textbox", { name: "接下来做什么 1" })).toHaveValue("更新后的第一项");

    fireEvent.click(screen.getByRole("button", { name: "下移 1" }));
    expect(screen.getByRole("textbox", { name: "接下来做什么 1" })).toHaveValue("第二项");

    fireEvent.click(screen.getAllByRole("button", { name: "删除" })[0]);
    expect(screen.getAllByRole("textbox")).toHaveLength(1);
  });

  it("does not allow a fourth next step", () => {
    render(<Fixture initial={["一", "二", "三"]} maxItems={3} />);
    expect(screen.getByRole("button", { name: "添加" })).toBeDisabled();
    expect(screen.getByText("3 / 3")).toBeInTheDocument();
  });
});
