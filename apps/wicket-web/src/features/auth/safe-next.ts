export function safeNext(next: string | undefined): string {
  if (
    typeof next === "string" &&
    next.startsWith("/") &&
    !next.startsWith("//") &&
    !next.startsWith("/login")
  ) {
    return next;
  }
  return "/";
}
