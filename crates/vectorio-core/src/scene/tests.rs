use super::*;
use crate::geom::{p, Rng};

fn paper() -> BBox { BBox { min: p(0.0, 0.0), max: p(210.0, 297.0) } }
fn camera() -> Camera { Camera { scale: 20.0, ..Camera::default() } }
fn scene() -> Scene { Scene::new(camera(), paper()) }
fn v(x: f64, y: f64, z: f64) -> V3 { V3::new(x, y, z) }
fn segments(output: &SceneOutput) -> Vec<(P, P)> {
    output.parts.iter().flat_map(|part| part.lines.iter().flat_map(|l| l.windows(2).map(|w| (w[0], w[1])))).collect()
}

// Detecta también duplicados parciales y con sentido inverso; compartir un extremo es lícito.
fn assert_no_overlaps(output: &SceneOutput) {
    let edges = segments(output);
    for (i, &(a, b)) in edges.iter().enumerate() {
        let d = b - a;
        let len = d.len();
        if len < 1e-8 { continue; }
        let unit = d * (1.0 / len);
        for &(c, e) in &edges[i + 1..] {
            if cross(unit, c - a).abs() > 1e-7 || cross(unit, e - a).abs() > 1e-7 { continue; }
            let (u, w) = ((c - a).dot(unit), (e - a).dot(unit));
            assert!(u.max(w).min(len) - u.min(w).max(0.0) <= 1e-7, "Segmentos solapados: {a:?} {b:?} / {c:?} {e:?}");
        }
    }
}

#[test]
fn cube_has_nine_unique_edges_and_expected_length() {
    let mut s = scene();
    s.box3(v(0.0, 0.0, 0.0), v(1.0, 1.0, 1.0));
    let out = s.render();
    assert_eq!(segments(&out).len(), 9);
    assert_no_overlaps(&out);
    let expected = 9.0 * 20.0 * (2.0f64 / 3.0).sqrt() / 1000.0;
    assert!((out.length_m(PartId(0)) - expected).abs() < 1e-12);
}

#[test]
fn rear_box_edge_is_cut_at_analytical_front_box_boundary() {
    let camera = Camera { yaw_deg: 0.0, elev_deg: 45.0, scale: 10.0, paper_center: p(100.0, 100.0), ..camera() };
    let mut s = Scene::new(camera, paper());
    let rear = s.part("caja detrás");
    s.set_part(rear);
    s.box3(v(-2.0, -2.0, 0.0), v(-1.0, 2.0, 1.0));
    s.set_part(PartId(0));
    s.box3(v(0.0, -1.0, 0.0), v(1.0, 1.0, 4.0));
    let out = s.render();
    // x_papel = 100 + 10*y_mundo. La caja delantera ocupa [90, 110].
    // La arista x=-1,z=1 queda a y_papel=100-20/sqrt(2).
    let y = 100.0 - 20.0 / 2f64.sqrt();
    let mut cuts: Vec<_> = out.parts[rear.0 as usize].lines.iter().flat_map(|l| l.windows(2))
        .filter(|w| (w[0].y - y).abs() < 1e-8 && (w[1].y - y).abs() < 1e-8)
        .map(|w| (w[0].x.min(w[1].x), w[0].x.max(w[1].x))).collect();
    cuts.sort_by(|a, b| a.0.total_cmp(&b.0));
    assert_eq!(cuts.len(), 2, "{cuts:?}");
    for (actual, expected) in cuts.iter().zip([(80.0, 90.0), (110.0, 120.0)]) {
        assert!((actual.0 - expected.0).abs() < 1e-6 && (actual.1 - expected.1).abs() < 1e-6);
    }
}

#[test]
fn depth_crossing_is_clipped_linearly_without_sampling() {
    let mut s = scene();
    let cam = *s.camera();
    let world = |x, y, depth| cam.unproject(p(x, y), depth);
    s.occluder(&[world(80.0, 80.0, 0.0), world(120.0, 80.0, 0.0),
        world(120.0, 120.0, 0.0), world(80.0, 120.0, 0.0)]);
    s.line(&[world(90.0, 100.0, -1.0), world(110.0, 100.0, 1.0)]);
    let out = s.render();
    let line = &out.parts[0].lines[0];
    // d(t)=-1+2t; la ocultación empieza en d=0,002 m.
    assert!((line.last().unwrap().x - 100.02).abs() < 1e-9);
}

#[test]
fn tolerance_keeps_coplanar_details_and_tiled_occluders_leave_no_false_gap() {
    let mut s = scene(); let cam = *s.camera();
    let w = |x, y, d| cam.unproject(p(x, y), d);
    for (x0, x1) in [(80.0, 100.0), (100.0, 120.0)] {
        s.occluder(&[w(x0, 80.0, 0.0), w(x1, 80.0, 0.0), w(x1, 120.0, 0.0), w(x0, 120.0, 0.0)]);
    }
    s.line(&[w(90.0, 90.0, 0.0019), w(110.0, 90.0, 0.0019)]);
    s.line(&[w(90.0, 100.0, 0.0021), w(110.0, 100.0, 0.0021)]);
    let out = s.render();
    eprintln!("OUT LINES: {:?}", out.parts[0].lines);
    assert_eq!(out.parts[0].lines.len(), 1);
    assert!((out.length_m(PartId(0)) - 0.02).abs() < 1e-12);
}

