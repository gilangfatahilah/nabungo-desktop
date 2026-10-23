// Tick sumbu Y bersama LineChart/BarChart (docs/design/design.md §8.8). Nilai chart selalu integer (uang satuan
// minor, basis point), jadi tick dibulatkan dan duplikat dibuang: rentang sempit (mis. 0–1) tidak menghasilkan
// label kembar "0, 0, 1, 1, 1". Tanpa data (semua nol) hanya garis 0.
export function yTicks(min: number, max: number, count: number, empty: boolean): number[] {
  if (empty) return [0]
  const ticks = Array.from({ length: count + 1 }, (_, i) => Math.round(min + ((max - min) * i) / count))
  return [...new Set(ticks)]
}
