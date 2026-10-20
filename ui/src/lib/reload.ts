/** Muat ulang seluruh UI (setelah restore DB). Dipisah agar bisa di-stub di test. */
export function reloadApp(): void {
  window.location.reload()
}