#[test]
fn coplanar_neighbors_do_not_hide_edges_or_hatching() {
    let mut s = scene();
    let mut faces = s.box3(v(0.0, 0.0, 0.0), v(1.0, 1.0, 1.0));
    faces.extend(s.box3(v(1.0, 0.0, 0.0), v(2.0, 1.0, 1.0)));
    let hatch = s.part("trama"); s.set_part(hatch);
    for face in faces {
        // Las tapas forman un plano continuo. La cara interior del muro sí debe
        // desaparecer: la caja vecina está delante de ella.
        if normal(&face).z < 0.9 { continue; }
        s.hatch_face(&face, face[1] - face[0], 1.17);
    }
    let out = s.render();
    assert!(!out.parts[0].lines.is_empty());
    assert!(out.parts[hatch.0 as usize].lines.len() > 20);
    // Cada trama debe llegar sin cortes a los dos lados de su propia cara.
    let strokes: Vec<_> = s.strokes.iter().filter(|stroke| stroke.part == hatch).collect();
    let expected: f64 = strokes.iter().map(|stroke| stroke.vertices[0].point.dist(stroke.vertices[1].point))
        .filter(|length| *length >= MIN_STROKE_MM).sum();
    assert!((out.length_m(hatch) * 1000.0 - expected).abs() < 1e-6);
    // Ambas cajas conservan sus cuatro aristas superiores completas.
    for x in [0.0, 2.0] {
        let (a, b) = (s.camera.project(v(x, 0.0, 1.0)).0, s.camera.project(v(x, 1.0, 1.0)).0);
        assert!(out.parts[0].lines.iter().any(|l| l[0].dist(a).min(l[0].dist(b)) < 1e-7 &&
            (crate::geom::length(l) - a.dist(b)).abs() < 1e-7));
    }
}

#[test]
fn edge_on_face_does_not_occlude() {
    let mut s = scene();
    let cam = *s.camera();
    s.occluder(&[cam.unproject(p(80.0, 100.0), -2.0), cam.unproject(p(120.0, 100.0), -2.0),
        cam.unproject(p(120.0, 100.0), -1.0)]);
    s.line(&[cam.unproject(p(90.0, 100.0), 1.0), cam.unproject(p(110.0, 100.0), 1.0)]);
    assert!(s.faces.is_empty());
    let line = &s.render().parts[0].lines[0];
    assert!(line[0].dist(p(90.0, 100.0)) < 1e-7 && line[1].dist(p(110.0, 100.0)) < 1e-7);
}

#[test]
fn whole_line_is_atomic_for_occlusion_and_paper_clipping() {
    let mut s = scene(); let cam = *s.camera();
    let w = |x, y, d| cam.unproject(p(x, y), d);
    s.occluder(&[w(95.0, 95.0, 0.0), w(105.0, 95.0, 0.0), w(105.0, 105.0, 0.0), w(95.0, 105.0, 0.0)]);
    s.whole_line(&[w(90.0, 100.0, 1.0), w(110.0, 100.0, 1.0)]);
    s.whole_line(&[w(90.0, 90.0, 1.0), w(100.0, 90.0, 1.0), w(110.0, 90.0, 1.0)]);
    s.whole_line(&[w(-1.0, 80.0, 1.0), w(10.0, 80.0, 1.0)]);
    let out = s.render();
    assert_eq!(out.parts[0].lines.len(), 1);
    assert_eq!(out.parts[0].lines[0].len(), 3);
    assert!((out.length_m(PartId(0)) - 0.02).abs() < 1e-12);
}

#[test]
fn smooth_cylinder_keeps_two_generators_and_upper_cap() {
    let mut s = scene();
    s.lathe(V3::default(), &[(1.0, 0.0), (1.0, 2.0)], true);
    let out = s.render();
    let vertical = (s.camera.project(v(0.0, 0.0, 2.0)).0 - s.camera.project(V3::default()).0).len();
    let generators: Vec<_> = segments(&out).into_iter().filter(|(a, b)| (a.x - b.x).abs() < 1e-8 &&
        (a.dist(*b) - vertical).abs() < 1e-8).collect();
    assert_eq!(generators.len(), 2);
    // Ninguna generatriz interior se emite siquiera antes del recorte.
    let raw_vertical = s.strokes.iter().filter(|stroke| {
        let (a, b) = (stroke.vertices[0].point, stroke.vertices[1].point);
        (a.x - b.x).abs() < 1e-8 && (a.dist(b) - vertical).abs() < 1e-8
    }).count();
    assert_eq!(raw_vertical, 2);
    let count = s.circle_segments(1.0);
    let top: Vec<_> = (0..count).map(|i| {
        let a = std::f64::consts::TAU * i as f64 / count as f64;
        s.camera.project(v(a.cos(), a.sin(), 2.0)).0
    }).collect();
    for i in 0..count {
        let (a, b) = (top[i], top[(i + 1) % count]);
        assert!(segments(&out).iter().any(|(c, d)| c.dist(a).min(c.dist(b)) < 1e-7 &&
            d.dist(a).min(d.dist(b)) < 1e-7), "Falta arista de tapa {i}");
    }
    assert_no_overlaps(&out);
}

#[test]
fn overlapping_shadows_are_merged_and_groups_are_aligned() {
    let mut s = scene();
    s.set_sun(Some(v(-1.0, -1.0, 2.0)));
    s.begin_shadows(0.0, Some([-5.0, -5.0, 5.0, 5.0]), 27.0);
    s.box3(v(0.0, 0.0, 0.0), v(1.0, 1.0, 2.0));
    s.box3(v(0.4, 0.3, 0.0), v(1.4, 1.3, 1.0));
    let shadow = s.part("sombra"); s.set_part(shadow);
    s.hatch_shadows(0.8);
    let mut out = s.render();
    out.parts.retain(|part| part.id == shadow);
    assert!(segments(&out).len() > 10);
    assert_no_overlaps(&out);
    let (sin, cos) = 27f64.to_radians().sin_cos();
    let n = p(-sin, cos);
    for (a, b) in segments(&out) {
        assert!((a.dot(n) / 0.8 - (a.dot(n) / 0.8).round()).abs() < 1e-8);
        assert!((a.dot(n) - b.dot(n)).abs() < 1e-8);
    }
    s.begin_shadows(0.0, None, 27.0);
    s.set_quiet(true);
    s.box3(v(3.0, 0.0, 0.0), v(4.0, 1.0, 1.0));
    s.hatch_shadows(0.8);
    for stroke in s.strokes.iter().filter(|stroke| stroke.part == shadow) {
        for q in &stroke.vertices { assert!((q.point.dot(n) / 0.8 - (q.point.dot(n) / 0.8).round()).abs() < 1e-8); }
    }
}

