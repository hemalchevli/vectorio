use super::{cross, Camera, Scene, SceneOutput, ScenePart, V3, DEPTH_TOLERANCE_M, FACE_INSET_MM, HOLE_TOLERANCE_M, MIN_STROKE_MM};
use crate::geom::{length, p, BBox, Polyline, P};

#[derive(Clone, Copy, Debug)]
pub(super) struct Vertex { pub point: P, pub depth: f64 }
impl From<(P, f64)> for Vertex { fn from((point, depth): (P, f64)) -> Self { Self { point, depth } } }

#[derive(Clone, Debug)]
pub(super) struct Face {
    pub points: Vec<P>,
    pub bbox: BBox,
    pub min_depth: f64,
    pub max_depth: f64,
    pub solid_id: Option<u64>,
    pub owner: Option<(u64, super::PartId)>,
    pub drawn_boundaries: Vec<(P,P)>,
    pub normal: V3,
    pub world_origin: V3,
    pub origin: Vertex,
    pub gradient: P,
    pub planes: Vec<(P, P)>,
}
impl Face {
    pub fn new(world: &[V3], camera: &Camera) -> Option<Self> {
        if !super::valid_face(world) { return None; }
        let clipped=camera.clip_polygon(world);let world=clipped.as_slice();
        if world.len()<3{return None;}
        let vertices: Vec<Vertex> = world.iter().map(|v| Vertex::from(camera.project(*v))).collect();
        let origin = vertices[0];
        let mut gradient = None;
        for pair in vertices[1..].windows(2) {
            let (u, v) = (pair[0].point - origin.point, pair[1].point - origin.point);
            let det = cross(u, v);
            if det.abs() <= 1e-10 { continue; }
            let (du, dv) = (pair[0].depth - origin.depth, pair[1].depth - origin.depth);
            gradient = Some(p((du * v.y - dv * u.y) / det, (u.x * dv - v.x * du) / det));
            break;
        }
        let gradient = gradient?;
        let mut points: Vec<_> = vertices.iter().map(|v| v.point).collect();
        let area: f64 = points[1..].windows(2).map(|w| cross(w[0] - points[0], w[1] - points[0])).sum();
        let longest = points.iter().zip(points.iter().cycle().skip(1)).take(points.len())
            .map(|(a, b)| a.dist(*b)).fold(0.0, f64::max);
        if !area.is_finite() || area.abs() / longest.max(1e-12) < 1e-6 { return None; }
        if area < 0.0 { points.reverse(); }
        let mut bbox = BBox::empty();
        let mut planes = Vec::new();
        for (a, b) in points.iter().zip(points.iter().cycle().skip(1)).take(points.len()) {
            bbox.add(*a);
            let edge = *b - *a;
            if edge.len() > 1e-12 { planes.push((*a, p(-edge.y, edge.x) * (1.0 / edge.len()))); }
        }
        Some(Self { drawn_boundaries:Vec::new(), solid_id: None, owner: None, normal: super::normal(world), world_origin: world[0],
            max_depth: vertices.iter().map(|v|v.depth).fold(f64::NEG_INFINITY,f64::max), points, bbox, min_depth: vertices.iter().map(|v| v.depth).fold(f64::INFINITY, f64::min),
            origin, gradient, planes })
    }
    pub fn depth(&self, point: P) -> f64 { self.origin.depth + (point - self.origin.point).dot(self.gradient) }
    /// Cyrus–Beck sobre semiplanos interiores normalizados, en mm.
    pub fn clip(&self, a: P, b: P, inset: f64) -> Option<(f64, f64)> {
        let mut interval = (0.0, 1.0);
        for (origin, normal) in &self.planes {
            if !clip_positive(&mut interval, (a - *origin).dot(*normal) - inset, (b - a).dot(*normal)) { return None; }
        }
        Some(interval)
    }
}

// Interseca el intervalo con f(t) >= 0. La función afín evita muestrear profundidad.
pub(super) fn clip_positive(interval: &mut (f64, f64), start: f64, delta: f64) -> bool {
    if delta.abs() < 1e-15 { return start >= 0.0; }
    let t = -start / delta;
    if delta > 0.0 { interval.0 = interval.0.max(t); } else { interval.1 = interval.1.min(t); }
    interval.0 <= interval.1
}

