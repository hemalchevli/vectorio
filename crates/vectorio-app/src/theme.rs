//! Aspecto inspirado en macOS 27: barra de herramientas uniforme, paneles flotantes de
//! vidrio legible (translúcidos pero con tinte suficiente), radios concéntricos y
//! tipografía del sistema. Sigue el modo claro/oscuro del sistema.

use egui::{epaint::Shadow, Color32, FontFamily, FontId, Rounding, Stroke, TextStyle, Vec2};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Appearance {
    #[default]
    #[serde(rename = "sistema")]
    System,
    #[serde(rename = "claro")]
    Light,
    #[serde(rename = "oscuro")]
    Dark,
}

impl Appearance {
    pub const ALL: [Self; 3] = [Self::System, Self::Light, Self::Dark];
    pub fn label(self) -> String { match self { Self::System => tr!("Sistema"), Self::Light => tr!("Claro"), Self::Dark => tr!("Oscuro") } }
    pub fn apply(self, ctx: &egui::Context) {
        ctx.set_theme(match self { Self::System => egui::ThemePreference::System, Self::Light => egui::ThemePreference::Light, Self::Dark => egui::ThemePreference::Dark });
        ctx.request_repaint();
    }
}

/// Radios concéntricos: el control interior vale el radio del panel menos su margen.
pub const PANEL_RADIUS: f32 = 22.0;
pub const PANEL_PAD: f32 = 12.0;
pub const CONTROL_RADIUS: f32 = PANEL_RADIUS - PANEL_PAD;
/// Hojas modales (Ajustes, Atajos, Color, Foto): algo más grandes que un panel.
pub const SHEET_RADIUS: f32 = 26.0;
pub const SHEET_PAD: f32 = 20.0;
/// Separación entre los paneles y el borde de la ventana.
pub const INSET: f32 = 10.0;
pub const TOOLBAR_H: f32 = 52.0;
/// Hueco para los botones de cerrar/minimizar/ampliar.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub const TRAFFIC_LIGHTS_W: f32 = 78.0;

/// Colores con alfa «ingenuos»: premultiplicados en el espacio gamma, que es en el que
/// mezcla egui. `Color32::from_white_alpha` y `from_rgba_unmultiplied` premultiplican en
/// lineal y aclaran mucho los blancos translúcidos, lo que vuelve imprevisible el vidrio.
pub fn rgba(r: u8, g: u8, b: u8, a: u8) -> Color32 {
    let p = |v: u8| (v as f32 * a as f32 / 255.0).round() as u8;
    Color32::from_rgba_premultiplied(p(r), p(g), p(b), a)
}

pub fn white(a: u8) -> Color32 {
    Color32::from_rgba_premultiplied(a, a, a, a)
}

#[derive(Clone, Copy)]
pub struct Palette {
    /// Color medio del lienzo (para mezclas y controles desactivados).
    pub canvas: Color32,
    /// El lienzo es un degradado vertical muy suave entre estos dos.
    pub canvas_top: Color32,
    pub canvas_bottom: Color32,
    /// Cuerpo translúcido de paneles y cápsulas.
    pub glass: Color32,
    /// Cuerpo más denso para hojas y menús.
    pub glass_sheet: Color32,
    /// Velo que aclara la parte superior del cuerpo (degradado de ≤ 4 %).
    pub glass_tint_top: Color32,
    /// Brillo especular del borde: intenso hacia la luz (arriba a la izquierda) y un
    /// contrabrillo débil en el lado opuesto.
    pub rim_hi: Color32,
    pub rim_lo: Color32,
    /// Línea interior oscura en la mitad inferior; da grosor sin biselar.
    pub inner_shade: Color32,
    /// Trazo exterior de 1 px.
    pub outline: Color32,
    /// Sombra ambiental (amplia) y de contacto (corta y nítida).
    pub shadow: Color32,
    pub shadow_contact: Color32,
    /// Superficies opacas que deben leerse siempre igual: barra superior y menús.
    pub bar: Color32,
    /// Tarjetas de tinta sobre el vidrio.
    pub card: Color32,
    pub text: Color32,
    pub secondary: Color32,
    /// Pistas y textos de ayuda.
    pub tertiary: Color32,
    pub accent: Color32,
    pub accent_soft: Color32,
    /// Campos y segmentos sin seleccionar: rellenos con alfa, válidos sobre cualquier vidrio.
    pub control: Color32,
    pub control_hover: Color32,
    /// Pomo del segmento seleccionado.
    pub knob: Color32,
    pub separator: Color32,
}

