export type HttpMethod = "GET" | "POST" | "PATCH";

export interface ApiError {
  code: string;
  message: string;
  requestId?: string;
  status: number;
}

interface ErrorBody {
  error?: {
    code?: string;
    message?: string;
  };
}

export class HttpClient {
  constructor(private readonly timeoutMs = 8_000) {}

  get<TResponse>(url: string, signal?: AbortSignal): Promise<TResponse> {
    return this.request<TResponse>("GET", url, undefined, signal);
  }

  post<TRequest, TResponse>(
    url: string,
    body: TRequest,
    signal?: AbortSignal,
  ): Promise<TResponse> {
    return this.request<TResponse>("POST", url, body, signal);
  }

  patch<TRequest, TResponse>(
    url: string,
    body: TRequest,
    signal?: AbortSignal,
  ): Promise<TResponse> {
    return this.request<TResponse>("PATCH", url, body, signal);
  }

  private async request<TResponse>(
    method: HttpMethod,
    url: string,
    body?: unknown,
    externalSignal?: AbortSignal,
  ): Promise<TResponse> {
    const controller = new AbortController();
    const timeout = window.setTimeout(() => controller.abort(), this.timeoutMs);
    const forwardAbort = () => controller.abort();
    externalSignal?.addEventListener("abort", forwardAbort, { once: true });

    try {
      const request: RequestInit = {
        method,
        credentials: "same-origin",
        headers: this.headers(method),
        signal: controller.signal,
      };
      if (body !== undefined) {
        request.body = JSON.stringify(body);
      }

      const response = await fetch(url, request);

      if (!response.ok) {
        throw await this.toApiError(response);
      }

      if (response.status === 204) {
        return undefined as TResponse;
      }

      return (await response.json()) as TResponse;
    } catch (error) {
      if (error instanceof DOMException && error.name === "AbortError") {
        throw {
          code: "timeout",
          message: "Tempo de resposta esgotado. Tente novamente.",
          status: 0,
        } satisfies ApiError;
      }
      throw error;
    } finally {
      window.clearTimeout(timeout);
      externalSignal?.removeEventListener("abort", forwardAbort);
    }
  }

  private headers(method: HttpMethod): HeadersInit {
    const headers: Record<string, string> = {
      accept: "application/json",
    };

    if (method !== "GET") {
      headers["content-type"] = "application/json";
      const csrf = readCookie("investment_csrf");
      if (csrf !== null) {
        headers["x-csrf-token"] = csrf;
      }
    }

    return headers;
  }

  private async toApiError(response: Response): Promise<ApiError> {
    const requestId = response.headers.get("x-request-id") ?? undefined;
    const fallback = defaultMessage(response.status);
    const contentType = response.headers.get("content-type") ?? "";

    if (!contentType.includes("application/json")) {
      return withOptionalRequestId({
        code: fallback.code,
        message: fallback.message,
        status: response.status,
      }, requestId);
    }

    const body = (await response.json().catch(() => undefined)) as ErrorBody | undefined;
    return withOptionalRequestId({
      code: body?.error?.code ?? fallback.code,
      message: body?.error?.message ?? fallback.message,
      status: response.status,
    }, requestId);
  }
}

function readCookie(name: string): string | null {
  const prefix = `${name}=`;
  const value = document.cookie
    .split(";")
    .map((item) => item.trim())
    .find((item) => item.startsWith(prefix));

  return value === undefined ? null : decodeURIComponent(value.slice(prefix.length));
}

function defaultMessage(status: number): Pick<ApiError, "code" | "message"> {
  if (status === 401) return { code: "unauthorized", message: "Faça login novamente." };
  if (status === 403) return { code: "forbidden", message: "Acesso negado." };
  if (status === 429) return { code: "rate_limited", message: "Muitas tentativas. Aguarde." };
  if (status >= 500) return { code: "server_error", message: "Falha temporária no servidor." };
  return { code: "request_error", message: "Revise os dados enviados." };
}

function withOptionalRequestId(error: Omit<ApiError, "requestId">, requestId: string | undefined): ApiError {
  return requestId === undefined ? error : { ...error, requestId };
}
