use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::file_id::DocKey;
use crate::store;

const MAX_ENTRIES: usize = 1000;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Entry {
    #[serde(flatten)]
    pub key: DocKey,
    pub char_index: u32,
    pub date_changed: u64,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Positions {
    pub schema_version: u32,
    pub entries: Vec<Entry>,
}

impl Default for Positions {
    fn default() -> Self {
        Self {
            schema_version: 1,
            entries: Vec::new(),
        }
    }
}

impl Positions {
    fn find(&self, key: &DocKey) -> Option<usize> {
        if let Some(id) = &key.file_id
            && let Some(i) = self
                .entries
                .iter()
                .position(|e| e.key.file_id.as_ref() == Some(id))
        {
            return Some(i);
        }
        let por_hash: Vec<(usize, &Entry)> = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.key.content_hash == key.content_hash)
            .collect();
        if let Some(&(i, _)) = por_hash.iter().find(|(_, e)| e.key.path == key.path) {
            return Some(i);
        }
        if let Some(&(i, _)) = por_hash.iter().max_by_key(|(_, e)| e.date_changed) {
            return Some(i);
        }
        self.entries.iter().position(|e| e.key.path == key.path)
    }

    fn get(&self, key: &DocKey, flat_len: u32) -> u32 {
        self.find(key)
            .map_or(0, |i| self.entries[i].char_index.min(flat_len))
    }

    fn set(&mut self, key: &DocKey, char_index: u32, now_ms: u64) {
        match self.find(key) {
            Some(i) => {
                let entry = &mut self.entries[i];
                entry.key = key.clone();
                entry.char_index = char_index;
                entry.date_changed = now_ms;
            }
            None => self.entries.push(Entry {
                key: key.clone(),
                char_index,
                date_changed: now_ms,
            }),
        }
    }

    fn prune(&mut self) {
        if self.entries.len() > MAX_ENTRIES {
            self.entries.sort_by_key(|e| std::cmp::Reverse(e.date_changed));
            self.entries.truncate(MAX_ENTRIES);
        }
    }
}

struct Inner {
    positions: Positions,
    dirty: bool,
    scheduled: bool,
}

pub struct PositionsState {
    inner: Mutex<Inner>,
    path: PathBuf,
}

impl PositionsState {
    pub fn load(path: PathBuf) -> Self {
        let positions = store::load(&path, |p: &Positions| p.schema_version == 1);
        Self {
            inner: Mutex::new(Inner {
                positions,
                dirty: false,
                scheduled: false,
            }),
            path,
        }
    }

    pub fn get(&self, key: &DocKey, flat_len: u32) -> u32 {
        self.inner.lock().unwrap().positions.get(key, flat_len)
    }

    pub fn record(&self, key: &DocKey, char_index: u32, now_ms: u64) -> bool {
        let mut inner = self.inner.lock().unwrap();
        inner.positions.set(key, char_index, now_ms);
        inner.dirty = true;
        if inner.scheduled {
            false
        } else {
            inner.scheduled = true;
            true
        }
    }

    pub fn flush(&self) {
        let mut inner = self.inner.lock().unwrap();
        self.guardar(&mut inner);
    }

    pub fn scheduled_flush(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.scheduled = false;
        self.guardar(&mut inner);
    }

    fn guardar(&self, inner: &mut Inner) {
        if !inner.dirty {
            return;
        }
        inner.positions.prune();
        match store::save(&self.path, &inner.positions) {
            Ok(()) => inner.dirty = false,
            Err(err) => log::error!("positions.json: {err}"),
        }
    }
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    struct Dir(PathBuf);

    impl Dir {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir()
                .join(format!("miso-positions-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).expect("crear directorio temporal");
            Dir(dir)
        }
    }

    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn key(file_id: Option<&str>, hash: u64, path: &str) -> DocKey {
        DocKey {
            file_id: file_id.map(str::to_owned),
            content_hash: hash,
            path: path.to_owned(),
        }
    }

    fn entry(
        file_id: Option<&str>,
        hash: u64,
        path: &str,
        char_index: u32,
        date_changed: u64,
    ) -> Entry {
        Entry {
            key: key(file_id, hash, path),
            char_index,
            date_changed,
        }
    }

    fn con(entries: Vec<Entry>) -> Positions {
        Positions {
            schema_version: 1,
            entries,
        }
    }

    #[test]
    fn find_por_file_id_aunque_hash_y_path_difieran() {
        let p = con(vec![entry(Some("1"), 1, "/a", 0, 0)]);
        assert_eq!(p.find(&key(Some("1"), 2, "/b")), Some(0));
    }

    #[test]
    fn find_por_hash_prefiere_igual_path() {
        let p = con(vec![
            entry(None, 7, "/otra", 0, 99),
            entry(None, 7, "/a", 0, 1),
        ]);
        assert_eq!(p.find(&key(None, 7, "/a")), Some(1));
    }

    #[test]
    fn find_por_hash_sin_path_gana_la_mas_reciente() {
        let p = con(vec![
            entry(None, 7, "/x", 0, 5),
            entry(None, 7, "/y", 0, 9),
            entry(None, 7, "/z", 0, 1),
        ]);
        assert_eq!(p.find(&key(None, 7, "/a")), Some(1));
    }

