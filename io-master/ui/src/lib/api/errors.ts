// The errors of the commands (src/views/error.rs), and the sentence to show
// for one. The type mirrors the error of the program; a change there needs
// the same change here.

/** An error of a command (src/views/error.rs). */
export interface ViewError {
  kind: "not_installed" | "already_installed" | "wrong_credentials" | "invalid" | "failed";
  message: string;
}

/** Whether a rejected call carries an error of a command. */
export function isViewError(error: unknown): error is ViewError {
  return typeof error === "object" && error !== null && "kind" in error && "message" in error;
}

/**
 * The sentence to show for a rejected call. The messages of the program are
 * written in lower case without a full stop, so that they can be put inside
 * other sentences; shown alone, they get both.
 */
export function sentence(error: unknown): string {
  const text = isViewError(error) ? error.message : String(error);
  const capital = text.charAt(0).toUpperCase() + text.slice(1);

  return /[.!?]$/.test(capital) ? capital : `${capital}.`;
}