impl Palette {
    /// Color opaco de una superficie translúcida colocada directamente sobre el lienzo.
    pub fn solid(&self, c: Color32) -> Color32 {
        over(c, self.canvas)
    }
}

pub fn palette(dark: bool) -> Palette {
    let accent_soft = |a: u8, c: Color32| rgba(c.r(), c.g(), c.b(), a);
    if dark {
        let accent = Color32::from_rgb(10, 132, 255);
        Palette {
            canvas: Color32::from_rgb(19, 19, 21),
            canvas_top: Color32::from_rgb(22, 22, 25),
            canvas_bottom: Color32::from_rgb(16, 16, 18),
            glass: rgba(44, 44, 50, 184),
            glass_sheet: rgba(38, 38, 42, 240),
            glass_tint_top: white(10),
            rim_hi: white(87),
            rim_lo: white(36),
            inner_shade: Color32::from_black_alpha(89),
            outline: Color32::from_black_alpha(153),
            shadow: Color32::from_black_alpha(115),
            shadow_contact: Color32::from_black_alpha(128),
            bar: Color32::from_rgb(38, 38, 42),
            card: white(18),
            text: Color32::from_rgb(245, 245, 247),
            secondary: Color32::from_rgb(174, 174, 182),
            tertiary: Color32::from_rgb(128, 128, 136),
            accent,
            accent_soft: accent_soft(56, accent),
            control: white(20),
            control_hover: white(31),
            knob: white(51),
            separator: white(26),
        }
    } else {
        let accent = Color32::from_rgb(0, 122, 255);
        Palette {
            canvas: Color32::from_rgb(234, 234, 238),
            canvas_top: Color32::from_rgb(238, 238, 242),
            canvas_bottom: Color32::from_rgb(230, 230, 235),
            glass: rgba(255, 255, 255, 189),
            glass_sheet: rgba(250, 250, 252, 240),
            glass_tint_top: white(26),
            rim_hi: white(255),
            rim_lo: white(140),
            inner_shade: Color32::from_black_alpha(13),
            outline: Color32::from_black_alpha(18),
            shadow: Color32::from_black_alpha(31),
            shadow_contact: Color32::from_black_alpha(20),
            bar: Color32::from_rgb(250, 250, 252),
            card: white(179),
            text: Color32::from_rgb(29, 29, 31),
            secondary: Color32::from_rgb(96, 96, 104),
            tertiary: Color32::from_rgb(124, 124, 132),
            accent,
            accent_soft: accent_soft(41, accent),
            control: Color32::from_black_alpha(13),
            control_hover: Color32::from_black_alpha(20),
            knob: Color32::WHITE,
            separator: Color32::from_black_alpha(20),
        }
    }
}

pub fn current(ctx: &egui::Context) -> Palette {
    palette(ctx.style().visuals.dark_mode)
}

/// Fondo de la ventana: degradado vertical casi imperceptible. Da algo de profundidad
/// al vidrio sin competir con el papel.
pub fn paint_canvas(painter: &egui::Painter, rect: egui::Rect, pal: &Palette) {
    let mut m = egui::Mesh::default();
    m.colored_vertex(rect.left_top(), pal.canvas_top);
    m.colored_vertex(rect.right_top(), pal.canvas_top);
    m.colored_vertex(rect.right_bottom(), pal.canvas_bottom);
    m.colored_vertex(rect.left_bottom(), pal.canvas_bottom);
    m.add_triangle(0, 1, 2);
    m.add_triangle(0, 2, 3);
    painter.add(egui::Shape::mesh(m));
}

/// Velo bajo la barra de herramientas: el color del lienzo, denso arriba y transparente
/// unos píxeles por debajo de la barra, para que los controles se lean sobre el papel.
pub fn toolbar_scrim(painter: &egui::Painter, bar: egui::Rect, pal: &Palette) {
    let mut m = egui::Mesh::default();
    let bottom = bar.bottom() + 12.0;
    let top = pal.canvas_top.gamma_multiply(0.85);
    m.colored_vertex(bar.left_top(), top);
    m.colored_vertex(bar.right_top(), top);
    m.colored_vertex(egui::pos2(bar.right(), bottom), Color32::TRANSPARENT);
    m.colored_vertex(egui::pos2(bar.left(), bottom), Color32::TRANSPARENT);
    m.add_triangle(0, 1, 2);
    m.add_triangle(0, 2, 3);
    painter.add(egui::Shape::mesh(m));
}

const BACKDROP: &str = "glass-backdrop";

