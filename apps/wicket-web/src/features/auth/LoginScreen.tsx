import { useRouter } from "@tanstack/react-router";
import { useState, type FormEvent } from "react";
import { login } from "../../api/client";
import { ApiError } from "../../api/http";
import "./auth.css";
import { safeNext } from "./safe-next";
import { setSession } from "./session";

type LoginScreenProps = {
  next?: string;
};

export function LoginScreen({ next }: LoginScreenProps) {
  const router = useRouter();
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState(false);

  async function onSubmit(event: FormEvent<HTMLFormElement>): Promise<void> {
    event.preventDefault();
    setError(null);
    setPending(true);
    try {
      const result = await login({ username, password });
      setSession({ csrf: result.csrf, displayName: result.displayName });
      router.history.push(safeNext(next));
    } catch (err) {
      if (err instanceof ApiError) {
        setError(err.message);
      } else if (err instanceof Error) {
        setError(err.message);
      } else {
        setError("The engine refused this sign-in.");
      }
    } finally {
      setPending(false);
    }
  }

  return (
    <div className="login-page">
      <section className="login-card">
        <h1>Sign in</h1>
        <p className="login-card__intro">
          A session is required before the shop can be opened. Sign in with your
          principal.
        </p>
        <form onSubmit={onSubmit} aria-busy={pending}>
          <div className="login-field">
            <label htmlFor="login-username">Username</label>
            <input
              id="login-username"
              name="username"
              autoComplete="username"
              value={username}
              onChange={(event) => setUsername(event.target.value)}
              required
            />
          </div>
          <div className="login-field">
            <label htmlFor="login-password">Password</label>
            <input
              id="login-password"
              name="password"
              type="password"
              autoComplete="current-password"
              value={password}
              onChange={(event) => setPassword(event.target.value)}
              required
            />
          </div>
          {error ? (
            <p className="login-error" role="alert">
              {error}
            </p>
          ) : null}
          <div className="login-actions">
            <button className="login-submit" type="submit" disabled={pending}>
              Sign in
            </button>
          </div>
        </form>
      </section>
    </div>
  );
}
