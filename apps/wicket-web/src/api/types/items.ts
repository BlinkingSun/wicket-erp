/** Throwaway wire types for items (`ItemBody`, docs/10 snake_case). */

export type ItemBodyWire = {
  id: string;
  number: string;
  revision: string;
  description: string;
  kind: string;
  stock_uom: number;
  stock_scale: number;
  residual_tolerance: string;
  cost_method: string;
  status: string;
  version: number;
  application_version: string;
  configuration_version: string;
  created_at: string;
};

export type ItemPatchWire = {
  description?: string;
  revision?: string;
  kind?: string;
  stock_uom?: number;
  stock_scale?: number;
  residual_tolerance?: string;
  cost_method?: string;
};