fn mixed_scene() -> Scene {
    let mut s = scene();
    s.set_sun(Some(v(-1.0, 2.0, 3.0)));
    s.begin_shadows(0.0, None, 31.0);
    let faces = s.box3(v(-3.0, -2.0, 0.0), v(-1.0, 0.0, 1.5));
    // Pieza pegada y almena: fuerzan aristas enteras y parciales compartidas.
    s.box3(v(-1.0, -2.0, 0.0), v(0.0, 0.0, 1.5));
    s.box3(v(-2.5, -2.0, 1.5), v(-1.8, -1.0, 2.2));
    s.rotated_box(v(1.0, -2.0, 0.0), v(2.0, -1.0, 2.0), 18.0);
    s.lathe(v(1.0, 2.0, 0.0), &[(0.8, 0.0), (0.8, 1.5), (0.0, 2.5)], false);
    let hatch = s.part("trama"); s.set_part(hatch);
    for f in faces { let direction = f[1] - f[0]; s.hatch_face(&f, direction, 1.13); }
    let shadow = s.part("sombra"); s.set_part(shadow); s.hatch_shadows(0.93);
    s
}

#[test]
fn mixed_scene_has_no_duplicate_segments() { assert_no_overlaps(&mixed_scene().render()); }

#[test]
fn rendering_is_exactly_deterministic_and_preserves_parts() {
    let mut s = mixed_scene();
    assert_eq!(s.part("trama"), PartId(1));
    assert_eq!(s.render(), s.render());
    assert_eq!(s.render(), mixed_scene().render());
    let lines = s.part("orden"); s.set_part(lines);
    let cam = *s.camera();
    s.line(&[cam.unproject(p(2.0, 2.0), -20.0), cam.unproject(p(4.0, 2.0), -20.0)]);
    s.line(&[cam.unproject(p(2.0, 3.0), -20.0), cam.unproject(p(4.0, 3.0), -20.0)]);
    let out = s.render();
    assert!(out.parts[lines.0 as usize].lines[0][0].y < out.parts[lines.0 as usize].lines[1][0].y);
}

#[test]
fn camera_roundtrip_fit_and_detail_scale() {
    let mut rng = Rng::new(42);
    for elev in [26.57, 30.0, 35.264, 65.0] {
        let mut cam = Camera { yaw_deg: 127.0, elev_deg: elev, ..camera() };
        cam.fit([v(-2.0, -3.0, 0.0), v(4.0, 5.0, 6.0)], paper(), 10.0);
        for x in [-2.0, 4.0] { for y in [-3.0, 5.0] { for z in [0.0, 6.0] {
            let q = cam.project(v(x, y, z)).0;
            assert!(q.x >= 10.0 - 1e-9 && q.x <= 200.0 + 1e-9 && q.y >= 10.0 - 1e-9 && q.y <= 287.0 + 1e-9);
        } } }
        for _ in 0..100 {
            let q = v(rng.sym() * 3.0, rng.sym() * 3.0, rng.f() * 6.0);
            let (p, depth) = cam.project(q);
            assert!((cam.ground(p, q.z) - q).length() < 1e-10);
            assert!((cam.unproject(p, depth) - q).length() < 1e-10);
        }
    }
    let s = scene();
    assert_eq!(s.paper_mm(0.5), 10.0);
    assert!(s.visible(0.5, 1.0)); assert!(!s.visible(0.001, 0.1));
    for r in [0.001, 0.1, 1.0, 10.0, 100.0] {
        let count = s.circle_segments(r);
        assert!(s.paper_mm(r) * (1.0 - (std::f64::consts::PI / count as f64).cos()) <= 0.04 + 1e-10);
    }
}

#[test]
fn hatching_has_paper_spacing_and_is_hidden_by_another_face() {
    let mut s = scene();
    let faces = s.box3(v(0.0, 0.0, 0.0), v(3.0, 3.0, 1.0));
    let face = faces.iter().find(|f| normal(f).z > 0.9).unwrap();
    let hatch = s.part("trama"); s.set_part(hatch);
    s.hatch_face(face, v(1.0, 0.0, 0.0), 0.7);
    let clear = s.render().length_m(hatch);
    let projected = s.camera.project(v(1.0, 0.0, 0.0)).0 - s.camera.project(V3::default()).0;
    let normal = p(-projected.y, projected.x) * (1.0 / projected.len());
    let raw: Vec<_> = s.strokes.iter().filter(|stroke| stroke.part == hatch).collect();
    assert!(raw.len() > 10);
    for pair in raw.windows(2) {
        assert!(((pair[1].vertices[0].point - pair[0].vertices[0].point).dot(normal).abs() - 0.7).abs() < 1e-9);
    }
    s.set_quiet(true); s.box3(v(0.5, 0.5, 1.0), v(2.5, 2.5, 2.0));
    assert!(s.render().length_m(hatch) < clear * 0.8);
}

#[test]
fn tiny_strokes_are_removed_after_joining_and_paper_clipping() {
    let mut s = scene(); let cam = *s.camera();
    s.line(&[cam.unproject(p(-1.0, 10.0), 0.0), cam.unproject(p(1.0, 10.0), 0.0)]);
    s.line(&[cam.unproject(p(1.0, 20.0), 0.0), cam.unproject(p(1.02, 20.0), 0.0)]);
    // Los tramos pequeños consecutivos conservan el trazo completo de 0,1 mm.
    let points: Vec<_> = (0..=10).map(|i| cam.unproject(p(2.0 + i as f64 * 0.01, 20.0), 0.0)).collect();
    s.line(&points);
    let out = s.render();
    assert_eq!(out.parts[0].lines.len(), 2);
    assert!((out.length_m(PartId(0)) - 0.0011).abs() < 1e-12);
}

