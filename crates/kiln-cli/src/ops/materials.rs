//! Slot bindings, materials by zone, validating a part, splitting a piece into zones.

use super::spec_file;
use crate::{agent::Ctx, production};

pub fn bind(ctx: &Ctx, piece: &str, slot: &str, material: &str) -> Result<(), String> {
    let cfg = production::config(&ctx.project, ctx.config.as_deref())?;
    kiln_core::bindings::set(&ctx.project, &cfg, piece, slot, material)?;
    // Kept on purpose for the page: it writes first and only then checks for empty fields.
    if piece.is_empty() || slot.is_empty() {
        return Err("falta la pieza o el hueco".into());
    }
    Ok(())
}

pub fn material_set(
    ctx: &Ctx,
    asset: &str,
    part: &str,
    zone: &str,
    material: &str,
    variant: Option<&str>,
) -> Result<(), String> {
    let cat = crate::load(&ctx.project, ctx.config.as_deref())?;
    if !material.is_empty() && !cat.materials.iter().any(|m| m.key == material) {
        return Err(format!("«{material}» no está en la paleta"));
    }
    let file = spec_file(&ctx.project, &cat, asset)?;
    let text = std::fs::read_to_string(&file).map_err(|e| e.to_string())?;
    let new = kiln_core::spec::set_material(&text, part, zone, material, variant)?;
    std::fs::write(&file, new).map_err(|e| e.to_string())
}

/// Moves the part's materials to the bindings of its catalog piece; returns the piece key.
pub fn material_validate(ctx: &Ctx, asset: &str, part: &str) -> Result<String, String> {
    let cat = crate::load(&ctx.project, ctx.config.as_deref())?;
    let cfg = production::config(&ctx.project, ctx.config.as_deref())?;
    let file = spec_file(&ctx.project, &cat, asset)?;
    let p = cat
        .specs
        .iter()
        .find(|s| s.asset.id == asset)
        .and_then(|s| s.parts.iter().find(|p| p.id == part))
        .ok_or(format!("la ficha no tiene la pieza «{part}»"))?;
    let piece = p
        .source
        .from
        .as_deref()
        .filter(|_| matches!(p.source.kind.as_str(), "reuse" | "variant"))
        .filter(|f| cat.pieces.iter().any(|c| &c.key == f))
        .ok_or("solo se valida una pieza que viene del catálogo")?;
    kiln_core::bindings::set_many(&ctx.project, &cfg, piece, &p.materials)?;
    let text = std::fs::read_to_string(&file).map_err(|e| e.to_string())?;
    let new = kiln_core::spec::clear_materials(&text, part)?;
    std::fs::write(&file, new).map_err(|e| e.to_string())?;
    Ok(piece.to_string())
}

pub fn zones_split(ctx: &Ctx, piece: &str, mode: &str) -> Result<(), String> {
    let cat = crate::load(&ctx.project, ctx.config.as_deref())?;
    if !cat.pieces.iter().any(|p| p.key == piece) {
        return Err(format!("«{piece}» no está en el catálogo"));
    }
    kiln_core::zones::set(&ctx.project, piece, mode)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::testing::{ctx, examples};

    fn read(c: &Ctx, rel: &str) -> String {
        std::fs::read_to_string(c.project.join(rel)).unwrap_or_default()
    }

    #[test]
    fn bind_writes_the_bindings_file() {
        let t = examples("");
        let c = ctx(&t);
        let cfg = production::config(&c.project, None).unwrap();
        let file = cfg.materials.bindings.clone();
        let before = read(&c, &file);
        bind(&c, "harbor/crate", "Wood", "oak").unwrap();
        assert_ne!(read(&c, &file), before);
    }

    #[test]
    fn bind_writes_before_it_checks_empty_fields() {
        let t = examples("");
        let c = ctx(&t);
        assert_eq!(bind(&c, "", "Wood", "oak").unwrap_err(), "falta la pieza o el hueco");
    }

    #[test]
    fn material_set_changes_the_spec_and_checks_the_palette() {
        let t = examples("");
        let c = ctx(&t);
        let spec = "specs/hoist.toml";
        let before = read(&c, spec);
        material_set(&c, "hoist", "load", "Wood", "oak", None).unwrap();
        assert_ne!(read(&c, spec), before);
        let e = material_set(&c, "hoist", "load", "Wood", "nope", None).unwrap_err();
        assert_eq!(e, "«nope» no está en la paleta");
    }

    #[test]
    fn material_validate_moves_materials_to_the_bindings() {
        let t = examples("");
        let c = ctx(&t);
        let before = read(&c, "specs/hoist.toml");
        assert_eq!(material_validate(&c, "hoist", "load").unwrap(), "harbor/crate");
        assert_ne!(read(&c, "specs/hoist.toml"), before);
        let e = material_validate(&c, "hoist", "ghost").unwrap_err();
        assert_eq!(e, "la ficha no tiene la pieza «ghost»");
    }

    #[test]
    fn zones_split_writes_the_zones_file_and_checks_the_catalog() {
        let t = examples("");
        let c = ctx(&t);
        zones_split(&c, "harbor/crate", "loose").unwrap();
        assert!(read(&c, "kiln/zones.toml").contains("loose"));
        let e = zones_split(&c, "nope/x", "loose").unwrap_err();
        assert_eq!(e, "«nope/x» no está en el catálogo");
    }
}
