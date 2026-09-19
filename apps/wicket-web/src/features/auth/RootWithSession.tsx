import { useQueryClient } from "@tanstack/react-query";
import { useRouter } from "@tanstack/react-router";
import { useEffect } from "react";
import { RootLayout } from "../../modes/root-layout";
import {
  bindQueryClient,
  hydrateSessionFromCookie,
  setUnauthorizedNavigate,
} from "./session";

export function RootWithSession() {
  const queryClient = useQueryClient();
  const routerInstance = useRouter();

  hydrateSessionFromCookie();
  bindQueryClient(queryClient);
  setUnauthorizedNavigate(() => {
    const { pathname, searchStr } = routerInstance.state.location;
    if (pathname === "/login") {
      return;
    }
    void routerInstance.navigate({
      to: "/login",
      search: { next: `${pathname}${searchStr}` },
      replace: true,
    });
  });

  useEffect(() => {
    return () => {
      bindQueryClient(null);
      setUnauthorizedNavigate(null);
    };
  }, [queryClient, routerInstance]);

  return <RootLayout />;
}