pub(super) fn clip_paper(a: P, b: P, paper: BBox) -> Option<(f64, f64)> {
    let mut interval = (0.0, 1.0);
    let d = b - a;
    for (start, delta) in [(a.x - paper.min.x, d.x), (paper.max.x - a.x, -d.x),
        (a.y - paper.min.y, d.y), (paper.max.y - a.y, -d.y)] {
        if !clip_positive(&mut interval, start, delta) { return None; }
    }
    Some(interval)
}

pub(super) fn merge_intervals(intervals: &mut Vec<(f64, f64)>) {
    intervals.sort_unstable_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
    let mut n = 0;
    for i in 0..intervals.len() {
        let next = intervals[i];
        if n > 0 && next.0 <= intervals[n - 1].1 + 1e-12 { intervals[n - 1].1 = intervals[n - 1].1.max(next.1); }
        else { intervals[n] = next; n += 1; }
    }
    intervals.truncate(n);
}

pub(super) struct Grid {
    paper: BBox,
    cell: f64,
    pub cols: usize,
    rows: usize,
    pub cells: Vec<Vec<usize>>,
    stamps: Vec<u64>,
    stamp: u64,
}
impl Grid {
    pub fn new(paper: BBox, faces: &[Face]) -> Self {
        let cell = 4.0f64.max(paper.width() / 1024.0).max(paper.height() / 1024.0);
        let (cols, rows) = ((paper.width() / cell).ceil() as usize, (paper.height() / cell).ceil() as usize);
        let mut grid = Self { paper, cell, cols, rows, cells: vec![Vec::new(); cols * rows], stamps: vec![0; faces.len()], stamp: 0 };
        for (i, face) in faces.iter().enumerate() {
            if let Some((x0, y0, x1, y1)) = grid.range(face.bbox) {
                for y in y0..=y1 { for x in x0..=x1 { grid.cells[y * cols + x].push(i); } }
            }
        }
        grid
    }
    pub fn range(&self, bounds: BBox) -> Option<(usize, usize, usize, usize)> {
        if bounds.max.x < self.paper.min.x || bounds.min.x > self.paper.max.x ||
            bounds.max.y < self.paper.min.y || bounds.min.y > self.paper.max.y { return None; }
        let x = |v: f64| (((v - self.paper.min.x) / self.cell).max(0.0) as usize).min(self.cols - 1);
        let y = |v: f64| (((v - self.paper.min.y) / self.cell).max(0.0) as usize).min(self.rows - 1);
        Some((x(bounds.min.x), y(bounds.min.y), x(bounds.max.x), y(bounds.max.y)))
    }
    // DDA: solo celdas atravesadas, incluyendo ambos vecinos en esquinas y
    // líneas sobre una frontera. La holgura cubre el índice de pasadas coincidentes.
    fn segment_cells(&self, a: P, b: P) -> Vec<usize> {
        let Some((lo, hi)) = clip_paper(a, b, self.paper) else { return Vec::new(); };
        let start = (a.lerp(b, lo) - self.paper.min) * (1.0 / self.cell);
        let stop = (a.lerp(b, hi) - self.paper.min) * (1.0 / self.cell);
        let d = stop - start;
        let mut x = start.x.floor() as i64;
        let mut y = start.y.floor() as i64;
        let (sx, sy) = (if d.x >= 0.0 { 1 } else { -1 }, if d.y >= 0.0 { 1 } else { -1 });
        let (dx, dy) = (if d.x.abs() < 1e-10 { f64::INFINITY } else { 1.0 / d.x.abs() },
            if d.y.abs() < 1e-10 { f64::INFINITY } else { 1.0 / d.y.abs() });
        let mut tx = if dx.is_infinite() { f64::INFINITY } else if sx > 0 { (x as f64 + 1.0 - start.x) * dx } else { (start.x - x as f64) * dx };
        let mut ty = if dy.is_infinite() { f64::INFINITY } else if sy > 0 { (y as f64 + 1.0 - start.y) * dy } else { (start.y - y as f64) * dy };
        let mut cells = Vec::new();
        let mut add = |x: i64, y: i64| {
            if x >= 0 && y >= 0 && x < self.cols as i64 && y < self.rows as i64 { cells.push(y as usize * self.cols + x as usize); }
        };
        for _ in 0..self.cols + self.rows + 4 {
            add(x, y);
            if d.x.abs() < 1e-10 && (start.x - start.x.round()).abs() < 1e-8 { add(x - 1, y); }
            if d.y.abs() < 1e-10 && (start.y - start.y.round()).abs() < 1e-8 { add(x, y - 1); }
            if tx.min(ty) > 1.0 + 1e-10 { break; }
            if (tx - ty).abs() < 1e-10 { add(x + sx, y); add(x, y + sy); x += sx; y += sy; tx += dx; ty += dy; }
            else if tx < ty { x += sx; tx += dx; } else { y += sy; ty += dy; }
        }
        cells
    }
    fn candidates(&mut self, a: P, b: P, indices: &mut Vec<usize>) {
        indices.clear();
        self.stamp = self.stamp.wrapping_add(1);
        if self.stamp == 0 { self.stamps.fill(0); self.stamp = 1; }
        for cell in self.segment_cells(a, b) {
            for &i in &self.cells[cell] {
                if self.stamps[i] != self.stamp { self.stamps[i] = self.stamp; indices.push(i); }
            }
        }
    }
    fn insert(&mut self, a: P, b: P) {
        let i = self.stamps.len(); self.stamps.push(0);
        for cell in self.segment_cells(a, b) { self.cells[cell].push(i); }
    }
    fn hidden(&mut self, a: Vertex, b: Vertex, faces: &[Face], sheet: (f64, f64), intervals: &mut Vec<(f64, f64)>) {
        self.stamp = self.stamp.wrapping_add(1);
        if self.stamp == 0 { self.stamps.fill(0); self.stamp = 1; }
        let mut bounds = BBox::empty();
        bounds.add(a.point.lerp(b.point, sheet.0)); bounds.add(a.point.lerp(b.point, sheet.1));
        for cell in self.segment_cells(a.point.lerp(b.point, sheet.0), a.point.lerp(b.point, sheet.1)) {
            for &i in &self.cells[cell] {
                if self.stamps[i] == self.stamp { continue; }
                self.stamps[i] = self.stamp;
                let face = &faces[i];
                if face.min_depth >= a.depth.max(b.depth) - DEPTH_TOLERANCE_M { continue; }
                if face.bbox.min.x > bounds.max.x || face.bbox.max.x < bounds.min.x ||
                    face.bbox.min.y > bounds.max.y || face.bbox.max.y < bounds.min.y { continue; }
                let Some(mut interval) = face.clip(a.point, b.point, FACE_INSET_MM) else { continue; };
                interval.0 = interval.0.max(sheet.0); interval.1 = interval.1.min(sheet.1);
                let f0 = a.depth - face.depth(a.point) - DEPTH_TOLERANCE_M;
                let df = b.depth - a.depth - (b.point - a.point).dot(face.gradient);
                if clip_positive(&mut interval, f0, df) && interval.1 > interval.0 {
                    intervals.push(interval);
                }
            }
        }
    }
}

