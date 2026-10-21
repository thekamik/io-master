// The parts of a group or a control as rows with their level
// (src/views/catalog_editor/part_rows.rs), shown without recursion.

export interface PartRowView {
  level: number;
  name: string;
  id: string | null;
  title: string | null;
  prose: string | null;
}
