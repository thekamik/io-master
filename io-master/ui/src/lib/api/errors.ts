// The errors of the commands (serializer/error_view.rs), and the sentence
// to show for one.
//
// The types mirror the views of src/app/serializer/; a change
// there needs the same change here.

/** An error of a command (serializer/error_view.rs). */
export interface AppError {
  kind:
    | "not_installed"
    | "already_installed"
    | "not_logged_in"
    | "wrong_credentials"
    | "invalid"
    | "ai_unavailable"
    | "ai_refused"
    | "failed";
  message: string;
  /** For a field of an AI connection that breaks a rule: its name. */
  field: string | null;
}

/** Whether a rejected call carries an error of a command. */
export function isAppError(error: unknown): error is AppError {
  return typeof error === "object" && error !== null && "kind" in error && "message" in error;
}

/**
 * The sentence to show for a rejected call. The messages of the
 * program are written in lower case without a full stop, so that
 * they can be put inside other sentences; shown alone, they get
 * both.
 */
export function sentence(error: unknown): string {
  const text = isAppError(error) ? error.message : String(error);
  const capital = text.charAt(0).toUpperCase() + text.slice(1);

  return /[.!?]$/.test(capital) ? capital : `${capital}.`;
}
