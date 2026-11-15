use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const HASH_PREFIX: usize = 64 * 1024;

#[cfg(windows)]
fn file_id(path: &Path) -> Option<String> {
    use std::ffi::c_void;
    use std::fs::File;
    use std::os::windows::io::AsRawHandle;

    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::Storage::FileSystem::{
        FILE_ID_INFO, FileIdInfo, GetFileInformationByHandleEx,
    };

    let file = File::open(path).ok()?;
    let mut info = FILE_ID_INFO::default();
    unsafe {
        GetFileInformationByHandleEx(
            HANDLE(file.as_raw_handle()),
            FileIdInfo,
            &mut info as *mut FILE_ID_INFO as *mut c_void,
            std::mem::size_of::<FILE_ID_INFO>() as u32,
        )
    }
    .ok()?;
    Some(format!(
        "{:016x}-{:032x}",
        info.VolumeSerialNumber,
        u128::from_le_bytes(info.FileId.Identifier)
    ))
}

#[cfg(not(windows))]
fn file_id(_path: &Path) -> Option<String> {
    None
}

pub fn fnv1a(seed: u64, bytes: &[u8]) -> u64 {
    bytes
        .iter()
        .fold(seed, |hash, &byte| (hash ^ u64::from(byte)).wrapping_mul(FNV_PRIME))
}

fn content_hash(bytes: &[u8]) -> u64 {
    let size = fnv1a(FNV_OFFSET, &(bytes.len() as u64).to_le_bytes());
    fnv1a(size, &bytes[..bytes.len().min(HASH_PREFIX)])
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DocKey {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub file_id: Option<String>,
    pub content_hash: u64,
    pub path: String,
}

impl DocKey {
    pub fn new(path: &Path, bytes: &[u8]) -> Self {
        let canonical = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        Self {
            file_id: file_id(path),
            content_hash: content_hash(bytes),
            path: canonical.to_string_lossy().into_owned(),
        }
    }

    pub fn same_file(&self, other: &DocKey) -> bool {
        self.path == other.path
            || matches!((&self.file_id, &other.file_id), (Some(a), Some(b)) if a == b)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use super::*;

    struct Dir(PathBuf);

    impl Dir {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir()
                .join(format!("miso-fileid-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).expect("crear directorio temporal");
            Dir(dir)
        }

        fn file(&self, name: &str, bytes: &[u8]) -> PathBuf {
            let path = self.0.join(name);
            fs::write(&path, bytes).expect("escribir archivo temporal");
            path
        }
    }

    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn key(file_id: Option<&str>, path: &str) -> DocKey {
        DocKey {
            file_id: file_id.map(str::to_owned),
            content_hash: 0,
            path: path.to_owned(),
        }
    }

    #[test]
    fn fnv1a_vectores_conocidos() {
        assert_eq!(fnv1a(FNV_OFFSET, b""), 0xcbf29ce484222325);
        assert_eq!(fnv1a(FNV_OFFSET, b"a"), 0xaf63dc4c8601ec8c);
    }

    #[test]
    fn renombrar_conserva_file_id() {
        let dir = Dir::new("renombrar");
        let antes = dir.file("a.txt", b"x");
        let id = file_id(&antes).unwrap_or_else(|| {
            panic!("file_id None en {} (volumen sin ID de archivo)", antes.display())
        });
        let despues = dir.0.join("b.txt");
        fs::rename(&antes, &despues).expect("renombrar temporal");
        assert_eq!(file_id(&despues), Some(id));
    }

    #[test]
    fn archivos_distintos_tienen_file_id_distinto() {
        let dir = Dir::new("distintos");
        let a = dir.file("a.txt", b"x");
        let b = dir.file("b.txt", b"x");
        let id_a = file_id(&a).unwrap_or_else(|| panic!("file_id None en {}", a.display()));
        let id_b = file_id(&b).unwrap_or_else(|| panic!("file_id None en {}", b.display()));
        assert_ne!(id_a, id_b);
    }

    #[test]
    fn hash_solo_mira_tamano_y_64_kb() {
        let base = vec![7u8; 70_000];
        let mut cola_distinta = base.clone();
        cola_distinta[65_536] = 9;
        assert_eq!(content_hash(&base), content_hash(&cola_distinta));

        let corto = vec![7u8; 65_536];
        let largo = vec![7u8; 65_537];
        assert_ne!(content_hash(&corto), content_hash(&largo));
    }

    #[test]
    fn same_file_por_ruta_o_file_id() {
        assert!(key(Some("1"), "/a").same_file(&key(Some("2"), "/a")));
        assert!(key(Some("1"), "/a").same_file(&key(Some("1"), "/b")));
        assert!(!key(None, "/a").same_file(&key(None, "/b")));
        assert!(!key(Some("1"), "/a").same_file(&key(None, "/b")));
    }

    #[test]
    fn doc_key_usa_ruta_canonica() {
        let dir = Dir::new("canonica");
        let path = dir.file("c.txt", b"hola");
        let doc_key = DocKey::new(&path, b"hola");
        let canonica = fs::canonicalize(&path).expect("canonicalizar temporal");
        assert_eq!(doc_key.path, canonica.to_string_lossy().into_owned());
        assert_eq!(doc_key.content_hash, content_hash(b"hola"));
    }
}
