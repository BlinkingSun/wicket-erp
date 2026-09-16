/** Throwaway wire type for GET /api/v1/work-orders/{id} (`getWorkOrder`, wo_json). */

export type QuantityBodyWire = {
  amount: string;
  unit: number;
  dimension: string;
};

export type WorkOrderBodyWire = {
  id: string;
  number: string | null;
  item_id: string;
  quantity: QuantityBodyWire;
  status: string;
  revision: string;
  wip_location_id: string | null;
  version: number;
  application_version: string;
  configuration_version: string;
  released_at: string | null;
  completed_at: string | null;
};
