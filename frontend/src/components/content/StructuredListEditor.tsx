type StructuredListEditorProps = {
  label: string;
  items: string[];
  onChange: (items: string[]) => void;
  maxItems?: number;
  itemMaxLength?: number;
  addLabel: string;
  emptyLabel: string;
  moveUpLabel: string;
  moveDownLabel: string;
  removeLabel: string;
  disabled?: boolean;
};

export function StructuredListEditor({
  label,
  items,
  onChange,
  maxItems = 20,
  itemMaxLength = 2_000,
  addLabel,
  emptyLabel,
  moveUpLabel,
  moveDownLabel,
  removeLabel,
  disabled = false,
}: StructuredListEditorProps) {
  const update = (index: number, value: string) => {
    onChange(items.map((item, itemIndex) => itemIndex === index ? value : item));
  };

  const move = (index: number, offset: -1 | 1) => {
    const target = index + offset;
    if (target < 0 || target >= items.length) return;
    const next = [...items];
    [next[index], next[target]] = [next[target], next[index]];
    onChange(next);
  };

  return (
    <fieldset className="structured-list-editor" disabled={disabled}>
      <legend>{label}</legend>
      {items.length === 0 ? <p className="structured-list-editor__empty">{emptyLabel}</p> : null}
      <div className="structured-list-editor__items">
        {items.map((item, index) => (
          <div className="structured-list-editor__item" key={index}>
            <textarea
              aria-label={`${label} ${index + 1}`}
              maxLength={itemMaxLength}
              onChange={(event) => update(index, event.target.value)}
              rows={2}
              value={item}
            />
            <div className="structured-list-editor__actions">
              <button
                aria-label={`${moveUpLabel} ${index + 1}`}
                className="button quiet compact"
                disabled={index === 0}
                onClick={() => move(index, -1)}
                type="button"
              >
                ↑
              </button>
              <button
                aria-label={`${moveDownLabel} ${index + 1}`}
                className="button quiet compact"
                disabled={index === items.length - 1}
                onClick={() => move(index, 1)}
                type="button"
              >
                ↓
              </button>
              <button
                className="button quiet compact danger"
                onClick={() => onChange(items.filter((_, itemIndex) => itemIndex !== index))}
                type="button"
              >
                {removeLabel}
              </button>
            </div>
          </div>
        ))}
      </div>
      <button
        className="button secondary compact"
        disabled={items.length >= maxItems}
        onClick={() => onChange([...items, ""])}
        type="button"
      >
        {addLabel}
      </button>
      <small>{items.length} / {maxItems}</small>
    </fieldset>
  );
}
