//! Traducciones. El español es el idioma de origen: el propio texto en español es la clave
//! y `locales/app.yml` solo aporta las demás lenguas. Una clave sin traducción se muestra
//! en español, y una prueba avisa de cualquier clave usada que falte en el archivo.

/// Texto traducido (`String`). Con variables: con `tr!` y `ruta = x`.
macro_rules! tr {
    ($($t:tt)*) => { rust_i18n::t!($($t)*).into_owned() };
}

/// Marca un texto en español de una tabla constante: devuelve el mismo texto y permite que la
/// prueba de claves lo encuentre. Se traduce al mostrarlo con `tr_key`.
macro_rules! tm {
    ($s:literal) => { $s };
}

/// Como `tr!`, pero devuelve `&'static str` para poder usarlo donde se espera un `&str`
/// (etiquetas, rótulos, ayudas). Cada traducción se guarda una sola vez y se reutiliza.
macro_rules! ts {
    ($s:literal) => { $crate::i18n::ts($s) };
}

pub fn ts(key: &'static str) -> &'static str {
    use std::{cell::RefCell, collections::HashMap};
    thread_local! { static CACHE: RefCell<HashMap<(bool, &'static str), &'static str>> = RefCell::new(HashMap::new()); }
    let english = !rust_i18n::locale().starts_with("es");
    CACHE.with(|cache| {
        *cache.borrow_mut().entry((english, key)).or_insert_with(|| {
            let text = rust_i18n::t!(key);
            if text == key { key } else { Box::leak(text.into_owned().into_boxed_str()) }
        })
    })
}

/// Nombres de tinta que genera Vectorio. Se guardan siempre en español en el documento y
/// se traducen al mostrarlos, para que sigan al idioma elegido y a su color automático.
const COLOR_NAMES: &[&str] = &[
    tm!("Negro fino"), tm!("Negro"), tm!("Blanco"), tm!("Gris"), tm!("Rojo"), tm!("Naranja"), tm!("Amarillo"), tm!("Verde"),
    tm!("Turquesa"), tm!("Cian"), tm!("Azul"), tm!("Índigo"), tm!("Violeta"), tm!("Morado"), tm!("Magenta"), tm!("Rosa"), tm!("Marrón"),
];

/// Nombre de una tinta para mostrar: las de nombre automático se traducen; las que el usuario
/// ha nombrado se dejan como están.
pub fn pen_name(pen: &vectorio_core::Pen) -> String {
    if pen.custom_name { return pen.name.clone(); }
    for stored in ["Rotulador ", "Pen "] {
        if let Some(n) = pen.name.strip_prefix(stored) {
            if n.chars().all(|c| c.is_ascii_digit()) { return tr!("Rotulador %{n}", n = n); }
        }
    }
    // También reconoce el nombre inglés, por si el documento se creó con la interfaz en inglés.
    match COLOR_NAMES.iter().find(|k| pen.name == **k || pen.name == rust_i18n::t!(**k, locale = "en")) {
        Some(key) => tr_key(key),
        None => pen.name.clone(),
    }
}

/// Las pruebas cambian el idioma global: las que dependen de él lo toman para no pisarse.
#[cfg(test)]
pub fn locale_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Traduce un texto marcado con `tm!` (o cualquier clave en español) en tiempo de ejecución.
pub fn tr_key(key: &str) -> String {
    rust_i18n::t!(key).into_owned()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    /// Claves en español que el código pide con `tr!("…")` o marca con `tm!("…")`.
    fn used_keys() -> BTreeSet<String> {
        let mut keys = BTreeSet::new();
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_none_or(|e| e != "rs") { continue; }
            let text = std::fs::read_to_string(&path).unwrap().replace("\r\n", "\n");
            // Las pruebas pueden usar textos propios: solo cuenta el código de la aplicación.
            let code = text.split("#[cfg(test)]").next().unwrap();
            let code: String = code.lines().filter(|l| !l.trim_start().starts_with("//")).collect::<Vec<_>>().join("\n");
            let code = code.as_str();
            for marker in ["tr!(", "tm!(", "ts!("] {
                let mut rest = code;
                while let Some(i) = rest.find(marker) {
                    rest = &rest[i + marker.len()..];
                    let after = rest.trim_start();
                    let Some(body) = after.strip_prefix('"') else { continue };
                    let (mut key, mut chars) = (String::new(), body.chars());
                    while let Some(c) = chars.next() {
                        match c {
                            '"' => break,
                            '\\' => match chars.next() { Some('n') => key.push('\n'), Some(o) => key.push(o), None => {} },
                            c => key.push(c),
                        }
                    }
                    keys.insert(key);
                }
            }
        }
        // Etiquetas y ayudas que vienen del núcleo (`label()` y `help()`): se traducen en la app.
        let core = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../vectorio-core/src");
        for entry in std::fs::read_dir(core).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_none_or(|e| e != "rs") { continue; }
            let text = std::fs::read_to_string(&path).unwrap().replace("\r\n", "\n");
            for marker in ["pub fn label(", "pub fn help("] {
                let mut rest = text.as_str();
                while let Some(i) = rest.find(marker) {
                    rest = &rest[i + marker.len()..];
                    let body = rest.split("\n    }\n").next().unwrap();
                    let mut chars = body.chars();
                    while let Some(c) = chars.next() {
                        if c != '"' { continue; }
                        let mut key = String::new();
                        for c in chars.by_ref() { if c == '"' { break; } key.push(c); }
                        keys.insert(key);
                    }
                }
            }
        }
        // Nombres de serie del núcleo: tintas y tamaños de papel.
        let doc = std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../vectorio-core/src/doc.rs")).unwrap().replace("\r\n", "\n");
        let pens = doc.split("pub fn default_pens()").nth(1).unwrap().split("\n}\n").next().unwrap();
        let papers = doc.split("pub fn presets() -> Vec<Paper>").nth(1).unwrap().split("\n    }\n").next().unwrap();
        for (text, marker) in [(pens, "Pen::new(\""), (papers, "mk(\"")] {
            for part in text.split(marker).skip(1) { keys.insert(part.split('"').next().unwrap().to_string()); }
        }
        for generator in vectorio_core::generate::all() {
            for text in [generator.name(), generator.category(), generator.description()] { keys.insert(text.to_owned()); }
            for spec in generator.params() {
                keys.insert(spec.label().into()); if let Some(hint) = spec.hint() { keys.insert(hint.into()); }
                if let vectorio_core::generate::ParamSpec::Choice { options, .. } = spec {
                    for (_, label) in options { keys.insert((*label).into()); }
                }
            }
        }
        keys
    }

    fn placeholders(text: &str) -> BTreeSet<String> {
        text.split("%{").skip(1).filter_map(|s| s.split_once('}').map(|(n, _)| n.to_string())).collect()
    }

    fn translations() -> serde_yaml::Mapping {
        let text = std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("locales/app.yml")).unwrap();
        serde_yaml::from_str::<serde_yaml::Mapping>(&text).unwrap()
    }

    #[test]
    fn every_key_used_in_the_code_is_translated_and_none_is_left_over() {
        let table = translations();
        let used = used_keys();
        assert!(used.len() > 100, "la búsqueda de claves no encuentra los textos");
        let mut problems = Vec::new();
        for key in &used {
            match table.get(key.as_str()).and_then(|v| v.get("en")).and_then(|v| v.as_str()) {
                None => problems.push(format!("falta la traducción al inglés de {key:?}")),
                Some(en) if placeholders(en) != placeholders(key) => problems.push(format!("las variables de {key:?} no coinciden con las del inglés")),
                Some(_) => {}
            }
        }
        for key in table.keys().filter_map(|k| k.as_str()) {
            if key != "_version" && !used.contains(key) { problems.push(format!("clave sin usar en locales/app.yml: {key:?}")); }
        }
        assert!(problems.is_empty(), "{}", problems.join("\n"));
    }

    #[test]
    fn missing_translations_fall_back_to_spanish() {
        let _lock = super::locale_lock();
        rust_i18n::set_locale("en");
        assert_eq!(tr!("Texto que nadie ha traducido"), "Texto que nadie ha traducido");
        assert_eq!(tr!("Plan guardado en %{ruta}", ruta = "/a"), "Plan saved to /a");
        rust_i18n::set_locale("es");
        assert_eq!(tr!("Plan guardado en %{ruta}", ruta = "/a"), "Plan guardado en /a");
    }

    #[test]
    fn automatic_pen_names_follow_the_language_in_both_directions() {
        let _lock = super::locale_lock();
        let auto = |n: &str| vectorio_core::Pen::new(n, [0, 0, 0], 0.5);
        let mut mine = auto("Rojo");
        mine.custom_name = true;
        rust_i18n::set_locale("en");
        assert_eq!((super::pen_name(&auto("Rojo")).as_str(), super::pen_name(&auto("Marrón")).as_str(), super::pen_name(&auto("Rotulador 3")).as_str()), ("Red", "Brown", "Pen 3"));
        assert_eq!(super::pen_name(&mine), "Rojo");
        rust_i18n::set_locale("es");
        // Un documento creado con la interfaz en inglés vuelve a verse en español.
        assert_eq!((super::pen_name(&auto("Red")).as_str(), super::pen_name(&auto("Pen 3")).as_str(), super::pen_name(&auto("Fine black")).as_str()), ("Rojo", "Rotulador 3", "Negro fino"));
    }
}
