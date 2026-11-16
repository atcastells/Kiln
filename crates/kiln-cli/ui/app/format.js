// Numbers as the interface shows them: Spanish grouping with a dot for the
// thousands (`5.000`, 4-digit numbers included: plain "es" does not group them).
const NF = new Intl.NumberFormat("es", { useGrouping: "always" });
export const fmtN = v => (v == null || Number.isNaN(v) ? "—" : NF.format(v));
