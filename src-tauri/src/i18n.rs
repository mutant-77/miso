use std::collections::HashMap;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};

pub const LANGS: [&str; 11] = ["en", "es", "pt", "de", "it", "fr", "nl", "ru", "nb", "sv", "fi"];

const SOURCES: [&str; 11] = [
    include_str!("../../locales/en.json"),
    include_str!("../../locales/es.json"),
    include_str!("../../locales/pt.json"),
    include_str!("../../locales/de.json"),
    include_str!("../../locales/it.json"),
    include_str!("../../locales/fr.json"),
    include_str!("../../locales/nl.json"),
    include_str!("../../locales/ru.json"),
    include_str!("../../locales/nb.json"),
    include_str!("../../locales/sv.json"),
    include_str!("../../locales/fi.json"),
];

type Catalog = HashMap<String, String>;

static CATALOGS: OnceLock<Vec<Catalog>> = OnceLock::new();
static SYSTEM: OnceLock<&'static str> = OnceLock::new();
static CURRENT: AtomicUsize = AtomicUsize::new(0);

fn catalogs() -> &'static [Catalog] {
    CATALOGS.get_or_init(|| {
        SOURCES
            .iter()
            .map(|src| serde_json::from_str(src).expect("catálogo de idioma inválido"))
            .collect()
    })
}

fn index(code: &str) -> Option<usize> {
    LANGS.iter().position(|&l| l == code)
}

/// Primer idioma soportado de la lista de preferencias del sistema; si no hay, inglés.
pub fn detect(locales: &[String]) -> &'static str {
    locales
        .iter()
        .filter_map(|locale| {
            let primary = locale.split(['-', '_']).next()?.to_ascii_lowercase();
            let primary = if primary == "no" || primary == "nn" { "nb".into() } else { primary };
            index(&primary).map(|i| LANGS[i])
        })
        .next()
        .unwrap_or(LANGS[0])
}

pub fn system() -> &'static str {
    SYSTEM.get_or_init(|| detect(&sys_locale::get_locales().collect::<Vec<_>>()))
}

/// `language` de prefs ("auto" o un código) → idioma de la interfaz.
pub fn effective(language: &str) -> &'static str {
    index(language).map_or_else(system, |i| LANGS[i])
}

pub fn is_valid(language: &str) -> bool {
    language == "auto" || index(language).is_some()
}

pub fn set(code: &str) {
    CURRENT.store(index(code).unwrap_or(0), Ordering::Relaxed);
}

pub fn current() -> &'static str {
    LANGS[CURRENT.load(Ordering::Relaxed)]
}

pub fn t(key: &'static str) -> &'static str {
    t_in(current(), key)
}

pub fn t_in(code: &str, key: &'static str) -> &'static str {
    let all = catalogs();
    let own = index(code).and_then(|i| all[i].get(key));
    own.or_else(|| all[0].get(key)).map_or(key, String::as_str)
}

pub fn format(template: &str, vars: &[(&str, &str)]) -> String {
    vars.iter()
        .fold(template.to_owned(), |text, (name, value)| text.replace(&format!("{{{name}}}"), value))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn locales(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    fn variables(text: &str) -> Vec<&str> {
        let mut vars: Vec<&str> = text
            .split('{')
            .skip(1)
            .filter_map(|rest| rest.split_once('}').map(|(name, _)| name))
            .collect();
        vars.sort_unstable();
        vars
    }

    #[test]
    fn detect_toma_el_primer_idioma_soportado() {
        assert_eq!(detect(&locales(&["es-ES"])), "es");
        assert_eq!(detect(&locales(&["ca-ES", "es-ES"])), "es");
        assert_eq!(detect(&locales(&["pt_BR"])), "pt");
        assert_eq!(detect(&locales(&["DE-de"])), "de");
    }

    #[test]
    fn detect_noruego_es_bokmal() {
        assert_eq!(detect(&locales(&["no-NO"])), "nb");
        assert_eq!(detect(&locales(&["nn"])), "nb");
        assert_eq!(detect(&locales(&["nb-NO"])), "nb");
    }

    #[test]
    fn detect_sin_soportado_es_ingles() {
        assert_eq!(detect(&locales(&["ja-JP"])), "en");
        assert_eq!(detect(&[]), "en");
    }

    #[test]
    fn effective_respeta_el_codigo_y_auto_va_al_sistema() {
        assert_eq!(effective("ru"), "ru");
        assert_eq!(effective("auto"), system());
        assert!(LANGS.contains(&system()));
    }

    #[test]
    fn is_valid_solo_auto_y_codigos() {
        assert!(is_valid("auto") && is_valid("fi"));
        assert!(!is_valid("pt-BR") && !is_valid("") && !is_valid("xx"));
    }

    #[test]
    fn catalogos_con_las_mismas_claves_y_variables_que_el_ingles() {
        let all = catalogs();
        let en = &all[0];
        for (code, catalog) in LANGS.iter().zip(all) {
            let mut keys: Vec<_> = catalog.keys().collect();
            let mut expected: Vec<_> = en.keys().collect();
            keys.sort();
            expected.sort();
            assert_eq!(keys, expected, "{code}");
            for (key, text) in catalog {
                assert!(!text.trim().is_empty(), "{code}: {key} vacío");
                assert_eq!(variables(text), variables(&en[key]), "{code}: {key}");
            }
        }
    }

    #[test]
    fn t_in_usa_el_idioma_y_si_falta_la_clave_la_devuelve() {
        assert_eq!(t_in("es", "clipboard.title"), "Portapapeles");
        assert_eq!(t_in("en", "clipboard.title"), "Clipboard");
        assert_eq!(t_in("xx", "clipboard.title"), "Clipboard");
        assert_eq!(t_in("es", "no.existe"), "no.existe");
    }

    #[test]
    fn format_sustituye_variables() {
        let text = format("{a} de {b}, {a}", &[("a", "1"), ("b", "2")]);
        assert_eq!(text, "1 de 2, 1");
    }
}
