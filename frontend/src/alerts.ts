import type { ApiError } from "./http.js";

interface SweetAlert {
  fire(options: {
    title: string;
    text?: string;
    icon?: "success" | "error" | "warning" | "info";
    timer?: number;
    showConfirmButton?: boolean;
    showCancelButton?: boolean;
    confirmButtonText?: string;
    cancelButtonText?: string;
  }): Promise<{
    isConfirmed?: boolean;
  }>;
}

declare global {
  interface Window {
    Swal?: SweetAlert;
  }
}

export function showError(error: unknown): void {
  const apiError = isApiError(error) ? error : undefined;
  const message = apiError?.requestId
    ? `${apiError.message} ID: ${apiError.requestId}`
    : (apiError?.message ?? "Não foi possível concluir a operação.");

  show({
    title: "Ação não concluída",
    text: message,
    icon: "error",
  });
}

export function showSuccess(message: string): void {
  show({
    title: message,
    icon: "success",
    timer: 1800,
    showConfirmButton: false,
  });
}

export async function confirmAction(message: string): Promise<boolean> {
  if (window.Swal !== undefined) {
    const result = await window.Swal.fire({
      title: "Confirmar ação",
      text: message,
      icon: "warning",
      showCancelButton: true,
      confirmButtonText: "Confirmar",
      cancelButtonText: "Cancelar",
    });

    return result.isConfirmed === true;
  }

  return false;
}

function show(options: Parameters<SweetAlert["fire"]>[0]): void {
  if (window.Swal !== undefined) {
    void window.Swal.fire(options);
    return;
  }

  const liveRegion = document.querySelector<HTMLElement>("[data-live-region]");
  if (liveRegion !== null) {
    liveRegion.textContent = [options.title, options.text].filter(Boolean).join(". ");
  }
}

function isApiError(error: unknown): error is ApiError {
  return (
    typeof error === "object" &&
    error !== null &&
    "code" in error &&
    "message" in error &&
    "status" in error
  );
}