/// El lienzo anota dónde está el papel (en pantalla) y de qué color, para que el vidrio
/// que lo cubra lo refleje. `None` cuando no hay papel a la vista (modo Salida).
pub fn set_backdrop(ctx: &egui::Context, paper: Option<(egui::Rect, Color32)>) {
    ctx.data_mut(|d| match paper {
        Some(v) => d.insert_temp(egui::Id::new(BACKDROP), v),
        None => d.remove::<(egui::Rect, Color32)>(egui::Id::new(BACKDROP)),
    });
}

fn backdrop(ctx: &egui::Context) -> Option<(egui::Rect, Color32)> {
    ctx.data(|d| d.get_temp(egui::Id::new(BACKDROP)))
}

/// Hoja modal (Ajustes, Atajos, Color): material `Sheet`, radio y margen de hoja.
pub fn glass_sheet<R>(ui: &mut egui::Ui, pal: &Palette, add: impl FnOnce(&mut egui::Ui) -> R) -> egui::InnerResponse<R> {
    glass_panel_with(ui, pal, SHEET_RADIUS, egui::Margin::same(SHEET_PAD), Material::Sheet, add)
}

/// Velo que oscurece la ventana tras una hoja modal.
pub fn veil(dark: bool) -> Color32 {
    Color32::from_black_alpha(if dark { 115 } else { 64 })
}

/// Material de una superficie flotante. Todas comparten sombra doble, cuerpo casi
/// uniforme, borde especular y línea de profundidad; cambia la densidad del cuerpo.
#[derive(Clone, Copy)]
#[allow(dead_code)] // Popup lo reservan los menús, que egui dibuja con `window_fill`.
pub enum Material {
    Panel,
    Capsule,
    Sheet,
    Popup,
    Tinted(Color32),
}

/// Panel flotante de vidrio líquido. egui no puede desenfocar lo que hay debajo, así
/// que el efecto combina un cuerpo translúcido, un borde especular y una sombra doble.
pub fn glass_panel<R>(ui: &mut egui::Ui, pal: &Palette, add: impl FnOnce(&mut egui::Ui) -> R) -> egui::InnerResponse<R> {
    glass_panel_with(ui, pal, PANEL_RADIUS, egui::Margin::same(PANEL_PAD), Material::Panel, add)
}

pub fn glass_panel_with<R>(ui: &mut egui::Ui, pal: &Palette, radius: f32, margin: egui::Margin, material: Material, add: impl FnOnce(&mut egui::Ui) -> R) -> egui::InnerResponse<R> {
    let background = ui.painter().add(egui::Shape::Noop);
    let see_through = ui.painter().add(egui::Shape::Noop);
    let inner = egui::Frame::none().inner_margin(margin).show(ui, add);
    let rect = inner.response.rect;
    ui.painter().set(background, glass(rect, radius, material, pal));
    // El reflejo comparte la silueta redondeada del vidrio. El recorte de egui
    // solo admite rectángulos: usarlo como máscara dejaba parches en las esquinas.
    if let Some((paper, color)) = backdrop(ui.ctx()) {
        if paper.expand(24.0).intersects(rect) {
            let strength = if ui.visuals().dark_mode { 0.2 } else { 0.35 };
            ui.painter().set(see_through, glass_backdrop(rect.shrink(1.0), (radius - 1.0).max(0.0), paper, color, strength));
        }
    }
    inner
}

/// Mancha difusa del papel, triangulada dentro del mismo contorno que el vidrio.
/// La distancia al papel suaviza el reflejo sin pintar fuera de las esquinas.
fn glass_backdrop(rect: egui::Rect, radius: f32, paper: egui::Rect, color: Color32, strength: f32) -> egui::Shape {
    let tint = |point: egui::Pos2| {
        let dx = (paper.left() - point.x).max(point.x - paper.right()).max(0.0);
        let dy = (paper.top() - point.y).max(point.y - paper.bottom()).max(0.0);
        let fade = (-(dx * dx + dy * dy) / (2.0 * 12.0 * 12.0)).exp();
        color.gamma_multiply(strength * fade)
    };
    let outline = rounded_outline(rect, radius);
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(rect.center(), tint(rect.center()));
    for (point, _) in &outline {
        mesh.colored_vertex(*point, tint(*point));
    }
    let n = outline.len() as u32;
    for i in 0..n {
        mesh.add_triangle(0, i + 1, 1 + (i + 1) % n);
    }
    egui::Shape::mesh(mesh)
}

/// Composición «a sobre b» con colores premultiplicados.
fn over(top: Color32, bottom: Color32) -> Color32 {
    let k = 1.0 - top.a() as f32 / 255.0;
    let c = |t: u8, b: u8| (t as f32 + b as f32 * k).round().min(255.0) as u8;
    Color32::from_rgba_premultiplied(c(top.r(), bottom.r()), c(top.g(), bottom.g()), c(top.b(), bottom.b()), c(top.a(), bottom.a()))
}

