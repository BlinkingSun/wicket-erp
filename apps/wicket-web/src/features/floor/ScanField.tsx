import { useState, type FormEvent } from "react";
import { SCAN_LOOKUP_UNAVAILABLE } from "./unbacked";

export { SCAN_LOOKUP_UNAVAILABLE };

function scanReceivedMessage(raw: string): string {
  const identifier = raw.trim();
  if (identifier.length === 0) {
    return `Received no identifier. ${SCAN_LOOKUP_UNAVAILABLE}`;
  }
  return `Received "${identifier}". ${SCAN_LOOKUP_UNAVAILABLE}`;
}

export function ScanField() {
  const [value, setValue] = useState("");
  const [message, setMessage] = useState<string | null>(null);

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setMessage(scanReceivedMessage(value));
  }

  return (
    <form className="floor-scan" onSubmit={handleSubmit} role="search">
      <div className="floor-scan__outer">
        <button className="floor-scan__tap" type="submit">
          TAP OR SCAN
        </button>
        <div className="floor-scan__frame">
          <div className="floor-scan__glyph" aria-hidden="true">
            <BarcodeGlyph />
          </div>
          <input
            id="floor-scan-input"
            className="floor-scan__input"
            name="identifier"
            value={value}
            onChange={(event) => setValue(event.target.value)}
            placeholder="Scan traveler or badge"
            autoComplete="off"
            spellCheck={false}
            autoFocus
            aria-label="Scan traveler or badge"
            title={SCAN_LOOKUP_UNAVAILABLE}
            aria-describedby="floor-scan-note"
          />
        </div>
      </div>
      <p id="floor-scan-note" className="floor-sr-only" role="status">
        {message ?? SCAN_LOOKUP_UNAVAILABLE}
      </p>
    </form>
  );
}

function BarcodeGlyph() {
  return (
    <svg
      className="floor-scan__barcode"
      viewBox="0 0 88 48"
      width="88"
      height="48"
      aria-hidden="true"
    >
      <rect x="0" y="4" width="3" height="40" fill="currentColor" />
      <rect x="6" y="4" width="2" height="40" fill="currentColor" />
      <rect x="11" y="4" width="5" height="40" fill="currentColor" />
      <rect x="18" y="4" width="2" height="40" fill="currentColor" />
      <rect x="23" y="4" width="3" height="40" fill="currentColor" />
      <rect x="29" y="4" width="6" height="40" fill="currentColor" />
      <rect x="38" y="4" width="2" height="40" fill="currentColor" />
      <rect x="43" y="4" width="4" height="40" fill="currentColor" />
      <rect x="50" y="4" width="2" height="40" fill="currentColor" />
      <rect x="55" y="4" width="7" height="40" fill="currentColor" />
      <rect x="65" y="4" width="2" height="40" fill="currentColor" />
      <rect x="70" y="4" width="3" height="40" fill="currentColor" />
      <rect x="76" y="4" width="2" height="40" fill="currentColor" />
      <rect x="81" y="4" width="5" height="40" fill="currentColor" />
    </svg>
  );
}
