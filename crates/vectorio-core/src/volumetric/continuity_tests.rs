use super::*;
use crate::{p, BBox, Document, Placement};
fn recipe() -> Recipe {
    let mut r = Recipe::new(Primitive::Cube.mesh(), Settings::default());
    r.inks = crate::doc::default_pens();
    r.passes = vec![Pass::new("Aristas", PassKind::Edges { angle: 25. }, 2)];
    r
}
fn volume(r: Recipe) -> VolumeObject {
    VolumeObject {
        id: 0,
        name: "Cubo editable".into(),
        recipe: r,
        placement: Default::default(),
        bindings: vec![],
    }
}
#[test]
fn live_volume_roundtrips_updates_in_place_and_preserves_design_transform() {
    let r = recipe();
    let d = Engine::default().draw_recipe(&r, &|| false).unwrap();
    let mut doc = Document::default();
    let mut sentinel = crate::Item::new(
        "Vecino",
        crate::Geometry::Path {
            lines: vec![],
            closed: false,
        },
        p(0., 0.),
    );
    sentinel.stroke = Some(1);
    let neighbor = doc.add(sentinel);
    let ids = doc.put_volume(volume(r.clone()), &d, None).unwrap();
    let group = doc.get(ids[0]).unwrap().group.unwrap();
    let transform = Placement {
        pos: p(27., -11.),
        scale: 0.7,
        rotation_deg: 18.,
    };
    for id in &ids {
        doc.get_mut(*id).unwrap().place = transform;
    }
    doc.get_mut(ids[0]).unwrap().stroke = Some(4);
    let mut saved = Document::from_json(&doc.to_json()).unwrap();
    let mut edited = saved.volume_recipe(group).unwrap();
    assert_eq!(edited.passes[0].pen, 4);
    edited.settings.yaw = 120.;
    let new = Engine::default().draw_recipe(&edited, &|| false).unwrap();
    let updated = saved
        .put_volume(volume(edited.clone()), &new, Some(group))
        .unwrap();
    assert_eq!(saved.items[0].id, neighbor);
    assert_eq!(saved.volumes.len(), 1);
    assert_eq!(saved.volume(group).unwrap().recipe, edited);
    assert!(ids.iter().all(|id| saved.get(*id).is_none()));
    assert!(updated
        .iter()
        .all(|id| saved.get(*id).unwrap().place == transform));
    assert_eq!(Document::from_json(&saved.to_json()).unwrap(), saved);
    saved.get_mut(updated[0]).unwrap().locked = true;
    let before = saved.clone();
    assert!(saved.put_volume(volume(r), &d, Some(group)).is_err());
    assert_eq!(saved, before);
}
#[test]
fn live_volume_duplicates_independently_remaps_inks_and_prunes_deleted_groups() {
    let r = recipe();
    let d = Engine::default().draw_recipe(&r, &|| false).unwrap();
    let mut doc = Document::default();
    let ids = doc.put_volume(volume(r), &d, None).unwrap();
    let g = doc.get(ids[0]).unwrap().group.unwrap();
    let copies = doc.duplicate(&ids, p(10., 5.));
    let copy = doc.get(copies[0]).unwrap().group.unwrap();
    assert_ne!(g, copy);
    assert_eq!(doc.volume(copy).unwrap().bindings[0].item, copies[0]);
    doc.swap_pens(2, 4);
    assert_eq!(doc.volume_recipe(g).unwrap().passes[0].pen, 4);
    doc.remove_pen(4);
    assert_eq!(doc.volume_recipe(g).unwrap().passes[0].pen, 0);
    Document::from_json(&doc.to_json()).unwrap();
    doc.items.retain(|it| it.group != Some(g));
    let saved = Document::from_json(&doc.to_json()).unwrap();
    assert!(saved.volume(g).is_none());
    assert!(saved.volume(copy).is_some());
    let mut invalid = saved.clone();
    invalid.volumes[0].recipe.settings.projection.distance = f64::NAN;
    assert!(Document::from_json(&invalid.to_json()).is_err());
}
#[test]
fn perspective_has_vanishing_points_reciprocal_depth_and_a_working_near_plane() {
    let mut c = Camera {
        scale: 10.,
        paper_center: p(0., 0.),
        perspective: Some([4., 0.2]),
        ..Default::default()
    };
    let (right, down, away) = c.axes();
    assert!((c.project(right).0.x - 10.).abs() < 1e-9);
    assert!((c.project(right + away * 4.).0.x - 5.).abs() < 1e-9);
    let point = right * 0.5 + down * 0.2 + away * 0.7;
    let (q, z) = c.project(point);
    assert!((c.unproject(q, z) - point).length() < 1e-9);
    let a = right * -1. + away * -1.;
    let b = right * 1. + away * 1.;
    let ta = 0.35;
    let world = a + (b - a) * ta;
    let (pa, za) = c.project(a);
    let (pb, zb) = c.project(b);
    let (pw, zw) = c.project(world);
    let t = (pw.x - pa.x) / (pb.x - pa.x);
    assert!((za + (zb - za) * t - zw).abs() < 1e-9);
    let clipped = c.clip_segment(away * -5., V3::default()).unwrap();
    assert!(c.near_value(clipped[0]).abs() < 1e-9);
    assert!(c.clip_segment(away * -6., away * -5.).is_none());
    let poly = c.clip_polygon(&[right + away * -5., right * -1., down]);
    assert!(poly.len() >= 3 && poly.iter().all(|q| c.near_value(*q) > -1e-9));
    let ground = c.ground(p(3., 5.), -0.3);
    assert!((ground.z + 0.3).abs() < 1e-9);
    assert!(c.project(ground).0.dist(p(3., 5.)) < 1e-9);
    c.perspective = None;
    assert!((c.project(right + away * 4.).0.x - 10.).abs() < 1e-9);
}
#[test]
fn perspective_visibility_uses_projective_depth_instead_of_linear_world_depth() {
    let camera = Camera {
        scale: 10.,
        paper_center: p(50., 50.),
        perspective: Some([4., 0.1]),
        ..Default::default()
    };
    let (r, d, a) = camera.axes();
    let world = |x: f64, y: f64, z: f64| r * x + d * y + a * z;
    let area = BBox {
        min: p(0., 0.),
        max: p(100., 100.),
    };
    let mut scene = crate::scene::Scene::new(camera, area);
    scene.set_intersections(false);
    scene.occluder(&[
        world(-1., -1., 0.),
        world(1., -1., 0.),
        world(1., 1., 0.),
        world(-1., 1., 0.),
    ]);
    scene.line(&[world(-2., 0., 1.), world(2., 0., 1.)]);
    let output = scene.render();
    assert_eq!(output.parts[0].lines.len(), 2);
    assert!(output.parts[0]
        .lines
        .iter()
        .all(|l| l.iter().all(|q| q.x <= 40.01 || q.x >= 59.99)));
    let mut near_scene = crate::scene::Scene::new(camera, area);
    near_scene.line(&[world(-1., 0., -5.), world(1., 0., 0.)]);
    let out = near_scene.render();
    assert!(!out.parts[0].lines.is_empty());
    assert!(out.parts[0]
        .lines
        .iter()
        .flatten()
        .all(|q| q.x.is_finite() && q.y.is_finite()));
}
#[test]
fn multi_view_sheets_are_clipped_repeatable_colored_and_persistent() {
    for sheet in [SheetViews::Orthographic, SheetViews::Turntable] {
        let mut r = recipe();
        r.settings.sheet = sheet;
        r.settings.view_count = 6;
        r.settings.projection.perspective = true;
        r.passes[0].reading = Some(ReadingMap {
            measure: Measure::Height,
            pens: vec![2, 4, 1],
            ..Default::default()
        });
        let mut engine = Engine::default();
        let d = engine.draw_recipe(&r, &|| false).unwrap();
        let again = engine.draw_recipe(&r, &|| false).unwrap();
        assert_eq!(d.parts.len(), r.passes.len());
        assert!(d.parts[0].families.len() >= 3);
        assert!(!d.parts[0].lines.is_empty());
        for q in d.parts.iter().flat_map(|p| p.lines.iter()).flatten() {
            assert!(
                q.x >= r.paper.margin_mm - 1e-7
                    && q.x <= r.paper.width_mm - r.paper.margin_mm + 1e-7
                    && q.y >= r.paper.margin_mm - 1e-7
                    && q.y <= r.paper.height_mm - r.paper.margin_mm + 1e-7
            );
        }
        assert_eq!(d.parts[0].lines, again.parts[0].lines);
        assert_eq!(r, Recipe::from_json(&r.to_json().unwrap()).unwrap());
        assert!(engine.draw_recipe(&r, &|| true).is_err());
    }
}
#[test]
fn all_scene_generators_expose_real_surfaces_and_2d_generator_does_not() {
    let ctx = crate::generate::GenContext {
        area: BBox {
            min: p(15., 15.),
            max: p(180., 120.),
        },
        seed: 7,
        pen_mm: 0.5,
        detail: crate::generate::Detail::Preview,
    };
    for generator in crate::generate::all() {
        if !generator.has_volume() {
            assert_eq!(generator.id(), "spiro");
            assert!(generator.scene(&Default::default(), &ctx).is_err());
            continue;
        }
        let mut params = crate::generate::Params::default();
        if generator.id() == "blocks" {
            params.set("rows", crate::generate::ParamValue::Number(1.));
            params.set("cols", crate::generate::ParamValue::Number(1.));
        }
        let (m, _) = generator.volume(&params, &ctx).unwrap();
        assert!(m.triangles.len() > 10, "{}", generator.id());
        assert!(m.vertices.iter().flatten().all(|v| v.is_finite()));
        assert_eq!(m.triangles.len(), m.face_objects.len());
        // A mesh is measured in world coordinates, not flattened from its 2D preview.
        let [lo, hi] = m.bounds();
        assert!((hi - lo).z > 0.);
    }
}
#[test]
fn creation_conversion_keeps_back_faces_without_affecting_normal_renders() {
    let camera = Camera::default();
    let area = BBox {
        min: p(0., 0.),
        max: p(210., 297.),
    };
    let make = || {
        let mut scene = crate::scene::Scene::new(camera, area);
        scene.box3(V3::new(-1., -1., -1.), V3::new(1., 1., 1.));
        scene
    };
    let normal = make();
    let captured = crate::scene::capture_geometry(make);
    assert_eq!(normal.render(), captured.render());
    assert!(normal.volume_mesh("No capturada").is_err());
    let (mesh, report) = captured.volume_mesh("Cubo").unwrap().validated().unwrap();
    assert_eq!(mesh.triangles.len(), 12);
    assert_eq!(report.boundary_edges, 0);
}
#[test]
fn perspective_and_near_clipping_work_in_all_legacy_pass_algorithms() {
    for kind in [
        RecipeKind::Atlas,
        RecipeKind::Strata,
        RecipeKind::XRay,
        RecipeKind::FlatShadow,
        RecipeKind::SurfaceWaves,
        RecipeKind::TriMesh,
        RecipeKind::Hybrid,
    ] {
        let mut r = recipe();
        r.settings.recipe = kind;
        r.settings.projection.perspective = true;
        r.settings.projection.distance = 0.6;
        r.settings.projection.near = 1.;
        r.passes = r.settings.passes();
        let d = Engine::default().draw_recipe(&r, &|| false).unwrap();
        assert!(d.camera.perspective.is_some());
        assert!(d.parts.iter().any(|p| !p.lines.is_empty()), "{kind:?}");
        for q in d.parts.iter().flat_map(|p| p.lines.iter()).flatten() {
            assert!(q.x.is_finite() && q.y.is_finite());
            assert!(
                q.x >= r.paper.margin_mm - 1e-6
                    && q.x <= r.paper.width_mm - r.paper.margin_mm + 1e-6
                    && q.y >= r.paper.margin_mm - 1e-6
                    && q.y <= r.paper.height_mm - r.paper.margin_mm + 1e-6,
                "{kind:?} {q:?}"
            );
        }
    }
}
