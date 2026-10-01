export function jsonObject<T>(value: T, message: string): object {
  if (value === null || typeof value !== "object") {
    throw new Error(message);
  }
  return value as object;
}
