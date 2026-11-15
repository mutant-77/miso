import type { Prefs } from './ipc/types/Prefs'
import en from '../../locales/en.json'
import es from '../../locales/es.json'
import pt from '../../locales/pt.json'
import de from '../../locales/de.json'
import it from '../../locales/it.json'
import fr from '../../locales/fr.json'
import nl from '../../locales/nl.json'
import ru from '../../locales/ru.json'
import nb from '../../locales/nb.json'
import sv from '../../locales/sv.json'
import fi from '../../locales/fi.json'

export type Key = keyof typeof en

const catalogs: Record<string, Record<string, string>> = { en, es, pt, de, it, fr, nl, ru, nb, sv, fi }

// Cada idioma por su nombre propio, sin traducir.
export const LANGUAGES: [string, string][] = [
  ['en', 'English'],
  ['es', 'Español'],
  ['pt', 'Português'],
  ['de', 'Deutsch'],
  ['it', 'Italiano'],
  ['fr', 'Français'],
  ['nl', 'Nederlands'],
  ['ru', 'Русский'],
  ['nb', 'Norsk bokmål'],
  ['sv', 'Svenska'],
  ['fi', 'Suomi'],
]

// Inyectadas por Rust en create_reader; __SYSTEM_LANG__ ya es un código soportado.
const injected = globalThis as { __PREFS__?: Prefs; __SYSTEM_LANG__?: string }

const own = (o: object, key: string) => Object.prototype.hasOwnProperty.call(o, key)
const known = (code: string | undefined): code is string => code !== undefined && own(catalogs, code)

/** Misma regla que `i18n::effective` en Rust. */
export function effective(language: string, system = injected.__SYSTEM_LANG__): string {
  if (known(language)) return language
  return known(system) ? system : 'en'
}

export const i18n = $state({ lang: effective(injected.__PREFS__?.language ?? 'auto') })

export function setLang(code: string) {
  i18n.lang = code
  if (typeof document !== 'undefined') document.documentElement.lang = code
}

export function format(template: string, vars: Record<string, string | number>): string {
  return template.replace(/\{(\w+)\}/g, (match, name: string) => (own(vars, name) ? String(vars[name]) : match))
}

export function t(key: Key, vars?: Record<string, string | number>): string {
  const text = catalogs[i18n.lang]?.[key] ?? en[key] ?? key
  return vars ? format(text, vars) : text
}

setLang(i18n.lang)