impl Grid {
    /// Tramos del segmento dentro de un hueco y a su profundidad (± HOLE_TOLERANCE_M).
    fn cut(&mut self, a: Vertex, b: Vertex, holes: &[Face], sheet: (f64, f64), intervals: &mut Vec<(f64, f64)>) {
        self.stamp = self.stamp.wrapping_add(1);
        if self.stamp == 0 { self.stamps.fill(0); self.stamp = 1; }
        let (near, far) = (a.depth.min(b.depth), a.depth.max(b.depth));
        for cell in self.segment_cells(a.point.lerp(b.point, sheet.0), a.point.lerp(b.point, sheet.1)) {
            for &i in &self.cells[cell] {
                if self.stamps[i] == self.stamp { continue; }
                self.stamps[i] = self.stamp;
                let hole = &holes[i];
                if hole.min_depth > far + HOLE_TOLERANCE_M || hole.max_depth < near - HOLE_TOLERANCE_M { continue; }
                let Some(mut interval) = hole.clip(a.point, b.point, 0.0) else { continue; };
                interval.0 = interval.0.max(sheet.0); interval.1 = interval.1.min(sheet.1);
                // Diferencia de profundidad con el plano del hueco: d0 + t·dd, en [-tol, tol].
                let d0 = a.depth - hole.depth(a.point);
                let dd = b.depth - a.depth - (b.point - a.point).dot(hole.gradient);
                if clip_positive(&mut interval, d0 + HOLE_TOLERANCE_M, dd) &&
                    clip_positive(&mut interval, HOLE_TOLERANCE_M - d0, -dd) && interval.1 > interval.0 {
                    intervals.push(interval);
                }
            }
        }
    }
}

fn flush(line: &mut Polyline, output: &mut Vec<Polyline>) {
    if length(line) >= MIN_STROKE_MM { output.push(std::mem::take(line)); } else { line.clear(); }
}

