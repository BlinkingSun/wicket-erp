import type { UnbackedCapability } from "../../api/view-models";

type NotYetAvailableProps = {
  capability: UnbackedCapability;
};

export function NotYetAvailable({ capability }: NotYetAvailableProps) {
  return (
    <div className="item-empty" role="status">
      <p className="item-empty__title">Not yet available</p>
      <p className="item-empty__detail">
        {capability.label} requires operation{" "}
        <span className="mono">{capability.operationId}</span>, which is not
        mounted on the engine yet.
      </p>
    </div>
  );
}
