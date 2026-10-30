/** A failed command shows nothing in the MVP; in development its code reaches the console, never its message. */
export function reportFailure(command: string, error: unknown): void {
  if (!import.meta.env.DEV) return;
  const code = typeof error === "object" && error !== null && "code" in error ? String(error.code) : "unknown";
  console.error(`${command} failed: ${code}`);
}