#[test]
fn tone_and_quiet_solids() {
    let mut s = scene();
    assert_eq!(s.tone(v(0.0, 0.0, 1.0)), Tone::Mid);
    s.set_sun(Some(v(1.0, 0.0, 2.0)));
    assert_eq!(s.tone(v(0.0, 0.0, 1.0)), Tone::Lit);
    assert_eq!(s.tone(v(1.0, 0.0, 0.0)), Tone::Mid);
    assert_eq!(s.tone(v(-1.0, 0.0, 0.0)), Tone::Dark);
    s.set_tone_thresholds(ToneThresholds { lit: 0.4, dark: -0.5 });
    assert_eq!(s.tone(v(1.0, 0.0, 0.0)), Tone::Lit);
    s.set_quiet(true); s.box3(v(0.0, 0.0, 0.0), v(1.0, 1.0, 1.0));
    assert!(s.render().parts[0].lines.is_empty());
    assert!(!s.faces.is_empty());
}

#[test]
fn own_visible_faces_keep_all_hatching_and_tangent_hatch_is_not_an_edge() {
    let mut s = scene();
    let faces = s.box3(v(0.0, 0.0, 0.0), v(1.0, 1.0, 1.0));
    let hatch = s.part("trama"); s.set_part(hatch);
    for f in faces { s.hatch_face(&f, f[1] - f[0], 0.83); }
    let expected: f64 = s.strokes.iter().filter(|stroke| stroke.part == hatch)
        .map(|stroke| stroke.vertices[0].point.dist(stroke.vertices[1].point)).filter(|l| *l >= MIN_STROKE_MM).sum();
    let out = s.render();
    assert!((out.length_m(hatch) * 1000.0 - expected).abs() < 1e-6);
    assert_no_overlaps(&out);
    // Una cara de pantalla cuyos dos bordes horizontales coinciden con la fase
    // de trama no debe volver a dibujar esos bordes, ni siquiera en otra parte.
    let mut s = scene(); let cam = *s.camera();
    let face: Vec<_> = [p(80.0, 80.0), p(100.0, 80.0), p(100.0, 100.0), p(80.0, 100.0)]
        .iter().map(|q| cam.unproject(*q, 0.0)).collect();
    let mut face = face;
    if !cam.facing(normal(&face)) { face.reverse(); }
    s.closed(&face);
    let hatch = s.part("otra tinta"); s.set_part(hatch);
    s.hatch_face(&face, cam.basis().0, 1.0);
    assert_eq!(s.render().parts[1].lines.len(), 19);
    assert_no_overlaps(&s.render());
}

#[test]
fn overlapping_lines_keep_first_pass_and_do_not_merge_parts() {
    let mut s = scene(); let cam = *s.camera();
    let w = |x, y| cam.unproject(p(x, y), 0.0);
    s.line(&[w(10.0, 10.0), w(20.0, 10.0), w(30.0, 10.0)]);
    s.line(&[w(40.0, 10.0), w(15.0, 10.0)]);
    let other = s.part("otra"); s.set_part(other);
    s.line(&[w(10.0, 10.0), w(30.0, 10.0)]);
    let output = s.render();
    assert_eq!(output.parts[0].lines.len(), 2);
    assert_eq!(output.parts[0].lines[0].len(), 3);
    assert!((output.length_m(PartId(0)) - 0.03).abs() < 1e-12);
    assert!((output.length_m(other) - 0.02).abs() < 1e-12);
    let mut first = output; first.parts.pop(); assert_no_overlaps(&first);
}

#[test]
fn mixed_winding_extrusion_and_lathe_tips_are_closed_and_deterministic() {
    let mut s = scene();
    let vertices = [v(0.0, 0.0, 0.0), v(1.0, 0.0, 0.0), v(0.0, 1.0, 0.0), v(0.0, 0.0, 1.0)];
    let faces = s.solid(&vertices, &[vec![0, 1, 2], vec![0, 1, 3], vec![0, 3, 2], vec![1, 2, 3]], &[], false);
    let center = v(0.25, 0.25, 0.25);
    assert!(faces.iter().all(|f| normal(f).dot(f[0] - center) > 0.0));
    s.extrude(&[v(2.0, 0.0, 0.0), v(3.0, 0.0, 0.0), v(2.5, 0.0, 1.0)], v(0.0, 1.0, 0.0));
    s.lathe(v(0.0, 3.0, 0.0), &[(0.0, 0.0), (0.5, 1.0), (0.0, 2.0)], true);
    assert_eq!(s.render(), s.render());
    assert_no_overlaps(&s.render());
}

#[test]
fn shadow_world_clip_and_ground_intersection_are_respected() {
    let mut s = scene(); s.set_sun(Some(v(-2.0, -1.0, 3.0)));
    s.begin_shadows(0.0, Some([0.0, 0.0, 2.0, 2.0]), -25.0);
    s.set_quiet(true);
    s.box3(v(-1.0, -1.0, -1.0), v(1.0, 1.0, 2.0));
    // La sombra de un objeto por completo bajo el suelo no se recoge.
    s.box3(v(3.0, 3.0, -2.0), v(4.0, 4.0, -1.0));
    // Comprobamos sus trazos de suelo antes de aplicar la ocultación.
    s.hatch_shadows(0.7);
    assert!(!s.strokes.is_empty());
    for stroke in &s.strokes { for vertex in &stroke.vertices {
        let world = s.camera.ground(vertex.point, 0.0);
        assert!(world.x >= -1e-8 && world.y >= -1e-8 && world.x <= 2.0 + 1e-8 && world.y <= 2.0 + 1e-8);
    } }
}

