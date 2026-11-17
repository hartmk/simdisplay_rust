//! Discover installed faces once; load font bytes only when a widget uses them.
use eframe::egui::{self, FontData, FontDefinitions, FontFamily};
use std::{collections::BTreeMap, sync::OnceLock};

pub struct InstalledFont {
    pub key: String,
    pub label: String,
    id: fontdb::ID,
}
struct SystemFonts {
    db: fontdb::Database,
    faces: Vec<InstalledFont>,
}
fn system() -> &'static SystemFonts {
    static FONTS: OnceLock<SystemFonts> = OnceLock::new();
    FONTS.get_or_init(|| {
        let mut db = fontdb::Database::new();
        db.load_system_fonts();
        let mut faces = BTreeMap::new();
        for face in db.faces() {
            if face.post_script_name.is_empty() {
                continue;
            }
            let key = format!("system:{}", face.post_script_name);
            faces.entry(key.clone()).or_insert(InstalledFont {
                key,
                label: face.post_script_name.clone(),
                id: face.id,
            });
        }
        let mut faces: Vec<_> = faces.into_values().collect();
        faces.sort_by_key(|face| face.label.to_lowercase());
        SystemFonts { db, faces }
    })
}
pub fn installed() -> &'static [InstalledFont] {
    &system().faces
}
pub fn valid_selection(name: &str) -> bool {
    matches!(name, "sans" | "mono" | "condensed")
        || name.strip_prefix("system:").is_some_and(|name| {
            !name.trim().is_empty() && name.len() <= 256 && !name.chars().any(char::is_control)
        })
}
pub fn label(name: &str) -> &str {
    name.strip_prefix("system:").unwrap_or(name)
}

#[derive(Clone)]
struct FontState {
    definitions: FontDefinitions,
    failed: Vec<String>,
}
fn state_id() -> egui::Id {
    egui::Id::new("widget-font-definitions")
}
pub fn configure(ctx: &egui::Context, definitions: FontDefinitions) {
    ctx.data_mut(|data| {
        data.insert_temp(
            state_id(),
            FontState {
                definitions: definitions.clone(),
                failed: vec![],
            },
        )
    });
    ctx.set_fonts(definitions);
}
pub fn unavailable(ctx: &egui::Context, name: &str) -> bool {
    name.starts_with("system:")
        && (installed().iter().all(|face| face.key != name)
            || ctx.data(|data| {
                data.get_temp::<FontState>(state_id())
                    .is_some_and(|state| state.failed.iter().any(|key| key == name))
            }))
}
pub fn family(ctx: &egui::Context, name: &str) -> FontFamily {
    match name {
        "mono" => return FontFamily::Monospace,
        "condensed" => return FontFamily::Name("condensed".into()),
        _ if !name.starts_with("system:") => return FontFamily::Proportional,
        _ => {}
    }
    let family = FontFamily::Name(name.into());
    // set_fonts takes effect on the next frame. Never request an unregistered family.
    if ctx.fonts(|fonts| fonts.families().contains(&family)) {
        return family;
    }
    let updated = ctx.data_mut(|data| {
        let state = data.get_temp_mut_or_insert_with(state_id(), || FontState {
            definitions: FontDefinitions::default(),
            failed: vec![],
        });
        if state.definitions.families.contains_key(&family)
            || state.failed.iter().any(|key| key == name)
        {
            return None;
        }
        let font = installed()
            .iter()
            .find(|face| face.key == name)
            .and_then(|face| {
                system()
                    .db
                    .with_face_data(face.id, |bytes, index| {
                        ab_glyph::FontRef::try_from_slice_and_index(bytes, index).ok()?;
                        let mut font = FontData::from_owned(bytes.to_vec());
                        font.index = index;
                        Some(font)
                    })
                    .flatten()
            });
        let Some(font) = font else {
            state.failed.push(name.into());
            return None;
        };
        state.definitions.font_data.insert(name.into(), font.into());
        let mut fallback = vec![name.into()];
        if let Some(names) = state.definitions.families.get(&FontFamily::Proportional) {
            fallback.extend(names.clone());
        }
        state.definitions.families.insert(family.clone(), fallback);
        Some(state.definitions.clone())
    });
    if let Some(definitions) = updated {
        ctx.set_fonts(definitions);
        ctx.request_repaint();
    }
    FontFamily::Proportional
}
