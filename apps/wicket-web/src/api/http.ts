const apiBase = import.meta.env.VITE_API_BASE ?? "";

export class ApiError extends Error {
  readonly status: number;

  constructor(status: number, message: string) {
    super(message);
    this.name = "ApiError";
    this.status = status;
  }
}

export async function getJson(path: string): Promise<unknown> {
  const response = await fetch(`${apiBase}${path}`, {
    method: "GET",
    credentials: "include",
    headers: { Accept: "application/json" },
  });
  if (!response.ok) {
    throw new ApiError(
      response.status,
      `Request failed (${response.status}) for ${path}`,
    );
  }
  return response.json();
}
