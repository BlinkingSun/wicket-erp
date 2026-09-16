import type { UnbackedCapability } from "../../api/view-models";
import { UNBACKED_FLOOR } from "../../api/view-models";

export function unbackedReason(capability: UnbackedCapability): string {
  return `${capability.label} requires operation ${capability.operationId}, which is not mounted on the engine yet.`;
}

export const SCAN_LOOKUP_UNAVAILABLE = unbackedReason(
  UNBACKED_FLOOR.identifierLookup,
);

export const REPORT_QTY_REASON = `${unbackedReason(UNBACKED_FLOOR.reportQuantity)} completeWorkOrder accepts quantity but completes the whole work order (quantity, location_id, If-Match).`;

export const REPORT_SCRAP_REASON = `${unbackedReason(UNBACKED_FLOOR.reportScrap)} Scrap on completeWorkOrder is a field of terminal complete, not shop-floor scrap reporting.`;

export const GAGE_REASON = `${unbackedReason(UNBACKED_FLOOR.gage)} approveCalibration is an approval, not current-cal status.`;
