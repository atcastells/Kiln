//! Licence labels found in free text (a kit's LICENSE.md, a piece's credit).

/// Every licence label named in `text`, normalized and without repeats.
/// Labels: CC0, CC-BY, CC-BY-SA, CC-BY-NC, CC-BY-ND, CC-BY-NC-SA,
/// CC-BY-NC-ND, GPL, MIT.
pub fn find(text: &str) -> Vec<String> {
    let up = text.to_uppercase().replace(['_', ' '], "-");
    let mut out: Vec<String> = Vec::new();
    let mut push = |l: &str| {
        if !out.iter().any(|x| x == l) {
            out.push(l.to_string());
        }
    };
    if up.contains("CC0") || up.contains("PUBLIC-DOMAIN") {
        push("CC0");
    }
    for (needle, label) in [
        ("CC-BY-NC-SA", "CC-BY-NC-SA"),
        ("CC-BY-NC-ND", "CC-BY-NC-ND"),
        ("CC-BY-NC", "CC-BY-NC"),
        ("CC-BY-ND", "CC-BY-ND"),
        ("CC-BY-SA", "CC-BY-SA"),
    ] {
        if up.contains(needle) {
            push(label);
        }
    }
    // Plain CC-BY: a mention not followed by -NC, -ND or -SA.
    let plain = up.match_indices("CC-BY").any(|(i, _)| {
        let rest = &up[i + 5..];
        !(rest.starts_with("-NC") || rest.starts_with("-ND") || rest.starts_with("-SA"))
    }) || up.contains("CREATIVECOMMONS.ORG/LICENSES/BY/")
        || up.contains("ATTRIBUTION-4.0");
    if plain {
        push("CC-BY");
    }
    if up.contains("GPL") {
        push("GPL");
    }
    if up.contains("MIT-LICENSE") || up.contains("LICENSE:-MIT") {
        push("MIT");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::find;

    #[test]
    fn reads_the_usual_labels() {
        assert_eq!(
            find("licensed under CC-BY-4.0 (http://creativecommons.org/licenses/by/4.0/)"),
            ["CC-BY"]
        );
        assert_eq!(find("Todo CC0, Poly Haven"), ["CC0"]);
        assert_eq!(find("CC BY-NC 4.0"), ["CC-BY-NC"]);
        assert_eq!(find("CC-BY-SA and CC0"), ["CC0", "CC-BY-SA"]);
        assert!(find("no licence here").is_empty());
    }
}