#[test]
#[ignore = "Medición de rendimiento: ejecutar con --release -- --ignored --nocapture"]
fn benchmark_5000_faces_20000_segments() {
    let mut s = scene();
    let cam = *s.camera();
    let mut rng = Rng::new(731);
    // 5.000 caras independientes repartidas en A4; 20.000 segmentos que cruzan
    // múltiples celdas, unos visibles y otros ocultos. No mide solo descartes.
    for i in 0..5000 {
        let (x, y) = (2.0 + rng.f() * 202.0, 2.0 + rng.f() * 289.0);
        let depth = -0.1 - (i % 7) as f64 * 0.2;
        s.occluder(&[cam.unproject(p(x, y), depth), cam.unproject(p(x + 3.0, y), depth),
            cam.unproject(p(x + 3.0, y + 3.0), depth), cam.unproject(p(x, y + 3.0), depth)]);
    }
    for i in 0..20000 {
        let (x, y) = (rng.f() * 200.0, rng.f() * 287.0);
        let depth = if i % 3 == 0 { -2.0 } else { 1.0 };
        s.line(&[cam.unproject(p(x, y), depth), cam.unproject(p(x + 5.0 + rng.f() * 5.0, y + 5.0), depth)]);
    }
    assert_eq!(s.faces.len(), 5000);
    assert_eq!(s.strokes.len(), 20000);
    let mut times = Vec::new();
    for _ in 0..7 {
        let start = std::time::Instant::now();
        let output = s.render();
        times.push(start.elapsed());
        std::hint::black_box(output);
    }
    times.sort();
    eprintln!("5.000 caras / 20.000 segmentos: mediana {:?}, mínimo {:?}, máximo {:?}", times[3], times[0], times[6]);
    assert!(times[3].as_millis() < 100, "La mediana debe estar por debajo de 100 ms en release");
}

#[test]
fn cylinder_surface_hatch_is_one_stroke_per_scanline() {
    let mut s = scene(); s.set_quiet(true);
    let faces = s.lathe(V3::default(), &[(1.0, 0.0), (1.0, 3.0)], true);
    s.hatch_surface(&faces, 0.0, 0.7);
    let output = s.render();
    let mut ys = Vec::new();
    for line in &output.parts[0].lines {
        assert!(line.iter().all(|q| (q.y - line[0].y).abs() < 1e-8));
        assert!(!ys.iter().any(|y: &f64| (*y - line[0].y).abs() < 1e-8));
        ys.push(line[0].y);
    }
    assert!(ys.len() > 50);
}

#[test]
fn degenerate_generated_input_is_harmless() {
    let mut s = scene();
    for n in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        s.box3(V3::default(), v(n, n, n));
        s.extrude(&[v(0.0,0.0,0.0), v(1.0,0.0,0.0), v(0.0,1.0,0.0)], v(1.0,0.0,if n.is_finite() {0.0} else {n}));
        s.lathe(V3::default(), &[(n,0.0),(n,0.0)], true);
        s.line(&[v(n,0.0,0.0), v(f64::NAN,0.0,0.0)]);
        s.hatch_surface(&[], n, n);
    }
    s.solid(&[], &[vec![9,10,11]], &[], false);
    s.occluder(&[v(0.0,0.0,0.0),v(1.0,0.0,0.0),v(1.0,1.0,1.0),v(0.0,1.0,0.0)]);
    s.begin_shadows(0.0, None, 0.0); s.hatch_shadows(1.0);
    s.set_part(PartId(999));
    assert!(segments(&s.render()).is_empty());
    let mut cam = Camera { scale: f64::NAN, ..camera() };
    cam.fit([V3::default();2], paper(), f64::NAN);
    assert!(cam.project(v(f64::NAN,0.0,0.0)).0.x.is_finite());
    assert!(cam.ground(p(f64::NAN,0.0),0.0).finite());
    let mut s = Scene::new(cam, paper()); s.line(&[V3::default(), v(1.0,1.0,1.0)]);
    assert!(segments(&s.render()).is_empty());
}

#[test]
fn nearly_edge_on_face_is_discarded_and_nontangent_hatch_is_projected() {
    let mut s = scene(); let c = *s.camera();
    s.occluder(&[c.unproject(p(80.0,100.0),0.0),c.unproject(p(120.0,100.0),0.0),
        c.unproject(p(120.0,100.0+1e-8),1.0),c.unproject(p(80.0,100.0+1e-8),1.0)]);
    assert!(s.faces.is_empty());
    let f = s.box3(V3::default(),v(1.0,1.0,1.0));
    for face in &f { s.hatch_face(face, (face[1]-face[0])+normal(face)*2.0, 1.0); }
    assert!(s.render().parts[0].lines.len() > 9);
}

#[test]
fn necked_lathe_has_outward_faces_and_convex_backface_culling_is_equivalent() {
    let mut s = scene();
    let faces = s.lathe(V3::default(), &[(1.0,0.0),(0.5,1.0),(1.0,2.0)], true);
    for f in &faces {
        let center = f.iter().copied().fold(V3::default(), |a,b|a+b)*(1.0/f.len() as f64);
        let n = normal(f);
        if n.z.abs() < 0.999 { assert!(n.dot(v(center.x,center.y,0.0)) > 0.0); }
    }
    assert_no_overlaps(&s.render());
    // Un punto en el interior del cuello queda oculto por su cara delantera.
    let part=s.part("interior");s.set_part(part);s.line(&[v(-0.1,0.0,0.9),v(0.1,0.0,0.9)]);
    assert!(s.render().parts[part.0 as usize].lines.is_empty());
    let mut s = scene(); let faces=s.box3(V3::default(),v(2.0,2.0,2.0));
    s.box3(v(-2.0,-1.0,0.0),v(0.0,1.0,1.0));
    let before=s.render();
    for f in faces { s.occluder(&f); }
    assert_eq!(before,s.render());
}

