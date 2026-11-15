import { hyphenateSync as en } from 'hyphen/en'
import { hyphenateSync as es } from 'hyphen/es'
import type { Doc } from '../ipc/types/Doc'
import { runsOf } from './textIndex'

/** Guion blando: sólo existe en el DOM; los índices de carácter no lo cuentan (anchor.ts). */
export const SHY = '­'

const hyphenators = { en, es }
export type Lang = keyof typeof hyphenators

const stopwords: Record<Lang, Set<string>> = {
  en: new Set('the and of to in that was he she it his her with for as had you not but is'.split(' ')),
  es: new Set('de la que el en y los las se del un una por con su al lo como más pero es'.split(' ')),
}

const SAMPLE = 5000
const MIN_HITS = 10

/** Idioma por palabras frecuentes; null si no hay muestra o no es claro (no se parte). */
export function detectLang(doc: Doc): Lang | null {
  const hits: Record<Lang, number> = { en: 0, es: 0 }
  let words = 0
  for (const block of doc.blocks) {
    for (const run of runsOf(block)) {
      for (const w of run.text.toLowerCase().match(/[a-záéíóúüñ]+/g) ?? []) {
        if (stopwords.en.has(w)) hits.en++
        if (stopwords.es.has(w)) hits.es++
        if (++words >= SAMPLE) return pick(hits)
      }
    }
  }
  return pick(hits)
}

function pick(hits: Record<Lang, number>): Lang | null {
  const [best, other]: Lang[] = hits.en >= hits.es ? ['en', 'es'] : ['es', 'en']
  return hits[best] >= MIN_HITS && hits[best] >= 2 * hits[other] ? best : null
}

export function hyphenate(text: string, lang: Lang | null): string {
  return lang ? hyphenators[lang](text, { minWordLength: 6 }) : text
}
