export type Match = { start: number; end: number }

const specials = '^$\\.*+?()[]{}|'

export function findAll(text: string, term: string): Match[] {
  if (term.length === 0) return []
  const esc = [...term].map((c) => (specials.includes(c) ? `\\${c}` : c)).join('')
  const re = new RegExp(esc, 'giu')
  const matches: Match[] = []
  // Sin matchAll: el target es es2017.
  for (let m = re.exec(text); m; m = re.exec(text)) matches.push({ start: m.index, end: m.index + m[0].length })
  return matches
}

export function firstAtOrAfter(matches: Match[], from: number): number {
  if (matches.length === 0) return -1
  const i = matches.findIndex((m) => m.start >= from)
  return i === -1 ? 0 : i
}
