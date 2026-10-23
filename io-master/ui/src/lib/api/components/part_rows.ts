// The component "parts of an object": the parts of a group or a control as
// rows with their level (src/components/part_rows/view.rs), shown without
// recursion.

export interface PartRowView {
  level: number;
  name: string;
  id: string | null;
  title: string | null;
  prose: string | null;
}
