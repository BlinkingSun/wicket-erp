import "@testing-library/jest-dom/vitest";
import { beforeEach } from "vitest";
import "../styles/global.css";
import { setSession } from "../features/auth/session";

beforeEach(() => {
  setSession({ csrf: "test-csrf", displayName: "Test Operator" });
});
