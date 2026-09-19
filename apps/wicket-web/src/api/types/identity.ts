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

/** Throwaway wire type for POST /api/v1/identity/login. */

export type LoginBodyWire = {
  session_id: string;
  principal_id: string;
  display_name: string;
  csrf: string;
};

/** Throwaway wire type for GET /api/v1/navigation. */

export type NavigationBodyWire = {
  visible: string[];
  hidden: string[];
};
