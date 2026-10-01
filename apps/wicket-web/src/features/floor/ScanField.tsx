import { useNavigate } from "@tanstack/react-router";
import { useState, type FormEvent } from "react";
import { SCAN_LOOKUP_UNAVAILABLE } from "./unbacked";

export { SCAN_LOOKUP_UNAVAILABLE };

export const TRAVELER_SCAN_MESSAGE =
  "A work-order UUID opens the job. Scanning a work-order number or traveler does not work yet.";

const WORK_ORDER_UUID =
  /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

function isWorkOrderUuid(raw: string): boolean {
  return WORK_ORDER_UUID.test(raw);
}

function scanReceivedMessage(raw: string): string {
  const identifier = raw.trim();
  if (identifier.length === 0) {
    return `Received no identifier. ${SCAN_LOOKUP_UNAVAILABLE}`;
  }
  return `Received "${identifier}". ${SCAN_LOOKUP_UNAVAILABLE}`;
}

type ScanFieldProps = {
  compact?: boolean;
};

export function ScanField({ compact = false }: ScanFieldProps) {
  const navigate = useNavigate();
  const [value, setValue] = useState("");
  const [message, setMessage] = useState<string | null>(null);
  const [visibleMessage, setVisibleMessage] = useState<string | null>(null);

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const identifier = value.trim();
    if (isWorkOrderUuid(identifier)) {
      setVisibleMessage(null);
      setMessage(null);
      void navigate({
        to: "/floor/work-orders/$workOrderId",
        params: { workOrderId: identifier.toLowerCase() },
      });
      return;
    }
    setMessage(scanReceivedMessage(value));
    setVisibleMessage(TRAVELER_SCAN_MESSAGE);
  }

  const scanClass = compact ? "floor-scan floor-scan--compact" : "floor-scan";

  return (
    <form className={scanClass} onSubmit={handleSubmit} role="search">
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
      {visibleMessage ? (
        <p className="floor-scan__message" role="alert">
          {visibleMessage}
        </p>
      ) : null}
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