#[test]
#[ignore = "Medición realista en release"]
fn benchmark_long_diagonals_and_shaded_boxes() {
    let mut s=scene(); let cam=*s.camera();
    s.set_sun(Some(v(-1.0,1.0,2.0))); s.begin_shadows(0.0,None,20.0);
    for y in 0..20 { for x in 0..25 {
        let (x,y)=(x as f64*0.32-4.0,y as f64*0.32-3.0);
        let faces=s.box3(v(x,y,0.0),v(x+0.22,y+0.22,0.3));
        for f in faces { let gap=match s.tone(normal(&f)){Tone::Lit=>1.5,Tone::Mid=>1.0,Tone::Dark=>0.7};
            s.hatch_face(&f,f[1]-f[0],gap); }
    } }
    s.hatch_shadows(1.2);
    for i in 0..2000 { let y=(i as f64*0.137)%297.0;
        s.line(&[cam.unproject(p(0.0,y),1.0),cam.unproject(p(210.0,297.0-y),1.0)]); }
    let mut times=Vec::new();
    for _ in 0..5 { let t=std::time::Instant::now();std::hint::black_box(s.render());times.push(t.elapsed()); }
    times.sort(); eprintln!("500 cajas tramadas + sombras + 2.000 diagonales: mediana {:?}, máximo {:?}",times[2],times[4]);
    assert!(times[2].as_millis()<100);
    s.set_intersections(false);let mut control=Vec::new();
    for _ in 0..7 {let t=std::time::Instant::now();std::hint::black_box(s.render());control.push(t.elapsed());}
    control.sort();let ratio=times[2].as_secs_f64()/control[3].as_secs_f64();
    eprintln!("Sin intersecciones {:?}; relación activadas/desactivadas {:.3}",control[3],ratio);
    assert!(ratio<1.2,"Las intersecciones no deben empeorar más de un 20 % esta carga");
}

fn intersections_only(scene:&Scene)->SceneOutput{
    let grid=super::hidden::Grid::new(scene.paper,&scene.faces);
    let strokes=super::intersections::strokes(scene,&grid);
    let mut only=scene.clone();only.strokes=strokes;only.set_intersections(false);only.render()
}
#[test]
fn interpenetrating_boxes_have_exact_analytical_intersection_in_later_part(){
    let mut s=scene();s.box3(v(0.0,0.0,0.0),v(2.0,2.0,3.0));
    let later=s.part("caja baja");s.set_part(later);s.box3(v(1.0,1.0,0.0),v(3.0,3.0,1.0));
    let out=intersections_only(&s);
    assert!(out.parts[0].lines.is_empty());let lines=&out.parts[later.0 as usize].lines;
    assert_eq!(lines.len(),1);assert_eq!(lines[0].len(),3);
    assert!((out.length_m(later)*1000.0-2.0*20.0*(2.0f64/3.0).sqrt()).abs()<1e-6);
    for part in s.render().parts {assert_no_overlaps(&SceneOutput{parts:vec![part]});}
}
#[test]
fn chimney_roof_and_lathe_intersections_join_and_are_occluded_normally(){
    let mut s=scene();
    s.extrude(&[v(-2.0,-2.0,2.0),v(2.0,-2.0,2.0),v(0.0,-2.0,4.0)],v(0.0,4.0,0.0));
    let chimney=s.part("chimenea");s.set_part(chimney);s.box3(v(0.5,-0.5,2.0),v(1.0,0.5,5.0));
    let out=intersections_only(&s);let lines=&out.parts[chimney.0 as usize].lines;
    assert_eq!(lines.len(),1);assert_eq!(lines[0].len(),3);
    for part in s.render().parts {assert_no_overlaps(&SceneOutput{parts:vec![part]});}
    // Una tapa nueva delante debe cortar también el encuentro añadido.
    let full=s.render();let mut shafts=0;
    for line in &full.parts[chimney.0 as usize].lines {for pair in line.windows(2) {
        if (pair[0].x-pair[1].x).abs()>1e-8 || pair[0].dist(pair[1])<3.0 {continue;}
        for x in [0.5,1.0] {for y in [-0.5,0.5] {
            let origin=s.camera.project(v(x,y,0.0)).0;
            if (origin.x-pair[0].x).abs()<1e-7 {
                let z=|q:P|(origin.y-q.y)/(s.camera.scale*s.camera.elev_deg.to_radians().cos());
                assert!(z(pair[0]).min(z(pair[1]))>=4.0-x-0.01,"Fuste dentro del tejado");shafts+=1;
            }
        }}
    }}assert!(shafts>=2);
    let before=out.length_m(chimney);s.set_quiet(true);s.box3(v(0.6,0.3,3.0),v(1.3,0.8,4.0));
    assert!(intersections_only(&s).length_m(chimney)<before);
    let mut s=scene();s.box3(v(-2.0,-2.0,0.0),v(2.0,2.0,1.0));
    let cylinder=s.part("cilindro");s.set_part(cylinder);s.lathe(V3::default(),&[(0.7,0.5),(0.7,3.0)],true);
    let out=intersections_only(&s);let lines=&out.parts[cylinder.0 as usize].lines;
    assert_eq!(lines.len(),1);assert!(lines[0].len()>10);
    for part in s.render().parts {assert_no_overlaps(&SceneOutput{parts:vec![part]});}
}
#[test]
fn contacts_quiet_coplanar_and_disabled_solids_do_not_duplicate_intersections(){
    let mut s=scene();s.box3(v(0.0,0.0,0.0),v(3.0,1.0,1.0));
    for i in 0..3{s.box3(v(i as f64,0.0,1.0),v(i as f64+0.6,1.0,1.6));}
    assert!(segments(&intersections_only(&s)).is_empty());for part in s.render().parts {assert_no_overlaps(&SceneOutput{parts:vec![part]});}
    for quiet in [false,true]{let mut s=scene();s.box3(v(0.0,0.0,0.0),v(2.0,2.0,3.0));
        s.set_quiet(quiet);if !quiet{s.set_intersections(false);}
        s.box3(v(1.0,1.0,0.0),v(3.0,3.0,1.0));assert!(segments(&intersections_only(&s)).is_empty());}
    let mut s=scene();for _ in 0..2{s.box3(V3::default(),v(1.0,1.0,1.0));}
    assert!(segments(&intersections_only(&s)).is_empty());for part in s.render().parts {assert_no_overlaps(&SceneOutput{parts:vec![part]});}
}