// Dos sólidos pegados pueden aportar la misma arista (también solo un tramo).
// Conserva la primera pasada de cada parte sin unir objetos ni cambiar su orden.
// La rejilla evita comparar entre sí todos los segmentos de una hoja grande.
fn remove_overlaps(lines: &mut Vec<Polyline>, paper: BBox, cancel: &dyn Fn() -> bool) -> bool {
    let mut index = Grid::new(paper, &[]);
    let mut edges: Vec<(P, P, bool)> = Vec::new();
    let mut candidates = Vec::new();
    let mut covered = Vec::new();
    let mut pending_ids = Vec::new();
    let finish = |current: &mut Polyline, ids: &mut Vec<usize>, edges: &mut Vec<(P, P, bool)>, out: &mut Vec<Polyline>| {
        if length(current) < MIN_STROKE_MM { for &i in ids.iter() { edges[i].2 = false; } }
        flush(current, out);
        ids.clear();
    };
    for source in std::mem::take(lines) {
        if cancel() { return false; }
        let mut current: Polyline = Vec::new();
        for pair in source.windows(2) {
            if cancel() { return false; }
            let (a, b) = (pair[0], pair[1]);
            let d = b - a;
            let len = d.len();
            if len < 1e-12 { continue; }
            let unit = d * (1.0 / len);
            index.candidates(a, b, &mut candidates);
            covered.clear();
            for &i in &candidates {
                let (c, e, active) = edges[i];
                if !active || cross(unit, c - a).abs() > 1e-8 || cross(unit, e - a).abs() > 1e-8 { continue; }
                let (t, u) = ((c - a).dot(unit) / len, (e - a).dot(unit) / len);
                let interval = (t.min(u).max(0.0), t.max(u).min(1.0));
                if interval.1 > interval.0 { covered.push(interval); }
            }
            merge_intervals(&mut covered);
            covered.push((1.0, 1.0));
            let mut cursor = 0.0;
            for &(lo, hi) in &covered {
                if lo > cursor {
                    let (start, end) = (a.lerp(b, cursor), a.lerp(b, lo));
                    if current.last().is_some_and(|q| q.dist(start) > 1e-8) {
                        finish(&mut current, &mut pending_ids, &mut edges, lines);
                    }
                    if current.is_empty() { current.push(start); }
                    current.push(end);
                    pending_ids.push(edges.len());
                    index.insert(start, end);
                    edges.push((start, end, true));
                }
                cursor = cursor.max(hi);
            }
            if covered.iter().any(|&(lo, hi)| hi >= 1.0 && lo < hi) {
                finish(&mut current, &mut pending_ids, &mut edges, lines);
            }
        }
        finish(&mut current, &mut pending_ids, &mut edges, lines);
    }
    true
}

pub(super) fn render(scene: &Scene) -> SceneOutput {
    render_cancellable(scene, &|| false).expect("sin cancelación")
}

/// Expande intervalos de oclusión en longitud de arco, también a través de vértices.
fn stroke_halo(stroke:&super::Stroke, scene:&Scene, grid:&mut Grid, cancel:&dyn Fn()->bool)->Option<Vec<(f64,f64)>> {
    let mut intervals=Vec::new();let mut hidden=Vec::new();let mut distance=0.;
    for pair in stroke.vertices.windows(2){
        if cancel(){return None;}let(a,b)=(pair[0],pair[1]);let len=a.point.dist(b.point);
        if len>1e-12 {if let Some(sheet)=clip_paper(a.point,b.point,scene.paper){hidden.clear();grid.hidden(a,b,&scene.faces,sheet,&mut hidden);for(lo,hi)in &hidden{intervals.push((distance+lo*len-scene.halo_mm,distance+hi*len+scene.halo_mm));}}}
        distance+=len;
    }
    if stroke.vertices.first().zip(stroke.vertices.last()).is_some_and(|(a,b)|a.point.dist(b.point)<1e-8)&&distance>0.{
        let copies=intervals.iter().copied().collect::<Vec<_>>();for(lo,hi)in copies{if lo<0.{intervals.push((distance+lo,distance));}if hi>distance{intervals.push((0.,hi-distance));}}
    }
    for interval in &mut intervals{interval.0=interval.0.max(0.);interval.1=interval.1.min(distance);}merge_intervals(&mut intervals);Some(intervals)
}

