// The data of the component "recovery code" (src/components/recovery_code/view.rs):
// it has no command of its own; the commands of the view Users that make a
// code give it (users.ts).

/** A recovery code, shown once (src/components/recovery_code/view.rs). */
export interface RecoveryCodeView {
  /** Seven groups of five characters joined by dashes. */
  code: string;
}