#[test]
fn holes_cut_only_coplanar_strokes_and_keep_frames_and_fill() {
    let mut s = scene(); let cam = *s.camera();
    let w = |x, y, d| cam.unproject(p(x, y), d);
    s.hole(&[w(90.0, 90.0, 0.0), w(110.0, 90.0, 0.0), w(110.0, 110.0, 0.0), w(90.0, 110.0, 0.0)]);
    let wall = s.part("pared"); s.set_part(wall);
    s.line(&[w(80.0, 100.0, 0.0), w(120.0, 100.0, 0.0)]);
    // Cruza el plano del hueco en x = 100: solo se borra el tramo dentro de la holgura.
    s.line(&[w(80.0, 95.0, -0.5), w(120.0, 95.0, 0.5)]);
    let near = s.part("delante"); s.set_part(near);
    s.line(&[w(80.0, 105.0, -1.0), w(120.0, 105.0, -1.0)]);
    let far = s.part("detrás"); s.set_part(far);
    s.line(&[w(80.0, 102.0, 1.0), w(120.0, 102.0, 1.0)]);
    let frame = s.part("marco"); s.set_part(frame);
    s.whole_line(&[w(90.0, 90.0, 0.0), w(110.0, 90.0, 0.0), w(110.0, 110.0, 0.0), w(90.0, 110.0, 0.0), w(90.0, 90.0, 0.0)]);
    s.whole_line(&[w(100.0, 90.0, 0.0), w(100.0, 110.0, 0.0)]);
    let out = s.render();
    let len = |id: PartId| out.length_m(id) * 1000.0;
    // 20 mm fuera del hueco, más la diagonal menos 2·0,01 m / (1 m / 40 mm) = 0,8 mm.
    assert!((len(wall) - 20.0 - 39.2).abs() < 1e-6, "pared: {}", len(wall));
    assert!(out.parts[wall.0 as usize].lines.iter().flatten().all(|q| !(q.x > 90.0 + 1e-6 && q.x < 110.0 - 1e-6 && (q.y - 100.0).abs() < 1e-9)));
    assert!((len(near) - 40.0).abs() < 1e-9 && (len(far) - 40.0).abs() < 1e-9);
    assert!((len(frame) - 100.0).abs() < 1e-9);
}

#[test]
fn window_hole_clears_wall_hatching_but_not_its_dark_fill() {
    let mut s = scene();
    let faces = s.box3(v(0.0, 0.0, 0.0), v(4.0, 4.0, 3.0));
    let face = faces.iter().find(|f| { let n = normal(f); n.z.abs() < 1e-9 && s.camera().facing(n) }).unwrap().clone();
    let (o, h, u) = (face[0], face[1] - face[0], face[3] - face[0]);
    let window = [o + h * 0.3 + u * 0.3, o + h * 0.7 + u * 0.3, o + h * 0.7 + u * 0.7, o + h * 0.3 + u * 0.7];
    let trama = s.part("trama"); s.set_part(trama);
    s.hatch_face(&face, h, 0.6);
    s.hole(&window);
    let inside = |q: P, s: &Scene| {
        let poly: Vec<P> = window.iter().map(|w| s.camera().project(*w).0).collect();
        let sign = (0..4).map(|i| cross(poly[(i + 1) % 4] - poly[i], q - poly[i])).collect::<Vec<_>>();
        sign.iter().all(|c| *c > 1e-6) || sign.iter().all(|c| *c < -1e-6)
    };
    let mids = |out: &SceneOutput| out.parts[trama.0 as usize].lines.iter().flat_map(|l| l.windows(2).map(|w| w[0].lerp(w[1], 0.5))).collect::<Vec<_>>();
    let out = s.render();
    assert!(!mids(&out).is_empty() && mids(&out).iter().all(|q| !inside(*q, &s)));
    let relleno = s.part("relleno"); s.set_part(relleno);
    s.hatch_hole(&window, u, 0.4);
    let out = s.render();
    assert!(out.parts[relleno.0 as usize].lines.len() >= 3);
    assert!(mids(&out).iter().all(|q| !inside(*q, &s)));
}

#[test]
fn lathe_parts_group_faces_by_band_and_meridian_with_caps_apart() {
    let profile = [(1.0, 0.0), (0.6, 1.0), (0.9, 2.0), (0.0, 3.0)];
    let mut s = scene();
    let lathe = s.lathe_parts(V3::default(), &profile, true).unwrap();
    assert_eq!(lathe.bands.len(), profile.len() - 1);
    assert!(lathe.bands.iter().all(|band| band.len() == lathe.segments));
    // La punta superior no tiene tapa; la base, sí.
    assert_eq!(lathe.caps.len(), 1);
    assert!(lathe.caps[0].iter().all(|q| q.z.abs() < 1e-12));
    let step = std::f64::consts::TAU / lathe.segments as f64;
    for (k, band) in lathe.bands.iter().enumerate() {
        for (m, face) in band.iter().enumerate() {
            assert!(face.iter().all(|q| q.z >= profile[k].1 - 1e-9 && q.z <= profile[k + 1].1 + 1e-9), "tramo {k}");
            let rim: Vec<_> = face.iter().filter(|q| q.x.hypot(q.y) > 1e-9).collect();
            let mean = rim.iter().map(|q| V3::new(q.x, q.y, 0.0).normalized()).fold(V3::default(), |a, b| a + b);
            let angle = mean.y.atan2(mean.x).rem_euclid(std::f64::consts::TAU);
            assert!((angle - (m as f64 + 0.5) * step).abs() < 1e-6, "meridiano {m}");
        }
    }
    let mut flat = scene();
    assert_eq!(flat.lathe(V3::default(), &profile, true), lathe.clone().into_faces());
    assert!(scene().lathe_parts(V3::default(), &[(1.0, 1.0), (1.0, 0.0)], true).is_none());
}

