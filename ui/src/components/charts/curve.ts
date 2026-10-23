// Path SVG kurva kubik monoton (Fritsch–Carlson, seperti d3 `curveMonotoneX`): mulus tapi tidak
// melampaui nilai titik data, sehingga garis tidak "turun di bawah nol" atau memalsukan puncak.
export function monotonePath(pts: { x: number; y: number }[]): string {
  const n = pts.length
  if (n === 0) return ''
  if (n < 3) return pts.map((p, i) => `${i ? 'L' : 'M'}${p.x},${p.y}`).join(' ')
  const slopes = pts.slice(1).map((p, i) => (p.y - pts[i]!.y) / (p.x - pts[i]!.x || 1))
  const tangents = pts.map((_, i) => {
    if (i === 0) return slopes[0]!
    if (i === n - 1) return slopes[n - 2]!
    const a = slopes[i - 1]!
    const b = slopes[i]!
    return a * b <= 0 ? 0 : (2 * a * b) / (a + b)
  })
  let d = `M${pts[0]!.x},${pts[0]!.y}`
  for (let i = 0; i < n - 1; i++) {
    const p = pts[i]!
    const q = pts[i + 1]!
    const h = (q.x - p.x) / 3
    d += ` C${p.x + h},${p.y + tangents[i]! * h} ${q.x - h},${q.y - tangents[i + 1]! * h} ${q.x},${q.y}`
  }
  return d
}
