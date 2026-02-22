import { createSignal } from "solid-js";

const [message, setMessage] = createSignal<string | null>(null);
let _timer: ReturnType<typeof setTimeout> | undefined;

export const toastMessage = message;

export function showToast(msg: string, durationMs = 2000): void {
  clearTimeout(_timer);
  setMessage(msg);
  _timer = setTimeout(() => setMessage(null), durationMs);
}
