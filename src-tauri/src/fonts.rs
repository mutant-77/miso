use crate::prefs::familia_valida;

/// Familias instaladas, ordenadas y sin repetir; las que las prefs rechazarían se omiten.
pub fn system_families() -> Vec<String> {
    let mut familias: Vec<String> = match nombres() {
        Ok(nombres) => nombres,
        Err(err) => {
            log::warn!("fuentes del sistema: {err}");
            return Vec::new();
        }
    }
    .into_iter()
    .filter(|n| !n.starts_with('@'))
    .filter_map(|n| familia_valida(&n))
    .collect();
    familias.sort_by_key(|n| n.to_lowercase());
    familias.dedup();
    familias
}

#[cfg(windows)]
fn nombres() -> windows::core::Result<Vec<String>> {
    use windows::Win32::Graphics::DirectWrite::{
        DWRITE_FACTORY_TYPE_SHARED, DWriteCreateFactory, IDWriteFactory,
    };
    use windows::core::{BOOL, w};

    unsafe {
        let factory: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;
        let mut coleccion = None;
        factory.GetSystemFontCollection(&mut coleccion, false)?;
        let Some(coleccion) = coleccion else { return Ok(Vec::new()) };
        let mut nombres = Vec::new();
        for i in 0..coleccion.GetFontFamilyCount() {
            let localizados = coleccion.GetFontFamily(i)?.GetFamilyNames()?;
            let (mut indice, mut existe) = (0, BOOL(0));
            localizados.FindLocaleName(w!("en-us"), &mut indice, &mut existe)?;
            if !existe.as_bool() {
                indice = 0;
            }
            let mut buf = vec![0u16; localizados.GetStringLength(indice)? as usize + 1];
            localizados.GetString(indice, &mut buf)?;
            buf.pop();
            nombres.push(String::from_utf16_lossy(&buf));
        }
        Ok(nombres)
    }
}

#[cfg(not(windows))]
fn nombres() -> Result<Vec<String>, std::convert::Infallible> {
    Ok(Vec::new())
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn lista_las_fuentes_de_windows() {
        let familias = system_families();
        assert!(familias.iter().any(|f| f == "Segoe UI"), "{familias:?}");
        assert!(familias.windows(2).all(|p| p[0].to_lowercase() < p[1].to_lowercase()));
    }
}
