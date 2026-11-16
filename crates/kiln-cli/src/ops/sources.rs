//! Searching external sites for a part.

use crate::agent::Ctx;

/// The candidates of every active site for one part.
pub fn search(ctx: &Ctx, asset: &str, part: &str) -> Result<serde_json::Value, String> {
    let found = crate::fetch::search(&ctx.project, ctx.config.as_deref(), asset, part)?;
    Ok(serde_json::to_value(found).unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::testing;

    #[test]
    fn unknown_asset_gives_the_route_message() {
        let t = testing::examples("");
        let c = testing::ctx(&t);
        assert_eq!(search(&c, "nope", "deck"), Err("no hay ficha con id «nope»".into()));
    }

    #[test]
    fn unknown_part_gives_the_route_message() {
        let t = testing::examples("");
        let c = testing::ctx(&t);
        assert_eq!(
            search(&c, "cargo_quay", "nope"),
            Err("la ficha no tiene la pieza «nope»".into())
        );
    }
}