fn plus(a: Color32, b: Color32) -> Color32 {
    Color32::from_rgba_premultiplied(a.r().saturating_add(b.r()), a.g().saturating_add(b.g()), a.b().saturating_add(b.b()), a.a().saturating_add(b.a()))
}

fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_premultiplied(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()), l(a.a(), b.a()))
}

/// Dirección de la luz, de arriba a la izquierda (y crece hacia abajo en pantalla).
const LIGHT: Vec2 = Vec2::new(-0.6, -0.8);
const ARC_STEPS: usize = 12;

/// Pesos del brillo especular para una normal exterior: el principal crece hacia la
/// luz y el contrabrillo hacia el lado opuesto.
fn rim_weights(normal: Vec2) -> (f32, f32) {
    let k = 0.5 + 0.5 * normal.dot(LIGHT);
    (k.powi(3), (1.0 - k).powi(3))
}

/// Contorno de un rectángulo redondeado en sentido horario, con su normal exterior.
fn rounded_outline(rect: egui::Rect, radius: f32) -> Vec<(egui::Pos2, Vec2)> {
    let r = radius.min(rect.width() / 2.0).min(rect.height() / 2.0).max(0.0);
    let corners = [
        (rect.right_top() + Vec2::new(-r, r), -90.0f32),
        (rect.right_bottom() + Vec2::new(-r, -r), 0.0),
        (rect.left_bottom() + Vec2::new(r, -r), 90.0),
        (rect.left_top() + Vec2::new(r, r), 180.0),
    ];
    let mut points = Vec::with_capacity(4 * (ARC_STEPS + 1));
    for (center, start) in corners {
        for i in 0..=ARC_STEPS {
            let a = (start + i as f32 * 90.0 / ARC_STEPS as f32).to_radians();
            let dir = Vec2::new(a.cos(), a.sin());
            points.push((center + dir * r, dir));
        }
    }
    points
}

/// Banda entre dos contornos con el mismo número de puntos.
fn ring(outer: &[(egui::Pos2, Vec2)], inner: &[(egui::Pos2, Vec2)], color: impl Fn(egui::Pos2, Vec2) -> Color32) -> egui::Shape {
    let mut mesh = egui::Mesh::default();
    for ((o, n), (i, _)) in outer.iter().zip(inner) {
        let c = color(*o, *n);
        mesh.colored_vertex(*o, c);
        mesh.colored_vertex(*i, c);
    }
    let n = outer.len() as u32;
    for k in 0..n {
        let (a, b) = (2 * k, 2 * ((k + 1) % n));
        mesh.add_triangle(a, a + 1, b);
        mesh.add_triangle(a + 1, b + 1, b);
    }
    egui::Shape::mesh(mesh)
}

pub fn glass(rect: egui::Rect, radius: f32, material: Material, pal: &Palette) -> egui::Shape {
    let (base, tint) = match material {
        Material::Panel | Material::Capsule => (pal.glass, pal.glass_tint_top),
        Material::Sheet | Material::Popup => (pal.glass_sheet, pal.glass_tint_top),
        Material::Tinted(c) => (c, white(36)),
    };
    let top = over(tint, base);
    let rel = |y: f32| ((y - rect.top()) / rect.height().max(1.0)).clamp(0.0, 1.0);
    let outer = rounded_outline(rect, radius);
    let rim_inner = rounded_outline(rect.shrink(1.0), (radius - 1.0).max(0.0));
    let shade_inner = rounded_outline(rect.shrink(2.0), (radius - 2.0).max(0.0));
    // Cuerpo: abanico desde el centro con un degradado vertical muy leve.
    let mut fill = egui::Mesh::default();
    fill.colored_vertex(rect.center(), mix(top, base, rel(rect.center().y)));
    for (p, _) in &outer {
        fill.colored_vertex(*p, mix(top, base, rel(p.y)));
    }
    let n = outer.len() as u32;
    for i in 0..n {
        fill.add_triangle(0, 1 + i, 1 + (i + 1) % n);
    }
    // Línea interior: solo en la mitad inferior, suavizada.
    let depth = ring(&rim_inner, &shade_inner, |p, _| {
        let t = ((rel(p.y) - 0.4) / 0.6).clamp(0.0, 1.0);
        pal.inner_shade.gamma_multiply(t * t * (3.0 - 2.0 * t))
    });
    // Borde especular: brillo hacia la luz y contrabrillo en el lado opuesto.
    let rim = ring(&outer, &rim_inner, |_, normal| {
        let (hi, lo) = rim_weights(normal);
        plus(pal.rim_hi.gamma_multiply(hi), pal.rim_lo.gamma_multiply(lo))
    });
    // Nota: las sombras también pasan por debajo del cuerpo translúcido y lo oscurecen
    // un poco; está calibrado en la paleta.
    // epaint no admite un desenfoque mayor que el lado corto: en piezas pequeñas
    // (Exportar, cápsulas) el borde interior de la sombra se cerraba en una línea y
    // salían picos en las esquinas. La sombra ambiental escala con la pieza.
    let blur = 32.0f32.min(rect.size().min_elem() * 0.7);
    let ambient = Shadow { offset: Vec2::new(0.0, 10.0 * blur / 32.0), blur, spread: 0.0, color: pal.shadow };
    let contact = Shadow { offset: Vec2::new(0.0, 1.0), blur: 3.0, spread: 0.0, color: pal.shadow_contact };
    let rounding = Rounding::same(radius);
    egui::Shape::Vec(vec![
        ambient.as_shape(rect, rounding).into(),
        contact.as_shape(rect, rounding).into(),
        egui::Shape::mesh(fill),
        depth,
        rim,
        outline_path(&outer, Stroke::new(1.0f32, pal.outline)),
    ])
}