    #[test]
    fn find_por_path_sola() {
        let p = con(vec![entry(Some("1"), 1, "/a", 0, 0)]);
        assert_eq!(p.find(&key(Some("2"), 2, "/a")), Some(0));
    }

    #[test]
    fn file_id_none_no_casa_con_none() {
        let p = con(vec![entry(None, 1, "/a", 0, 0)]);
        assert_eq!(p.find(&key(None, 2, "/b")), None);
    }

    #[test]
    fn set_reescribe_la_clave_sin_anadir() {
        let mut p = con(vec![entry(Some("1"), 7, "/a", 0, 1)]);
        let nueva = key(Some("2"), 7, "/b");
        p.set(&nueva, 42, 99);
        assert_eq!(p.entries.len(), 1);
        assert_eq!(p.entries[0].key, nueva);
        assert_eq!(p.entries[0].char_index, 42);
        assert_eq!(p.entries[0].date_changed, 99);
    }

    #[test]
    fn set_de_clave_nueva_anade() {
        let mut p = con(vec![entry(Some("1"), 1, "/a", 0, 1)]);
        p.set(&key(Some("2"), 2, "/b"), 5, 2);
        assert_eq!(p.entries.len(), 2);
    }

    #[test]
    fn get_recorta_a_flat_len_y_da_cero_sin_entrada() {
        let p = con(vec![
            entry(Some("1"), 1, "/a", 50, 0),
            entry(Some("2"), 2, "/b", 5, 0),
        ]);
        assert_eq!(p.get(&key(Some("1"), 1, "/a"), 20), 20);
        assert_eq!(p.get(&key(Some("2"), 2, "/b"), 20), 5);
        assert_eq!(p.get(&key(Some("3"), 3, "/c"), 20), 0);
    }

    #[test]
    fn prune_deja_las_mas_recientes() {
        let entries = (0..1005)
            .map(|i| entry(None, i, &format!("/p{i}"), 0, i))
            .collect();
        let mut p = con(entries);
        p.prune();
        assert_eq!(p.entries.len(), MAX_ENTRIES);
        assert!(p.entries.iter().all(|e| e.date_changed >= 5));
        assert!(p.entries.iter().any(|e| e.date_changed == 1004));
    }

    #[test]
    fn serializa_sin_file_id_al_nivel_superior() {
        let e = entry(None, 3, "/a", 12, 99);
        let v = serde_json::to_value(&e).unwrap();
        let obj = v.as_object().unwrap();
        let mut claves: Vec<&str> = obj.keys().map(String::as_str).collect();
        claves.sort();
        assert_eq!(claves, ["char_index", "content_hash", "date_changed", "path"]);
    }

    #[test]
    fn ida_y_vuelta_con_y_sin_file_id() {
        for e in [entry(None, 3, "/a", 12, 99), entry(Some("abc"), 3, "/a", 12, 99)] {
            let v = serde_json::to_value(&e).unwrap();
            let vuelta: Entry = serde_json::from_value(v).unwrap();
            assert_eq!(vuelta, e);
        }
    }

    #[test]
    fn schema_distinto_da_defecto_y_aparta() {
        let dir = Dir::new("schema");
        let path = dir.0.join("positions.json");
        fs::write(&path, r#"{"schema_version": 2, "entries": []}"#).unwrap();
        let state = PositionsState::load(path.clone());
        assert_eq!(state.inner.lock().unwrap().positions, Positions::default());
        assert!(!path.exists());
        assert!(dir.0.join("positions.json.corrupt").exists());
    }

    #[test]
    fn flush_sin_cambios_no_crea_archivo() {
        let dir = Dir::new("sin-cambios");
        let path = dir.0.join("positions.json");
        PositionsState::load(path.clone()).flush();
        assert!(!path.exists());
    }

    #[test]
    fn record_flush_y_recarga() {
        let dir = Dir::new("record");
        let path = dir.0.join("positions.json");
        let state = PositionsState::load(path.clone());
        let k = key(Some("1"), 5, "/a");
        assert!(state.record(&k, 30, 1000));
        state.flush();
        assert!(path.exists());
        assert_eq!(PositionsState::load(path).get(&k, 100), 30);
    }

    #[test]
    fn record_solo_programa_hasta_scheduled_flush() {
        let dir = Dir::new("programar");
        let state = PositionsState::load(dir.0.join("positions.json"));
        let k = key(Some("1"), 5, "/a");
        assert!(state.record(&k, 30, 1000));
        assert!(!state.record(&k, 31, 1001));
        state.scheduled_flush();
        assert!(state.record(&k, 32, 1002));
    }

    #[test]
    fn flush_con_error_deja_dirty() {
        let dir = Dir::new("bloqueo");
        let bloqueo = dir.0.join("bloqueo");
        fs::write(&bloqueo, b"x").unwrap();
        let state = PositionsState::load(bloqueo.join("positions.json"));
        state.record(&key(Some("1"), 5, "/a"), 1, 1);
        state.flush();
        assert!(state.inner.lock().unwrap().dirty);
    }
}
