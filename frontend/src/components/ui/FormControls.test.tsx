import { fireEvent, render } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import {
  CheckboxField,
  FormField,
  SelectField,
} from "./FormControls";

describe("form control contract", () => {
  it("keeps the required mark inside the field label and associates help text", () => {
    const { getByLabelText, getByText } = render(
      <FormField
        description="Choose the model deployment name."
        id="provider-model"
        label="Model"
        required
      >
        <input className="form-control" id="provider-model" required />
      </FormField>,
    );

    const input = getByLabelText(/Model/);
    expect(input).toHaveAttribute("aria-describedby", "provider-model-description");
    expect(getByText("*")).toHaveClass("form-field__required");
  });

  it("uses a native select and reports value changes", () => {
    const onChange = vi.fn();
    const { getByLabelText } = render(
      <SelectField
        id="provider-preset"
        label="Preset"
        onChange={onChange}
        options={[
          { label: "Custom", value: "custom" },
          { label: "DeepSeek", value: "deepseek" },
        ]}
        value="custom"
      />,
    );

    fireEvent.change(getByLabelText("Preset"), { target: { value: "deepseek" } });
    expect(onChange).toHaveBeenCalledWith("deepseek", expect.anything());
  });

  it("keeps checkbox text aligned in a dedicated checkbox field", () => {
    const onChange = vi.fn();
    const { getByLabelText } = render(
      <CheckboxField
        checked={false}
        description="Reject calls when the key is missing."
        id="provider-secret-required"
        label="Credential required"
        onChange={onChange}
      />,
    );

    const checkbox = getByLabelText("Credential required");
    expect(checkbox).toHaveAttribute("type", "checkbox");
    expect(checkbox).toHaveAttribute(
      "aria-describedby",
      "provider-secret-required-description",
    );
    expect(checkbox.closest(".checkbox-field")).toBeInTheDocument();
    fireEvent.click(checkbox);
    expect(onChange).toHaveBeenCalledWith(true);
  });
});