/// Contorno cerrado sin puntos repetidos. Con radio igual a media altura, los
/// tramos rectos miden cero y el trazo de egui saca picos en las juntas.
fn outline_path(points: &[(egui::Pos2, Vec2)], stroke: Stroke) -> egui::Shape {
    let mut path: Vec<egui::Pos2> = Vec::with_capacity(points.len());
    for (p, _) in points {
        if path.last().is_none_or(|q| q.distance(*p) > 0.05) { path.push(*p); }
    }
    if path.len() > 1 && path[0].distance(path[path.len() - 1]) <= 0.05 { path.pop(); }
    egui::Shape::closed_line(path, stroke)
}

/// egui pinta el pomo del deslizador con el relleno del carril y el color del texto,
/// y la casilla con el radio de los botones. Al acabar el fotograma se cambian esas
/// formas por un pomo opaco con sombra y una casilla cuadrada con borde.
pub fn restyle_controls(ctx: &egui::Context) {
    let style = ctx.style();
    let w = &style.visuals.widgets;
    let states = [&w.inactive, &w.hovered, &w.active];
    let box_size = style.spacing.icon_width;
    let dark = style.visuals.dark_mode;
    let mut layers: Vec<egui::LayerId> = ctx.memory(|m| m.layer_ids().collect());
    layers.push(egui::LayerId::background());
    let knob = if dark { Color32::from_gray(214) } else { Color32::WHITE };
    let knob_line = if dark { Color32::from_black_alpha(90) } else { Color32::from_black_alpha(38) };
    let box_line = if dark { white(56) } else { Color32::from_black_alpha(46) };
    ctx.graphics_mut(|g| {
        for layer in layers {
            let Some(list) = g.get_mut(layer) else { continue };
            for i in 0..list.next_idx().0 {
                list.mutate_shape(egui::layers::ShapeIdx(i), |clipped| match &mut clipped.shape {
                    egui::Shape::Circle(c) if states.iter().any(|s| c.fill == s.bg_fill && c.stroke == s.fg_stroke) => {
                        let (center, r) = (c.center, c.radius);
                        clipped.shape = egui::Shape::Vec(vec![
                            egui::Shape::circle_filled(center + Vec2::new(0.0, 1.0), r + 0.5, Color32::from_black_alpha(if dark { 70 } else { 28 })),
                            egui::Shape::circle_filled(center, r, knob),
                            egui::Shape::circle_stroke(center, r, Stroke::new(0.5f32, knob_line)),
                        ]);
                    }
                    egui::Shape::Rect(r) if (r.rect.width() - r.rect.height()).abs() < 0.5
                        && (r.rect.width() - box_size).abs() < 2.5
                        && states.iter().any(|s| r.fill == s.bg_fill && r.stroke == s.bg_stroke) => {
                        r.rounding = Rounding::same(4.0);
                        r.stroke = Stroke::new(1.0f32, box_line);
                    }
                    _ => {}
                });
            }
        }
    });
}

