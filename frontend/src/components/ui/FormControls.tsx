import type {
  ChangeEvent,
  HTMLAttributes,
  ReactElement,
  ReactNode,
  SelectHTMLAttributes,
} from "react";
import { cloneElement, isValidElement } from "react";

function mergeClassNames(...classNames: Array<string | false | undefined>) {
  return classNames.filter(Boolean).join(" ");
}

export interface FormFieldProps extends HTMLAttributes<HTMLDivElement> {
  id: string;
  label: string;
  required?: boolean;
  description?: string;
  error?: string;
  children: ReactNode;
}

export function FormField({
  id,
  label,
  required = false,
  description,
  error,
  className,
  children,
  ...props
}: FormFieldProps) {
  const descriptionId = description ? `${id}-description` : undefined;
  const errorId = error ? `${id}-error` : undefined;
  const describedBy = [descriptionId, errorId].filter(Boolean).join(" ") || undefined;
  const control = isValidElement(children)
    ? cloneElement(children as ReactElement<Record<string, unknown>>, {
      "aria-describedby": describedBy,
      "aria-invalid": error ? true : undefined,
    })
    : children;

  return (
    <div
      {...props}
      className={mergeClassNames("form-field", error && "form-field--error", className)}
    >
      <label className="form-field__label" htmlFor={id}>
        <span>{label}</span>
        {required ? <span aria-hidden="true" className="form-field__required">*</span> : null}
      </label>
      {control}
      {description ? (
        <small className="form-field__description" id={descriptionId}>
          {description}
        </small>
      ) : null}
      {error ? (
        <small className="form-field__error" id={errorId} role="alert">
          {error}
        </small>
      ) : null}
    </div>
  );
}

export interface SelectOption {
  label: string;
  value: string;
}

export interface SelectFieldProps
  extends Omit<SelectHTMLAttributes<HTMLSelectElement>, "children" | "onChange"> {
  id: string;
  label: string;
  options: SelectOption[];
  description?: string;
  error?: string;
  onChange: (value: string, event: ChangeEvent<HTMLSelectElement>) => void;
}

export function SelectField({
  id,
  label,
  options,
  description,
  error,
  required,
  className,
  onChange,
  ...props
}: SelectFieldProps) {
  const describedBy = [
    description ? `${id}-description` : undefined,
    error ? `${id}-error` : undefined,
  ]
    .filter(Boolean)
    .join(" ") || undefined;

  return (
    <FormField
      description={description}
      error={error}
      id={id}
      label={label}
      required={required}
    >
      <select
        {...props}
        aria-describedby={describedBy}
        aria-invalid={error ? true : undefined}
        className={mergeClassNames("form-control form-control--select", className)}
        id={id}
        onChange={(event) => onChange(event.target.value, event)}
        required={required}
      >
        {options.map((option) => (
          <option key={option.value} value={option.value}>
            {option.label}
          </option>
        ))}
      </select>
    </FormField>
  );
}

export interface CheckboxFieldProps
  extends Omit<HTMLAttributes<HTMLDivElement>, "onChange"> {
  id: string;
  label: string;
  description?: string;
  checked: boolean;
  disabled?: boolean;
  onChange: (checked: boolean) => void;
}

export function CheckboxField({
  id,
  label,
  description,
  checked,
  disabled,
  className,
  onChange,
  ...props
}: CheckboxFieldProps) {
  const descriptionId = description ? `${id}-description` : undefined;

  return (
    <div
      {...props}
      className={mergeClassNames(
        "checkbox-field",
        disabled && "checkbox-field--disabled",
        className,
      )}
    >
      <input
        aria-describedby={descriptionId}
        checked={checked}
        disabled={disabled}
        id={id}
        onChange={(event) => onChange(event.target.checked)}
        type="checkbox"
      />
      <label htmlFor={id}>
        <span className="checkbox-field__label">{label}</span>
      </label>
      {description ? (
        <small className="checkbox-field__description" id={descriptionId}>
          {description}
        </small>
      ) : null}
    </div>
  );
}

export interface FormSectionProps extends HTMLAttributes<HTMLElement> {
  title: string;
  description?: string;
  children: ReactNode;
}

export function FormSection({
  title,
  description,
  className,
  children,
  ...props
}: FormSectionProps) {
  return (
    <section {...props} className={mergeClassNames("form-section", className)}>
      <header className="form-section__header">
        <h3>{title}</h3>
        {description ? <p>{description}</p> : null}
      </header>
      <div className="form-section__body">{children}</div>
    </section>
  );
}
