import { describe, expect, it } from 'vitest'
import { LANGUAGES, effective, format, i18n, setLang, t } from './i18n.svelte'

describe('i18n', () => {
  it('effective respeta el código y "auto" va al sistema o al inglés', () => {
    expect(effective('ru', 'es')).toBe('ru')
    expect(effective('auto', 'es')).toBe('es')
    expect(effective('auto', undefined)).toBe('en')
    expect(effective('auto', 'xx')).toBe('en')
    expect(effective('constructor', 'de')).toBe('de')
  })

  it('format sustituye variables y deja intactas las desconocidas', () => {
    expect(format('{current} de {total}', { current: 2, total: 9 })).toBe('2 de 9')
    expect(format('{a} {b}', { a: 'x' })).toBe('x {b}')
  })

  it('t traduce al idioma activo', () => {
    setLang('es')
    expect(i18n.lang).toBe('es')
    expect(t('find.none')).toBe('Sin resultados')
    expect(t('find.count', { current: 1, total: 3 })).toBe('1 de 3')
    setLang('ru')
    expect(t('find.count', { current: 1, total: 3 })).toBe('1 из 3')
  })

  it('idioma desconocido cae al inglés', () => {
    setLang('xx')
    expect(t('find.none')).toBe('No results')
  })

  it('hay un nombre por cada idioma', () => {
    expect(LANGUAGES.map(([code]) => code)).toEqual(['en', 'es', 'pt', 'de', 'it', 'fr', 'nl', 'ru', 'nb', 'sv', 'fi'])
  })
})