fn visuals(dark: bool) -> egui::Visuals {
    let pal = palette(dark);
    let mut v = if dark { egui::Visuals::dark() } else { egui::Visuals::light() };
    v.panel_fill = pal.canvas;
    v.window_fill = pal.bar;
    v.window_stroke = Stroke::new(1.0f32, pal.outline);
    v.window_rounding = Rounding::same(PANEL_RADIUS);
    v.window_shadow = Shadow { offset: Vec2::new(0.0, 10.0), blur: 32.0, spread: 0.0, color: pal.shadow };
    v.popup_shadow = Shadow { offset: Vec2::new(0.0, 8.0), blur: 20.0, spread: 0.0, color: pal.shadow };
    v.menu_rounding = Rounding::same(CONTROL_RADIUS + 2.0);
    v.extreme_bg_color = pal.control;
    v.faint_bg_color = pal.control;
    v.selection.bg_fill = pal.accent_soft;
    v.selection.stroke = Stroke::new(1.0f32, pal.accent);
    v.hyperlink_color = pal.accent;
    v.override_text_color = Some(pal.text);
    let r = Rounding::same(CONTROL_RADIUS);
    for (w, fill, stroke) in [
        (&mut v.widgets.noninteractive, Color32::TRANSPARENT, pal.separator),
        (&mut v.widgets.inactive, pal.control, Color32::TRANSPARENT),
        (&mut v.widgets.hovered, pal.control_hover, Color32::TRANSPARENT),
        (&mut v.widgets.active, pal.accent_soft, Color32::TRANSPARENT),
        (&mut v.widgets.open, pal.control_hover, Color32::TRANSPARENT),
    ] {
        w.rounding = r;
        w.bg_fill = fill;
        w.weak_bg_fill = fill;
        w.bg_stroke = Stroke::new(1.0f32, stroke);
        w.expansion = 0.0;
    }
    // egui atenúa los controles desactivados hacia este color; si fuese transparente,
    // no los dibujaría.
    v.widgets.noninteractive.weak_bg_fill = pal.canvas;
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0f32, pal.text);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0f32, pal.text);
    v.widgets.hovered.fg_stroke = Stroke::new(1.0f32, pal.text);
    v.widgets.active.fg_stroke = Stroke::new(1.0f32, pal.accent);
    v.slider_trailing_fill = true;
    v
}

fn style(dark: bool) -> egui::Style {
    let mut s = egui::Style { visuals: visuals(dark), ..Default::default() };
    s.spacing.item_spacing = Vec2::new(8.0, 7.0);
    s.spacing.button_padding = Vec2::new(10.0, 5.0);
    s.spacing.interact_size = Vec2::new(36.0, 26.0);
    s.spacing.slider_width = 140.0;
    s.spacing.combo_width = 150.0;
    s.text_styles = [
        (TextStyle::Small, FontId::new(11.0, FontFamily::Proportional)),
        (TextStyle::Body, FontId::new(13.0, FontFamily::Proportional)),
        (TextStyle::Button, FontId::new(13.0, FontFamily::Proportional)),
        (TextStyle::Heading, FontId::new(17.0, FontFamily::Proportional)),
        (TextStyle::Monospace, FontId::new(12.0, FontFamily::Monospace)),
    ]
    .into();
    s
}

/// Tipografía del sistema (SF Pro) si está disponible; si no, la de egui.
fn fonts() -> egui::FontDefinitions {
    let mut fonts = egui::FontDefinitions::default();
    for (name, path) in [("sf", "/System/Library/Fonts/SFNS.ttf"), ("sfmono", "/System/Library/Fonts/SFNSMono.ttf")] {
        if let Ok(bytes) = std::fs::read(path) {
            fonts.font_data.insert(name.into(), egui::FontData::from_owned(bytes));
            let family = if name == "sf" { FontFamily::Proportional } else { FontFamily::Monospace };
            fonts.families.entry(family).or_default().insert(0, name.into());
        }
    }
    fonts
}

pub fn install(ctx: &egui::Context) {
    ctx.set_fonts(fonts());
    ctx.set_style_of(egui::Theme::Light, style(false));
    ctx.set_style_of(egui::Theme::Dark, style(true));
    ctx.options_mut(|o| o.theme_preference = egui::ThemePreference::System);
}

/// Título de sección en el estilo de las barras laterales de macOS.
pub fn section(ui: &mut egui::Ui, text: &str) {
    let pal = current(ui.ctx());
    ui.add_space(4.0);
    ui.label(egui::RichText::new(text).size(11.5).color(pal.secondary).strong());
}

/// Botón de acento (acción principal): cápsula de vidrio teñido con el azul del sistema.
pub fn accent_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    accent_button_sized(ui, text, None)
}