pub(super) fn render_cancellable(scene: &Scene, cancel: &dyn Fn() -> bool) -> Option<SceneOutput> {
    if cancel() { return None; }
    let mut output = SceneOutput { parts: scene.names.iter().enumerate().map(|(i, name)| ScenePart {
        id: super::PartId(i as u16), name: name.clone(), lines: Vec::new(),
    }).collect() };
    let mut grid = Grid::new(scene.paper, &scene.faces);
    let mut holes = (!scene.holes.is_empty()).then(|| Grid::new(scene.paper, &scene.holes));
    let mut hidden = Vec::new();
    let mut visible = Vec::new();
    let intersections = if scene.intersections { super::intersections::strokes(scene, &grid) } else { Vec::new() };
    for stroke in scene.strokes.iter().chain(&intersections) {
        if cancel() { return None; }
        let mut lines = Vec::new();
        let mut current = Vec::new();
        let mut rejected = false;
        let halo=if scene.halo_mm>0.&&!scene.hidden_lines&&!stroke.fill&&!stroke.whole{Some(stroke_halo(stroke,scene,&mut grid,cancel)?)}else{None};
        let mut distance=0.;
        for segment in stroke.vertices.windows(2) {
            if cancel() { return None; }
            let (a, b) = (segment[0], segment[1]);
            let len=a.point.dist(b.point);let start=distance;distance+=len;
            if len < 1e-12 { continue; }
            let Some(sheet) = clip_paper(a.point, b.point, scene.paper) else { rejected = true; flush(&mut current, &mut lines); continue; };
            hidden.clear();
            if let Some(halo)=&halo {for(lo,hi)in halo{let lo=((lo-start)/len).max(sheet.0);let hi=((hi-start)/len).min(sheet.1);if hi>lo{hidden.push((lo,hi));}}}
            else {grid.hidden(a,b,&scene.faces,sheet,&mut hidden);}
            if let Some(holes) = holes.as_mut().filter(|_| !stroke.whole && !stroke.fill) {
                holes.cut(a, b, &scene.holes, sheet, &mut hidden);
            }
            merge_intervals(&mut hidden);
            if sheet.0 > 0.0 || sheet.1 < 1.0 || !hidden.is_empty() { rejected = true; }
            if stroke.whole && rejected { break; }
            visible.clear();
            let mut cursor = sheet.0;
            for &(lo, hi) in &hidden {
                if lo > cursor { visible.push((cursor, lo)); }
                cursor = cursor.max(hi);
            }
            if cursor < sheet.1 { visible.push((cursor, sheet.1)); }
            if scene.hidden_lines { visible.clone_from(&hidden); }
            for &(lo, hi) in &visible {
                let (start, end) = (a.point.lerp(b.point, lo), a.point.lerp(b.point, hi));
                if current.last().is_some_and(|q: &P| q.dist(start) > 1e-8) { flush(&mut current, &mut lines); }
                if current.is_empty() { current.push(start); }
                current.push(end);
            }
            if visible.last().is_none_or(|v| v.1 < 1.0) { flush(&mut current, &mut lines); }
        }
        flush(&mut current, &mut lines);
        if !stroke.whole || !rejected { output.parts[stroke.part.0 as usize].lines.extend(lines); }
    }
    for part in &mut output.parts {
        if cancel() { return None; }
        if !remove_overlaps(&mut part.lines, scene.paper, cancel) { return None; }
    }
    Some(output)
}

#[cfg(test)]
mod grid_tests {
    use super::*;
    #[test]
    fn dda_covers_diagonals_corners_and_cell_boundaries() {
        let paper=BBox{min:p(0.0,0.0),max:p(210.0,297.0)};
        let camera=Camera::default();let mut faces=Vec::new();
        for y in 0..74 {for x in 0..52{
            let (x,y)=(x as f64*4.0,y as f64*4.0);
            let face=[p(x,y),p(x+3.9,y),p(x+3.9,y+3.9),p(x,y+3.9)].map(|q|camera.unproject(q,0.0));
            faces.push(Face::new(&face,&camera).unwrap());
        }}
        let mut grid=Grid::new(paper,&faces);let mut candidates=Vec::new();
        for (a,b) in [(p(0.0,0.0),p(210.0,297.0)),(p(208.0,296.0),p(0.0,0.0)),
            (p(4.0,0.0),p(4.0,297.0)),(p(0.0,4.0),p(210.0,4.0))]{
            grid.candidates(a,b,&mut candidates);
            for (i,face) in faces.iter().enumerate(){
                if face.clip(a,b,0.0).is_some_and(|(lo,hi)|hi-lo>1e-12){assert!(candidates.contains(&i),"Celda omitida {i}");}
            }
            assert!(grid.segment_cells(a,b).len()<4*(grid.cols+grid.rows+4));
        }
    }
}
