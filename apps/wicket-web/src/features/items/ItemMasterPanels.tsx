import {
  UNBACKED_BOM,
  UNBACKED_REVISION_HISTORY,
} from "../../api/view-models";
import { BOM_COLUMN_HEADERS } from "./bom-columns";
import { NotYetAvailable } from "./NotYetAvailable";

export function RevisionStrip() {
  return (
    <div className="item-revision-strip" aria-label="Revision history">
      <NotYetAvailable capability={UNBACKED_REVISION_HISTORY} />
    </div>
  );
}

export function BomTabPanel() {
  return (
    <section className="item-tab-panel" aria-label="Bill of Material">
      <table className="item-data-table">
        <thead>
          <tr>
            {BOM_COLUMN_HEADERS.map((label) => (
              <th key={label} scope="col">
                {label}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          <tr>
            <td colSpan={BOM_COLUMN_HEADERS.length}>
              <NotYetAvailable capability={UNBACKED_BOM} />
            </td>
          </tr>
        </tbody>
      </table>
    </section>
  );
}
