//! Mesh integrity on a real published GLB (spec R6): the EST-004 cavity of
//! ASTRA, one welded piece of genus 1 with its normals into the void. Reads
//! the GLB named by `KILN_MESH_FIXTURE` and writes nothing:
//! `KILN_MESH_FIXTURE=<ASTRA>/godot/assets/kiln/historic_cavity/historic_cavity.glb
//! cargo test -p kiln-core --test integrity_fixture -- --ignored --nocapture`.
use kiln_core::canonical::topology::pieces_and_euler;
use kiln_core::integrity::{check, weld};
use kiln_core::spec::{MeshRules, Normals};
use std::time::Instant;

/// Counts of ASTRA's own checker on the same cavity (`.sdlc/historic-excavations/
/// review/T43-check-final.log`: faces 322, distinct 161, euler 0).
const ASTRA_TRIANGLES: usize = 322;
const ASTRA_VERTICES: usize = 161;

#[test]
#[ignore = "needs KILN_MESH_FIXTURE pointing at ASTRA's published historic_cavity.glb"]
fn the_published_cavity_passes_its_declared_topology() {
    let path = std::env::var("KILN_MESH_FIXTURE").expect("KILN_MESH_FIXTURE must name ASTRA's historic_cavity.glb");
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let w = weld(&bytes).expect("readable GLB").unwrap_or_else(|r| panic!("weld: {} {:?} {}", r.code, r.element, r.detail));
    let g = &w.geometry;
    let (pieces, chi, _) = pieces_and_euler(g);
    eprintln!("welded: {} triangles, {} vertices, {} pieces, V - E + F = {chi}", g.triangles.len(), g.vertices.len(), pieces);
    assert_eq!((g.triangles.len(), g.vertices.len(), pieces, chi), (ASTRA_TRIANGLES, ASTRA_VERTICES, 1, 0));

    let rules = |genus, normals| MeshRules { components: 1, genus, normals };
    let start = Instant::now();
    let passed = check(&w, &rules(1, Normals::Inward));
    eprintln!("check genus 1 inward (full sweep included): {:?} in {:?}", passed.as_ref().map_err(|r| r.code), start.elapsed());
    if let Err(r) = passed {
        panic!("declared rules fail: {} element {:?}: {}", r.code, r.element, r.detail);
    }
    for (name, genus, normals, code) in [("genus 0", 0, Normals::Inward, "mesh_topology"), ("outward", 1, Normals::Outward, "mesh_normals")] {
        let r = check(&w, &rules(genus, normals)).expect_err(name);
        eprintln!("check {name}: {} {:?}: {}", r.code, r.element, r.detail);
        assert_eq!(r.code, code, "{name}: {}", r.detail);
    }
}