fn cube_with_split_top(size: f64) -> Vec<Vec<V3>> {
    let s = size;
    vec![
        vec![v(0.0, 0.0, 0.0), v(0.0, s, 0.0), v(s, s, 0.0), v(s, 0.0, 0.0)],
        vec![v(0.0, 0.0, s), v(s, 0.0, s), v(s, s, s)],
        vec![v(0.0, 0.0, s), v(s, s, s), v(0.0, s, s)],
        vec![v(0.0, 0.0, 0.0), v(s, 0.0, 0.0), v(s, 0.0, s), v(0.0, 0.0, s)],
        vec![v(s, 0.0, 0.0), v(s, s, 0.0), v(s, s, s), v(s, 0.0, s)],
        vec![v(s, s, 0.0), v(0.0, s, 0.0), v(0.0, s, s), v(s, s, s)],
        vec![v(0.0, s, 0.0), v(0.0, 0.0, 0.0), v(0.0, 0.0, s), v(0.0, s, s)],
    ]
}

#[test]
fn coplanar_pieces_of_one_solid_share_no_visible_edge() {
    let mut s = scene();
    let groups = s.polyhedron(&cube_with_split_top(2.0));
    assert_eq!(groups.len(), 7);
    let out = s.render();
    // La diagonal de la tapa no se dibuja: quedan las nueve aristas de una caja.
    assert_eq!(out.parts[0].lines.len(), 9, "{:?}", out.parts[0].lines);
    assert!(s.seams(&out).is_empty());
    // Dos cajas adosadas sí conservan su junta.
    let mut s = scene();
    s.box3(V3::default(), v(2.0, 2.0, 2.0));
    s.box3(v(2.0, 0.0, 0.0), v(4.0, 2.0, 3.0));
    assert!(!s.seams(&s.render()).is_empty());
}

#[test]
fn polyhedron_orients_mixed_windings_and_rejects_open_shells() {
    let mut faces = cube_with_split_top(2.0);
    for face in faces.iter_mut().step_by(2) { face.reverse(); }
    let mut s = scene();
    let groups = s.polyhedron(&faces);
    assert_eq!(groups.len(), 7);
    let center = v(1.0, 1.0, 1.0);
    for group in &groups { for piece in group {
        let n = (piece[1] - piece[0]).cross(piece[2] - piece[0]);
        assert!(n.dot(piece[0] - center) > 0.0);
    } }
    let mut open = cube_with_split_top(2.0);
    open.pop();
    assert!(scene().polyhedron(&open).is_empty());
}

#[test]
fn stepped_prism_is_one_solid_with_convex_pieces_and_no_seams() {
    // Perfil de escalera de tres peldaños en el plano yz, extruido a lo largo de x.
    let profile = [v(0.0, 0.0, 0.0), v(0.0, 3.0, 0.0), v(0.0, 3.0, 1.5), v(0.0, 2.0, 1.5), v(0.0, 2.0, 1.0),
        v(0.0, 1.0, 1.0), v(0.0, 1.0, 0.5), v(0.0, 0.0, 0.5)];
    let mut s = scene();
    let groups = s.prism(&profile, v(2.0, 0.0, 0.0));
    assert_eq!(groups.len(), 2 + profile.len());
    assert!(groups[0].len() > 1 && groups[0].len() <= 3, "{}", groups[0].len());
    for piece in groups.iter().flatten() { assert!(super::valid_face(piece)); }
    let out = s.render();
    assert!(s.seams(&out).is_empty(), "{:?}", s.seams(&out));
    assert!(s.construction_issues(1.5).is_empty());
    // Ni perfiles planos degenerados ni vectores tangentes.
    assert!(scene().prism(&profile, v(0.0, 1.0, 0.0)).is_empty());
    assert!(scene().prism(&profile[..2], v(1.0, 0.0, 0.0)).is_empty());
}

#[test]
fn hatching_a_split_face_gives_one_continuous_family_of_lines() {
    let whole = vec![v(0.0, 0.0, 0.0), v(4.0, 0.0, 0.0), v(4.0, 0.0, 3.0), v(0.0, 0.0, 3.0)];
    let pieces = [vec![v(0.0, 0.0, 0.0), v(4.0, 0.0, 0.0), v(4.0, 0.0, 3.0)], vec![v(0.0, 0.0, 0.0), v(4.0, 0.0, 3.0), v(0.0, 0.0, 3.0)]];
    // La cara mira a la cámara en el giro que sea necesario.
    let flip = !camera().facing(super::normal(&whole));
    let turned = |face: &Vec<V3>| { let mut f = face.clone(); if flip { f.reverse(); } f };
    let (whole, pieces) = (turned(&whole), pieces.iter().map(turned).collect::<Vec<_>>());
    let lines = |split: bool| {
        let mut s = scene();
        if split { s.hatch_faces(&pieces, v(1.0, 0.0, 0.3), 0.7); } else { s.hatch_face(&whole, v(1.0, 0.0, 0.3), 0.7); }
        s.render().parts[0].lines.clone()
    };
    let (one, two) = (lines(false), lines(true));
    assert!(one.len() > 10);
    assert_eq!(one.len(), two.len());
    for (a, b) in one.iter().zip(&two) {
        assert_eq!(b.len(), 2);
        assert!((length(a) - length(b)).abs() < 1e-6);
    }
}
