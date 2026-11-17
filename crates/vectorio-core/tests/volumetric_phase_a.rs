//! Referencias guardadas ANTES del refactor, ejecutando los ejemplos con su toro predeterminado.
use std::sync::Arc;
use vectorio_core::{svg_out, volumetric::*, Paper, Pen};

const ROOT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/volumetric/phase-a/"
);
fn inks(width: f64) -> Vec<Pen> {
    vec![
        Pen::new("Índigo", [53, 33, 108], width),
        Pen::new("Óxido", [182, 73, 48], width),
        Pen::new("Azul mineral", [50, 105, 150], width),
    ]
}
fn check(engine: &mut Engine, source: Arc<Mesh>, settings: Settings, name: &str, shadow: bool) {
    let paper = Paper::default();
    let pens = if shadow {
        inks(0.22)[..2].to_vec()
    } else {
        inks(0.1764)
    };
    let assign = if shadow { [0, 1, 0] } else { [0, 2, 1] };
    let expected = std::fs::read_to_string(format!("{ROOT}{name}.svg")).unwrap().replace("\r\n", "\n");
    let legacy = engine
        .draw(source.clone(), &settings, &paper, &|| false)
        .unwrap();
    let explicit = engine
        .draw_passes(
            source.clone(),
            &settings,
            &settings.passes(),
            &paper,
            &|| false,
        )
        .unwrap();
    let mut recipe = Recipe::new(source.as_ref().clone(), settings);
    recipe.paper = paper.clone();
    recipe.inks = pens.clone();
    recipe.set_layer_pens(assign);
    let recipe = Recipe::from_json(&recipe.to_json().unwrap()).unwrap();
    let restored = engine.draw_recipe(&recipe, &|| false).unwrap();
    for (label, drawing, mapping) in [
        ("adaptador antiguo", legacy, assign.to_vec()),
        ("pila explícita", explicit, assign.to_vec()),
        ("receta v2", restored, vec![]),
    ] {
        let doc = drawing.document(paper.clone(), pens.clone(), mapping);
        let actual = svg_out::export(&doc, &doc.layers()).replace("\r\n", "\n");
        if actual != expected {
            let left_lines: Vec<_> = actual.lines().collect();
            let right_lines: Vec<_> = expected.lines().collect();
            for (i, (l, r)) in left_lines.iter().zip(right_lines.iter()).enumerate() {
                if l != r {
                    panic!("Mismatch in {name} ({label}) at line {i}:\nLEFT:  {l}\nRIGHT: {r}");
                }
            }
            if left_lines.len() != right_lines.len() {
                panic!("Different line count in {name} ({label}): left {}, right {}", left_lines.len(), right_lines.len());
            }
        }
    }
}
#[test]
fn volumetric_example_svgs_are_byte_identical() {
    let mut engine = Engine::default();
    let source = Arc::new(Primitive::Torus.mesh());
    for (name, recipe) in [
        ("atlas", RecipeKind::Atlas),
        ("strata", RecipeKind::Strata),
        ("x-ray", RecipeKind::XRay),
    ] {
        check(
            &mut engine,
            source.clone(),
            Settings {
                recipe,
                ..Default::default()
            },
            name,
            false,
        );
    }
}
#[test]
fn volumetric_arte_example_svgs_are_byte_identical() {
    let mut engine = Engine::default();
    let source = Arc::new(Primitive::Torus.mesh());
    for (name, recipe, pattern, mesh_inks) in [
        ("ondas", RecipeKind::SurfaceWaves, WavePattern::Waves, 3),
        ("malla", RecipeKind::TriMesh, WavePattern::Waves, 3),
        ("combinado", RecipeKind::Hybrid, WavePattern::Waves, 2),
        (
            "interferencias",
            RecipeKind::SurfaceWaves,
            WavePattern::Interference,
            3,
        ),
        (
            "malla-una-tinta",
            RecipeKind::TriMesh,
            WavePattern::Waves,
            1,
        ),
    ] {
        check(
            &mut engine,
            source.clone(),
            Settings {
                recipe,
                art: ArtSettings {
                    pattern,
                    mesh_inks,
                    ..Default::default()
                },
                ..Default::default()
            },
            name,
            false,
        );
    }
}
#[test]
fn flat_shadow_example_svgs_are_byte_identical() {
    let mut engine = Engine::default();
    let source = Arc::new(Primitive::Torus.mesh());
    for (name, azimuth, middle_tone, outline) in [
        ("sombra-plana", 135., false, false),
        ("sombra-lateral", 20., false, false),
        ("sombra-dos-tonos", 135., true, true),
    ] {
        check(
            &mut engine,
            source.clone(),
            Settings {
                recipe: RecipeKind::FlatShadow,
                shadow: ShadowSettings {
                    azimuth,
                    middle_tone,
                    outline,
                    ..Default::default()
                },
                ..Default::default()
            },
            name,
            true,
        );
    }
}
#[test]
fn real_v1_recipe_migrates_to_v2_without_changing_its_svg() {
    // Archivo emitido por `volumetric` antes de introducir Pass; no se fabrica con Recipe::new.
    let text = include_str!("fixtures/volumetric/recipe-v1.json");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(text).unwrap()["version"],
        1
    );
    let recipe = Recipe::from_json(text).unwrap();
    assert_eq!(recipe.version, 2);
    assert_eq!(recipe.passes.len(), 3);
    assert_eq!(
        recipe.passes.iter().map(|p| p.pen).collect::<Vec<_>>(),
        [0, 2, 1]
    );
    let saved = recipe.to_json().unwrap();
    let json: serde_json::Value = serde_json::from_str(&saved).unwrap();
    assert_eq!(json["version"], 2);
    assert!(json.get("layer_pens").is_none());
    assert_eq!(Recipe::from_json(&saved).unwrap(), recipe);
    let drawing = Engine::default().draw_recipe(&recipe, &|| false).unwrap();
    let doc = drawing.document(recipe.paper, recipe.inks, []);
    assert_eq!(
        svg_out::export(&doc, &doc.layers()).replace("\r\n", "\n"),
        include_str!("fixtures/volumetric/phase-a/atlas.svg").replace("\r\n", "\n")
    );
}
#[test]
fn v1_migrates_all_seven_styles_and_rejects_invalid_assignments_and_versions() {
    let mut old: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/volumetric/recipe-v1.json")).unwrap();
    for kind in [
        RecipeKind::Atlas,
        RecipeKind::Strata,
        RecipeKind::XRay,
        RecipeKind::FlatShadow,
        RecipeKind::SurfaceWaves,
        RecipeKind::TriMesh,
        RecipeKind::Hybrid,
    ] {
        old["settings"]["recipe"] = serde_json::to_value(kind).unwrap();
        let r = Recipe::from_json(&old.to_string()).unwrap();
        assert_eq!(r.version, 2);
        let mut expected = r.settings.passes();
        for p in &mut expected {
            p.pen = [0, 2, 1][p.pen];
        }
        assert_eq!(r.passes, expected);
    }
    old["layer_pens"] = serde_json::json!([0, 2, 99]);
    assert!(Recipe::from_json(&old.to_string()).is_err());
    old["layer_pens"] = serde_json::json!([0, 1]);
    assert!(Recipe::from_json(&old.to_string()).is_err());
    old["version"] = 3.into();
    assert!(Recipe::from_json(&old.to_string()).is_err());
}
#[test]
fn arbitrary_stack_and_six_ink_mesh_roundtrip_and_export_without_collapsing_pens() {
    let settings = Settings {
        recipe: RecipeKind::TriMesh,
        art: ArtSettings {
            facets: 400,
            mesh_inks: 6,
            coloring: MeshColor::Height,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut recipe = Recipe::new(Primitive::Sphere.mesh(), settings);
    recipe.inks = (0..8)
        .map(|i| Pen::new(&format!("Tinta {i}"), [i * 30, 40, 90], 0.2))
        .collect();
    recipe.set_layer_pens([0, 1, 2, 3, 4, 5]);
    let scene = Settings::default();
    let mut wave = scene.passes()[0].clone();
    wave.kind = PassKind::Waves(WaveParams {
        facets: 400,
        lines: 20,
        pattern: WavePattern::Waves,
        origin_azimuth: 30.,
        origin_elevation: 20.,
        deformation: 0.,
        frequency: 5.,
        major_every: 5,
        family: CurveFamily::All,
        outline: false,
    });
    wave.name = "Ondas adicionales".into();
    wave.pen = 7;
    recipe.passes.insert(2, wave);
    recipe.passes.reverse();
    let restored = Recipe::from_json(&recipe.to_json().unwrap()).unwrap();
    assert_eq!(restored, recipe);
    let drawing = Engine::default().draw_recipe(&restored, &|| false).unwrap();
    assert_eq!(drawing.parts.len(), 7);
    let items = drawing.items([]);
    let used: std::collections::BTreeSet<_> = items.iter().filter_map(|i| i.stroke).collect();
    assert_eq!(used, [0, 1, 2, 3, 4, 5, 7].into_iter().collect());
    let doc = drawing.document(restored.paper, restored.inks, []);
    assert_eq!(doc.layers().len(), 7);
    let svg = svg_out::export(&doc, &doc.layers());
    assert!(usvg::Tree::from_str(&svg, &usvg::Options::default()).is_ok());
}
