import type { TFunction } from "i18next";

export interface LocalizedAppError {
  title: string;
  guidance?: string;
  technicalCode: string;
  technicalMessage?: string;
  retryable: boolean;
}

function errorShape(error: unknown): { code: string; message?: string; retryable: boolean } {
  if (error && typeof error === "object") {
    return {
      code: "code" in error ? String(error.code) : "unknown_error",
      message: "message" in error ? String(error.message) : undefined,
      retryable: "retryable" in error ? Boolean(error.retryable) : false,
    };
  }
  return { code: "unknown_error", message: error instanceof Error ? error.message : typeof error === "string" ? error : undefined, retryable: false };
}

export function localizeAppError(error: unknown, t: TFunction): LocalizedAppError {
  const shape = errorShape(error);
  const titleKey = `errors.${shape.code}`;
  const guidanceKey = `errors.${shape.code}_guidance`;
  const translatedTitle = t(titleKey);
  const translatedGuidance = t(guidanceKey);
  return {
    title: translatedTitle === titleKey ? t("errors.unknownTitle") : translatedTitle,
    guidance: translatedGuidance === guidanceKey ? t("errors.unknownGuidance") : translatedGuidance,
    technicalCode: shape.code,
    technicalMessage: shape.message,
    retryable: shape.retryable,
  };
}

export function localizedErrorText(error: unknown, t: TFunction): string {
  const value = localizeAppError(error, t);
  return value.guidance ? `${value.title}。${value.guidance}` : value.title;
}
