import {
  createContext,
  useContext,
  useEffect,
  useId,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
  type ReactNode,
} from "react";
import { useTranslation } from "react-i18next";
import { formatFactPreview } from "../../utils/contentPreview";

type ContentStyle = CSSProperties & Record<"--collapsed-lines" | "--expanded-max-height", string>;

export type CollapsibleTextProps = {
  value: string;
  collapsedLines?: number;
  expandedMaxHeight?: number;
  previewMode?: "plain" | "fact";
  className?: string;
  expanded?: boolean;
  onExpandedChange?: (expanded: boolean) => void;
};

type ExclusiveExpansionContextValue = {
  expandedId: string | null;
  setExpandedId: (id: string | null) => void;
};

const ExclusiveExpansionContext = createContext<ExclusiveExpansionContextValue | null>(null);

export function ExclusiveExpansionGroup({ children }: { children: ReactNode }) {
  const parent = useContext(ExclusiveExpansionContext);
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const value = useMemo(() => ({ expandedId, setExpandedId }), [expandedId]);

  if (parent) return <>{children}</>;
  return <ExclusiveExpansionContext.Provider value={value}>{children}</ExclusiveExpansionContext.Provider>;
}

export function CollapsibleText({
  value,
  collapsedLines = 3,
  expandedMaxHeight = 480,
  previewMode = "plain",
  className = "",
  expanded: controlledExpanded,
  onExpandedChange,
}: CollapsibleTextProps) {
  const { t } = useTranslation();
  const [localExpanded, setLocalExpanded] = useState(false);
  const [overflowing, setOverflowing] = useState(false);
  const expansionId = useId();
  const contentId = `${expansionId}-content`;
  const contentRef = useRef<HTMLDivElement>(null);
  const previousValue = useRef(value);
  const expansionGroup = useContext(ExclusiveExpansionContext);
  const isControlled = controlledExpanded !== undefined;
  const expanded = isControlled
    ? controlledExpanded
    : expansionGroup
      ? expansionGroup.expandedId === expansionId
      : localExpanded;
  const preview = previewMode === "fact" ? formatFactPreview(value) : value;
  const content = expanded ? value : preview;

  useEffect(() => {
    if (previousValue.current === value) return;
    previousValue.current = value;
    if (isControlled) {
      onExpandedChange?.(false);
    } else if (expansionGroup) {
      if (expansionGroup.expandedId === expansionId) expansionGroup.setExpandedId(null);
    } else {
      setLocalExpanded(false);
    }
  }, [value]);

  useEffect(() => {
    const element = contentRef.current;
    if (!element || expanded) return;
    const measure = () => setOverflowing(element.scrollHeight > element.clientHeight + 1);
    measure();
    if (typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    return () => observer.disconnect();
  }, [collapsedLines, expanded, preview]);

  const canToggle = preview !== value || overflowing;
  const style: ContentStyle = {
    "--collapsed-lines": String(collapsedLines),
    "--expanded-max-height": `${expandedMaxHeight}px`,
  };

  function updateExpanded(next: boolean) {
    if (isControlled) {
      onExpandedChange?.(next);
    } else if (expansionGroup) {
      expansionGroup.setExpandedId(next ? expansionId : null);
    } else {
      setLocalExpanded(next);
    }
  }

  return <div className={`collapsible-text ${className}`.trim()} data-expanded={expanded}>
    <div className="collapsible-text__content" id={contentId} ref={contentRef} style={style}>{content}</div>
    {canToggle ? <button aria-controls={contentId} aria-expanded={expanded} className="content-toggle" onClick={() => updateExpanded(!expanded)} type="button">{expanded ? t("common.collapse") : t("common.expandFull")}</button> : null}
  </div>;
}

export type BoundedListProps = {
  items: string[];
  initialCount?: number;
  itemLines?: number;
  emptyLabel?: string;
  ordered?: boolean;
  className?: string;
  exclusiveExpansion?: boolean;
  expandedMaxHeight?: number;
};

export function BoundedList({
  items,
  initialCount = 3,
  itemLines = 2,
  emptyLabel,
  ordered = false,
  className = "",
  exclusiveExpansion = false,
  expandedMaxHeight = 480,
}: BoundedListProps) {
  const { t } = useTranslation();
  const [expanded, setExpanded] = useState(false);
  const visible = expanded ? items : items.slice(0, initialCount);
  const List = ordered ? "ol" : "ul";

  useEffect(() => {
    setExpanded(false);
  }, [items]);

  if (!items.length) return <p className="content-empty">{emptyLabel ?? t("common.noData")}</p>;
  const content = <div className={`bounded-list ${className}`.trim()}>
    <List>{visible.map((item, index) => <li key={`${index}-${item}`}><CollapsibleText collapsedLines={itemLines} expandedMaxHeight={expandedMaxHeight} value={item} /></li>)}</List>
    {items.length > initialCount ? <button aria-expanded={expanded} className="content-toggle" onClick={() => setExpanded((value) => !value)} type="button">{expanded ? t("common.collapse") : t("common.viewRemaining", { count: items.length - initialCount })}</button> : null}
  </div>;
  return exclusiveExpansion ? <ExclusiveExpansionGroup>{content}</ExclusiveExpansionGroup> : content;
}

export type RecordDisclosureProps = {
  recordId: string;
  expanded: boolean;
  onToggle: (recordId: string) => void;
  summary: ReactNode;
  children: ReactNode;
  className?: string;
};

export function RecordDisclosure({ recordId, expanded, onToggle, summary, children, className = "" }: RecordDisclosureProps) {
  const { t } = useTranslation();
  const detailId = useId();
  return <article className={`record-disclosure ${className}`.trim()} data-expanded={expanded}>
    <button aria-controls={detailId} aria-expanded={expanded} className="record-disclosure__trigger" onClick={() => onToggle(recordId)} type="button">
      <span>{summary}</span><b>{expanded ? t("common.collapse") : t("common.details")}</b>
    </button>
    {expanded ? <div className="record-disclosure__content" id={detailId}>{children}</div> : null}
  </article>;
}

export type CompactIdentifierProps = {
  value: string;
  copyable?: boolean;
  label?: string;
  className?: string;
};

export function CompactIdentifier({ value, copyable = true, label, className = "" }: CompactIdentifierProps) {
  const { t } = useTranslation();
  const [copied, setCopied] = useState(false);
  const accessibleLabel = label ?? t("common.copyIdentifier");
  async function copy() {
    await navigator.clipboard.writeText(value);
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1_500);
  }
  return <span className={`compact-identifier ${className}`.trim()}>
    {label ? <span className="compact-identifier__label">{label}</span> : null}
    <code title={value}>{value}</code>
    {copyable ? <button aria-label={label ? `${t("common.copy")} ${accessibleLabel}` : accessibleLabel} onClick={() => void copy()} type="button">{copied ? t("common.copied") : t("common.copy")}</button> : null}
  </span>;
}
