import assert from "node:assert/strict";
import { afterEach, beforeEach, test } from "node:test";

import { HttpClient, type ApiError } from "../src/http.js";

type FetchCall = {
  url: string;
  init: RequestInit;
};

let fetchCalls: FetchCall[] = [];

beforeEach(() => {
  fetchCalls = [];
  setWindow();
  setDocumentCookie("investment_csrf=csrf-token");
});

afterEach(() => {
  deleteGlobal("fetch");
  deleteGlobal("window");
  deleteGlobal("document");
});

test("HttpClient sends JSON requests with CSRF and parses success responses", async () => {
  mockFetch(async (url, init) => {
    fetchCalls.push({ url: String(url), init });
    return jsonResponse(201, { id: "asset-1" });
  });

  const result = await new HttpClient().post("/api/assets", { symbol: "PETR4" });

  assert.deepEqual(result, { id: "asset-1" });
  assert.equal(fetchCalls[0]?.url, "/api/assets");
  assert.equal(fetchCalls[0]?.init.method, "POST");
  assert.equal(headerValue(fetchCalls[0]?.init.headers, "x-csrf-token"), "csrf-token");
  assert.equal(headerValue(fetchCalls[0]?.init.headers, "content-type"), "application/json");
  assert.equal(fetchCalls[0]?.init.body, JSON.stringify({ symbol: "PETR4" }));
});

test("HttpClient omits mutation headers on GET requests", async () => {
  mockFetch(async (url, init) => {
    fetchCalls.push({ url: String(url), init });
    return jsonResponse(200, { assets: [] });
  });

  await new HttpClient().get("/api/assets");

  assert.equal(fetchCalls[0]?.init.method, "GET");
  assert.equal(headerValue(fetchCalls[0]?.init.headers, "content-type"), undefined);
  assert.equal(headerValue(fetchCalls[0]?.init.headers, "x-csrf-token"), undefined);
});

test("HttpClient maps validation errors and request id from JSON response", async () => {
  mockFetch(async () => jsonResponse(422, { error: { code: "validation_error", message: "campo inválido" } }, "req-1"));

  const error = await rejectsApiError(new HttpClient().post("/api/assets", {}));

  assert.equal(error.status, 422);
  assert.equal(error.code, "validation_error");
  assert.equal(error.message, "campo inválido");
  assert.equal(error.requestId, "req-1");
});

test("HttpClient maps authentication, authorization, rate limit and server errors", async () => {
  for (const [status, code] of [
    [401, "unauthorized"],
    [403, "forbidden"],
    [429, "rate_limited"],
    [500, "server_error"],
  ] as const) {
    mockFetch(async () => new Response("", { status }));
    const error = await rejectsApiError(new HttpClient().get("/api/private"));

    assert.equal(error.status, status);
    assert.equal(error.code, code);
  }
});

test("HttpClient maps request timeout to stable ApiError", async () => {
  mockFetch(async (_url, init) => {
    await new Promise((_resolve, reject) => {
      init.signal?.addEventListener("abort", () => reject(new DOMException("aborted", "AbortError")));
    });
    throw new Error("unreachable");
  });

  const error = await rejectsApiError(new HttpClient(1).get("/api/slow"));

  assert.equal(error.status, 0);
  assert.equal(error.code, "timeout");
});

test("HttpClient maps external cancellation to stable ApiError", async () => {
  mockFetch(async (_url, init) => {
    await new Promise((_resolve, reject) => {
      init.signal?.addEventListener("abort", () => reject(new DOMException("aborted", "AbortError")));
    });
    throw new Error("unreachable");
  });
  const controller = new AbortController();
  const request = new HttpClient(10_000).get("/api/cancel", controller.signal);

  controller.abort();
  const error = await rejectsApiError(request);

  assert.equal(error.status, 0);
  assert.equal(error.code, "timeout");
});

function mockFetch(handler: (url: URL | RequestInfo, init: RequestInit) => Promise<Response>): void {
  Object.defineProperty(globalThis, "fetch", {
    configurable: true,
    value: (url: URL | RequestInfo, init?: RequestInit) => handler(url, init ?? {}),
  });
}

function jsonResponse(status: number, body: unknown, requestId?: string): Response {
  const headers = new Headers({ "content-type": "application/json" });
  if (requestId !== undefined) headers.set("x-request-id", requestId);
  return new Response(JSON.stringify(body), { status, headers });
}

async function rejectsApiError(promise: Promise<unknown>): Promise<ApiError> {
  try {
    await promise;
  } catch (error) {
    return error as ApiError;
  }
  throw new Error("expected promise to reject");
}

function headerValue(headers: HeadersInit | undefined, name: string): string | undefined {
  if (headers === undefined) return undefined;
  return new Headers(headers).get(name) ?? undefined;
}

function setWindow(): void {
  Object.defineProperty(globalThis, "window", {
    configurable: true,
    value: {
      setTimeout,
      clearTimeout,
    },
  });
}

function setDocumentCookie(cookie: string): void {
  Object.defineProperty(globalThis, "document", {
    configurable: true,
    value: {
      cookie,
      querySelector: () => null,
    },
  });
}

function deleteGlobal(name: "document" | "fetch" | "window"): void {
  Reflect.deleteProperty(globalThis, name);
}