pub fn accent_button_sized(ui: &mut egui::Ui, text: &str, size: Option<Vec2>) -> egui::Response {
    let pal = current(ui.ctx());
    let font = FontId::new(13.0, FontFamily::Proportional);
    let galley = ui.painter().layout_no_wrap(text.to_string(), font, Color32::WHITE);
    let size = size.unwrap_or_else(|| Vec2::new(galley.size().x + 28.0, 28.0));
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    let enabled = ui.is_enabled();
    let base = if !enabled {
        pal.accent.gamma_multiply(0.45)
    } else if resp.is_pointer_button_down_on() {
        mix(pal.accent, Color32::BLACK, 0.08)
    } else if resp.hovered() {
        mix(pal.accent, Color32::WHITE, 0.06)
    } else {
        pal.accent
    };
    ui.painter().add(glass(rect, rect.height() / 2.0, Material::Tinted(base), &pal));
    let color = if enabled { Color32::WHITE } else { Color32::WHITE.gamma_multiply(0.7) };
    ui.painter().galley(rect.center() - galley.size() / 2.0, galley, color);
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, text));
    focus_ring(ui, &resp);
    resp
}

/// Flecha de los desplegables: un chevron fino en lugar del triángulo de egui.
pub fn combo_icon(ui: &egui::Ui, rect: egui::Rect, visuals: &egui::style::WidgetVisuals, is_open: bool, _above_or_below: egui::AboveOrBelow) {
    let _ = visuals;
    let pal = current(ui.ctx());
    let c = rect.center();
    let dy = if is_open { 2.0 } else { -2.0 };
    let pts = vec![c + Vec2::new(-4.0, dy), c + Vec2::new(0.0, -dy), c + Vec2::new(4.0, dy)];
    ui.painter().add(egui::Shape::line(pts, Stroke::new(1.5f32, pal.secondary)));
}

/// Muestra de color redonda.
pub fn swatch(ui: &mut egui::Ui, color: [u8; 3], size: f32) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::click());
    let pal = current(ui.ctx());
    ui.painter().circle_filled(rect.center(), size / 2.0, Color32::from_rgb(color[0], color[1], color[2]));
    ui.painter().circle_stroke(rect.center(), size / 2.0, Stroke::new(1.0f32, pal.outline));
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::ColorButton, ui.is_enabled(),
        tr!("Editar color %{color}", color = format!("#{:02X}{:02X}{:02X}", color[0], color[1], color[2]))));
    focus_ring(ui, &resp);
    resp
}

/// El foco de teclado no depende del estado de paso del puntero.
pub fn focus_ring(ui: &egui::Ui, response: &egui::Response) {
    if response.has_focus() && response.enabled() {
        ui.painter().rect_stroke(response.rect.shrink(1.0), 4.0, Stroke::new(2.0f32, current(ui.ctx()).accent));
    }
}

