/** Throwaway wire type for GET /api/v1/identity/me (`getOwnProfile`). */

export type OwnProfileBodyWire = {
  id: string;
  principal_kind: string;
  username: string;
  display_name: string;
  status: string;
  created_at: string;
  deactivated_at: string | null;
};
