use std::borrow::Cow;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::de::DeserializeOwned;

pub fn load<T: DeserializeOwned + Default>(path: &Path, ok: impl Fn(&T) -> bool) -> T {
    if !path.exists() {
        return T::default();
    }
    let leido = fs::read(path)
        .map_err(|err| err.to_string())
        .and_then(|bytes| serde_json::from_slice::<T>(&bytes).map_err(|err| err.to_string()));
    match leido {
        Ok(value) if ok(&value) => value,
        Ok(_) => apartar(path, "valor fuera de rango"),
        Err(err) => apartar(path, &err),
    }
}

pub fn save<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = con_sufijo(path, ".tmp");
    let bytes = serde_json::to_vec_pretty(value).map_err(io::Error::other)?;
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, path)
}

fn apartar<T: Default>(path: &Path, motivo: &str) -> T {
    let nombre = nombre(path);
    log::warn!("{nombre} no válido ({motivo}); se usan los valores por defecto");
    let corrupto = con_sufijo(path, ".corrupt");
    let _ = fs::remove_file(&corrupto);
    if let Err(err) = fs::rename(path, &corrupto) {
        log::warn!("no se pudo apartar {nombre}: {err}");
    }
    T::default()
}

fn nombre(path: &Path) -> Cow<'_, str> {
    path.file_name().unwrap_or(path.as_os_str()).to_string_lossy()
}

fn con_sufijo(path: &Path, sufijo: &str) -> PathBuf {
    let mut nombre = path.as_os_str().to_owned();
    nombre.push(sufijo);
    PathBuf::from(nombre)
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    #[derive(Serialize, Deserialize, Default, Debug, PartialEq)]
    struct Muestra {
        valor: u32,
    }

    fn dir_de_prueba(nombre: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("miso-store-{nombre}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn load_de_json_roto_aparta_el_archivo() {
        let dir = dir_de_prueba("corrupto");
        let path = dir.join("muestra.json");
        fs::write(&path, "{ no es json").unwrap();
        assert_eq!(load(&path, |_: &Muestra| true), Muestra::default());
        assert!(!path.exists());
        assert!(dir.join("muestra.json.corrupt").exists());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn load_con_ok_falso_da_defecto_y_aparta() {
        let dir = dir_de_prueba("ok-falso");
        let path = dir.join("muestra.json");
        save(&path, &Muestra { valor: 20 }).unwrap();
        assert_eq!(load(&path, |m: &Muestra| m.valor < 10), Muestra::default());
        assert!(!path.exists());
        assert!(dir.join("muestra.json.corrupt").exists());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn save_y_load_son_ida_y_vuelta() {
        let dir = dir_de_prueba("ida-vuelta").join("anidado");
        let path = dir.join("muestra.json");
        let muestra = Muestra { valor: 18 };
        save(&path, &muestra).unwrap();
        assert!(!dir.join("muestra.json.tmp").exists());
        assert_eq!(load(&path, |_: &Muestra| true), muestra);
        fs::remove_dir_all(dir.parent().unwrap()).unwrap();
    }

    #[test]
    fn load_de_inexistente_da_defecto() {
        let dir = dir_de_prueba("inexistente");
        assert_eq!(load(&dir.join("muestra.json"), |_: &Muestra| true), Muestra::default());
        fs::remove_dir_all(&dir).unwrap();
    }
}
