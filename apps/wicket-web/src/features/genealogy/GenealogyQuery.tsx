import { useState, type FormEvent } from "react";

const UUID_RE =
  /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

export const HUMAN_ID_NOTE =
  "This field accepts a lot UUID. Human-identifier entry is not yet available.";

type GenealogyQueryProps = {
  initialLotId: string;
  onTrace: (lotId: string) => void;
};

export function GenealogyQuery({ initialLotId, onTrace }: GenealogyQueryProps) {
  const [value, setValue] = useState(initialLotId);
  const [error, setError] = useState<string | null>(null);

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const lotId = value.trim();
    if (!UUID_RE.test(lotId)) {
      setError("Enter a lot UUID (xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx).");
      return;
    }
    setError(null);
    onTrace(lotId);
  }

  return (
    <div className="query-block">
      <form className="query-chrome" role="search" onSubmit={handleSubmit}>
        <label className="query-chrome__label" htmlFor="lot-uuid-query">
          Lot UUID query
        </label>
        <input
          id="lot-uuid-query"
          className="query-chrome__input"
          name="from_lot_id"
          value={value}
          onChange={(event) => setValue(event.target.value)}
          placeholder="01932c5a-8b10-7001-8000-000000000003"
          autoComplete="off"
          spellCheck={false}
          aria-describedby="lot-uuid-note"
        />
        <button className="query-chrome__submit" type="submit" aria-label="Run genealogy trace">
          <SearchIcon />
        </button>
      </form>
      <p id="lot-uuid-note" className="query-note">
        {HUMAN_ID_NOTE}
      </p>
      {error ? <p className="query-error">{error}</p> : null}
    </div>
  );
}

function SearchIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 14 14" aria-hidden="true">
      <circle cx="6" cy="6" r="4.25" fill="none" stroke="currentColor" strokeWidth="1.5" />
      <path d="M9.2 9.2 L12 12" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
    </svg>
  );
}
