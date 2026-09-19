import { expireUnauthorizedSession, getCsrfToken } from "../features/auth/session";

const apiBase = import.meta.env.VITE_API_BASE ?? "";

export class ApiError extends Error {
  readonly status: number;
  readonly code: string | null;
  readonly field: string | null;
  readonly requestId: string | null;

  constructor(
    status: number,
    message: string,
    details: {
      code?: string | null;
      field?: string | null;
      requestId?: string | null;
    } = {},
  ) {
    super(message);
    this.name = "ApiError";
    this.status = status;
    this.code = details.code ?? null;
    this.field = details.field ?? null;
    this.requestId = details.requestId ?? null;
  }
}

export type HttpCallOptions = {
  skipUnauthorized?: boolean;
  headers?: Record<string, string>;
};

function csrfHeaders(): Record<string, string> {
  const csrf = getCsrfToken();
  if (!csrf) {
    return {};
  }
  return { "X-CSRF-Token": csrf };
}

function idempotencyKey(): string {
  return crypto.randomUUID();
}

function parseErrorEnvelope(payload: unknown): {
  code: string | null;
  message: string | null;
  field: string | null;
  requestId: string | null;
} {
  if (payload === null || typeof payload !== "object" || !("error" in payload)) {
    return { code: null, message: null, field: null, requestId: null };
  }
  const envelope = (payload as { error: unknown }).error;
  if (envelope === null || typeof envelope !== "object") {
    return { code: null, message: null, field: null, requestId: null };
  }
  const error = envelope as {
    code?: unknown;
    message?: unknown;
    field?: unknown;
    request_id?: unknown;
  };
  return {
    code: typeof error.code === "string" ? error.code : null,
    message: typeof error.message === "string" ? error.message : null,
    field: typeof error.field === "string" ? error.field : null,
    requestId: typeof error.request_id === "string" ? error.request_id : null,
  };
}

async function readJsonResponse(
  response: Response,
  path: string,
  method: string,
  skipUnauthorized: boolean,
): Promise<unknown> {
  if (!response.ok) {
    const fallback = `Request failed (${response.status}) for ${method} ${path}`;
    let parsed: unknown;
    try {
      const text = await response.text();
      parsed = text ? JSON.parse(text) : null;
    } catch {
      parsed = null;
    }
    const envelope = parseErrorEnvelope(parsed);
    if (response.status === 401 && !skipUnauthorized) {
      expireUnauthorizedSession();
    }
    throw new ApiError(response.status, envelope.message ?? fallback, {
      code: envelope.code,
      field: envelope.field,
      requestId: envelope.requestId,
    });
  }
  if (response.status === 204) {
    return undefined;
  }
  const text = await response.text();
  if (!text) {
    return undefined;
  }
  return JSON.parse(text) as unknown;
}

export async function getJson(
  path: string,
  options: HttpCallOptions = {},
): Promise<unknown> {
  const response = await fetch(`${apiBase}${path}`, {
    method: "GET",
    credentials: "include",
    headers: { Accept: "application/json", ...options.headers },
  });
  return readJsonResponse(
    response,
    path,
    "GET",
    options.skipUnauthorized === true,
  );
}

export async function postJson(
  path: string,
  body: unknown,
  options: HttpCallOptions = {},
): Promise<unknown> {
  const response = await fetch(`${apiBase}${path}`, {
    method: "POST",
    credentials: "include",
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json",
      "Idempotency-Key": idempotencyKey(),
      ...csrfHeaders(),
      ...options.headers,
    },
    body: JSON.stringify(body ?? {}),
  });
  return readJsonResponse(
    response,
    path,
    "POST",
    options.skipUnauthorized === true,
  );
}

export async function patchJson(
  path: string,
  body: unknown,
  headers: Record<string, string>,
): Promise<unknown> {
  const response = await fetch(`${apiBase}${path}`, {
    method: "PATCH",
    credentials: "include",
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json",
      ...csrfHeaders(),
      ...headers,
    },
    body: JSON.stringify(body),
  });
  return readJsonResponse(response, path, "PATCH", false);
}

export async function putJson(
  path: string,
  body: unknown,
  options: HttpCallOptions = {},
): Promise<unknown> {
  const response = await fetch(`${apiBase}${path}`, {
    method: "PUT",
    credentials: "include",
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json",
      "Idempotency-Key": idempotencyKey(),
      ...csrfHeaders(),
      ...options.headers,
    },
    body: JSON.stringify(body ?? {}),
  });
  return readJsonResponse(
    response,
    path,
    "PUT",
    options.skipUnauthorized === true,
  );
}
