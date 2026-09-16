const apiBase = import.meta.env.VITE_API_BASE ?? "";

export class ApiError extends Error {
  readonly status: number;

  constructor(status: number, message: string) {
    super(message);
    this.name = "ApiError";
    this.status = status;
  }
}

async function readJsonResponse(
  response: Response,
  path: string,
  method: string,
): Promise<unknown> {
  if (!response.ok) {
    throw new ApiError(
      response.status,
      `Request failed (${response.status}) for ${method} ${path}`,
    );
  }
  return response.json();
}

export async function getJson(path: string): Promise<unknown> {
  const response = await fetch(`${apiBase}${path}`, {
    method: "GET",
    credentials: "include",
    headers: { Accept: "application/json" },
  });
  return readJsonResponse(response, path, "GET");
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
      ...headers,
    },
    body: JSON.stringify(body),
  });
  return readJsonResponse(response, path, "PATCH");
}