pub fn pen_color(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

pub fn close_button(ui: &mut egui::Ui) -> egui::Response {
    let pal = current(ui.ctx());
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(28.0), egui::Sense::click());
    ui.painter().circle_filled(rect.center(), 12.0, if response.hovered() { pal.control_hover } else { pal.control });
    let center = rect.center();
    let stroke = Stroke::new(1.5f32, pal.secondary);
    ui.painter().line_segment([center + Vec2::new(-4.0, -4.0), center + Vec2::new(4.0, 4.0)], stroke);
    ui.painter().line_segment([center + Vec2::new(-4.0, 4.0), center + Vec2::new(4.0, -4.0)], stroke);
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), "Cerrar"));
    focus_ring(ui, &response);
    response.on_hover_text("Cerrar sin guardar (Esc)")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn manual_appearance_overrides_system_and_system_mode_tracks_it_again() {
        let ctx = egui::Context::default();
        install(&ctx);
        for (appearance, system, expected_dark) in [
            (Appearance::Light, egui::Theme::Dark, false),
            (Appearance::Dark, egui::Theme::Light, true),
            (Appearance::System, egui::Theme::Light, false),
            (Appearance::System, egui::Theme::Dark, true),
        ] {
            let saved = serde_json::to_vec(&appearance).unwrap();
            let restored: Appearance = serde_json::from_slice(&saved).unwrap();
            restored.apply(&ctx);
            let _ = ctx.run(egui::RawInput { system_theme: Some(system), ..Default::default() }, |ctx| {
                assert_eq!(ctx.style().visuals.dark_mode, expected_dark);
                assert_eq!(current(ctx).canvas, palette(expected_dark).canvas);
            });
        }
    }

    fn luminance(c: Color32) -> f32 {
        let f = |v: u8| { let v = v as f32 / 255.0; if v <= 0.03928 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) } };
        0.2126 * f(c.r()) + 0.7152 * f(c.g()) + 0.0722 * f(c.b())
    }

    fn contrast(a: Color32, b: Color32) -> f32 {
        let (la, lb) = (luminance(a), luminance(b));
        (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
    }

    #[test]
    fn texts_are_readable_on_glass_and_on_the_bare_canvas() {
        for dark in [false, true] {
            let pal = palette(dark);
            for canvas in [pal.canvas_top, pal.canvas_bottom] {
                for glass in [pal.glass, pal.glass_sheet] {
                    let bg = over(over(pal.glass_tint_top, glass), canvas);
                    for (name, fg, min) in [("text", pal.text, 7.0), ("secondary", pal.secondary, 4.5), ("tertiary", pal.tertiary, 3.0)] {
                        let c = contrast(fg, bg);
                        assert!(c >= min, "{name} sobre vidrio (oscuro: {dark}): {c:.2} < {min}; bg={bg:?} fg={fg:?}");
                    }
                }
                for (name, fg, min) in [("text", pal.text, 7.0), ("secondary", pal.secondary, 4.5), ("tertiary", pal.tertiary, 3.0)] {
                    let c = contrast(fg, canvas);
                    assert!(c >= min, "{name} sobre el lienzo (oscuro: {dark}): {c:.2} < {min}");
                }
            }
        }
    }

    #[test]
    fn rim_peaks_towards_the_light_and_is_symmetric_around_its_axis() {
        let at = |deg: f32| { let a = deg.to_radians(); rim_weights(Vec2::new(a.cos(), a.sin())) };
        let light_deg = LIGHT.y.atan2(LIGHT.x).to_degrees();
        let best = (0..360).max_by(|a, b| at(*a as f32).0.total_cmp(&at(*b as f32).0)).unwrap() as f32;
        let diff = ((best - light_deg + 540.0) % 360.0 - 180.0).abs();
        assert!(diff <= 1.0, "el máximo está a {best}° y la luz a {light_deg}°");
        for d in [15.0f32, 40.0, 90.0, 130.0] {
            let (a, b) = (at(light_deg + d), at(light_deg - d));
            assert!((a.0 - b.0).abs() < 1e-4 && (a.1 - b.1).abs() < 1e-4);
        }
        let (hi, lo) = at(light_deg + 180.0);
        assert!(hi < 0.01 && lo > 0.99, "el lado opuesto solo lleva contrabrillo");
    }

    #[test]
    fn paper_reflection_never_fills_the_corners_outside_the_glass() {
        for size in [Vec2::new(220.0, 36.0), Vec2::new(500.0, 430.0), Vec2::splat(36.0)] {
            let rect = egui::Rect::from_min_size(egui::pos2(50.0, 60.0), size);
            for radius in [8.0, 18.0, 40.0] {
                for paper in [rect.expand(100.0), rect.translate(Vec2::new(size.x * 0.6, 0.0))] {
                    let egui::Shape::Mesh(mesh) = glass_backdrop(rect, radius, paper, Color32::WHITE, 0.35) else { panic!("El reflejo debe ser una malla redondeada") };
                    let covers = |point: egui::Pos2| mesh.indices.as_chunks::<3>().0.iter().any(|triangle| {
                        let a = mesh.vertices[triangle[0] as usize].pos;
                        let b = mesh.vertices[triangle[1] as usize].pos;
                        let c = mesh.vertices[triangle[2] as usize].pos;
                        let cross = |a: Vec2, b: Vec2| a.x * b.y - a.y * b.x;
                        let area = cross(b - a, c - a);
                        if area.abs() < 1e-6 { return false; }
                        let u = cross(point - a, c - a) / area;
                        let v = cross(b - a, point - a) / area;
                        u >= 0.0 && v >= 0.0 && u + v <= 1.0
                    });
                    assert!(covers(rect.center()));
                    for corner in [rect.left_top(), rect.right_top(), rect.left_bottom(), rect.right_bottom()] {
                        let point = corner + (rect.center() - corner).normalized() * 0.5;
                        assert!(!covers(point), "El reflejo sale de la esquina con radio {radius}");
                    }
                    assert!(mesh.vertices.iter().any(|v| v.color.a() > 0));
                }
            }
        }
    }

    #[test]
    fn glass_shapes_have_matching_rings_for_every_material() {
        let pal = palette(true);
        let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), Vec2::new(200.0, 40.0));
        for m in [Material::Panel, Material::Capsule, Material::Sheet, Material::Popup, Material::Tinted(pal.accent)] {
            // Un radio mayor que media altura se recorta sin romper el contorno.
            let shape = glass(rect, 40.0, m, &pal);
            assert!(matches!(shape, egui::Shape::Vec(ref v) if v.len() == 6));
        }
    }
}
