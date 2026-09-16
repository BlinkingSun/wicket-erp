export function truncateUuid(uuid: string): string {
  if (uuid.length < 13) {
    return uuid;
  }
  return `${uuid.slice(0, 8)}…${uuid.slice(-4)}`;
}
