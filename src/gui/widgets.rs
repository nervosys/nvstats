//! Custom widgets for Silicon Monitor GUI
//!
//! Cyber-styled widgets for displaying hardware metrics
//! Now with Glances-style threshold colors and quicklook panel

use super::theme::{threshold_color, CyberColors};
use egui::epaint::PathShape;
use egui::{Color32, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2, Widget};

/// A cyber-styled progress bar with glow effect, animated pulse, and Glances-style threshold colors
pub struct CyberProgressBar {
    progress: f32,
    color: Color32,
    label: Option<String>,
    show_percentage: bool,
    height: f32,
    animated: bool,
    use_threshold_color: bool,
    trend: Option<&'static str>,
}

impl CyberProgressBar {
    pub fn new(progress: f32) -> Self {
        Self {
            progress: progress.clamp(0.0, 1.0),
            color: CyberColors::CYAN,
            label: None,
            show_percentage: true,
            height: 20.0,
            animated: true,
            use_threshold_color: false,
            trend: None,
        }
    }

    pub fn color(mut self, color: Color32) -> Self {
        self.color = color;
        self
    }

    /// Use Glances-style threshold colors based on percentage
    pub fn with_threshold_color(mut self) -> Self {
        self.use_threshold_color = true;
        self
    }

    /// Add a trend indicator (↑, ↓, or →)
    pub fn with_trend(mut self, trend: &'static str) -> Self {
        self.trend = Some(trend);
        self
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    #[allow(dead_code)]
    pub fn show_percentage(mut self, show: bool) -> Self {
        self.show_percentage = show;
        self
    }

    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    #[allow(dead_code)]
    pub fn animated(mut self, animated: bool) -> Self {
        self.animated = animated;
        self
    }
}

impl Widget for CyberProgressBar {
    fn ui(self, ui: &mut Ui) -> Response {
        let desired_size = Vec2::new(ui.available_width(), self.height);
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::hover());

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let time = ui.input(|i| i.time) as f32;

            // Use threshold color if enabled, otherwise use provided color
            let bar_color = if self.use_threshold_color {
                threshold_color(self.progress * 100.0)
            } else {
                self.color
            };

            // Background with gradient effect
            painter.rect_filled(rect, 4.0, CyberColors::BACKGROUND_DARK);

            // Subtle inner shadow
            let inner_shadow = Rect::from_min_size(
                rect.min + Vec2::new(1.0, 1.0),
                rect.size() - Vec2::new(2.0, 2.0),
            );
            painter.rect_filled(
                inner_shadow,
                3.0,
                Color32::from_rgba_unmultiplied(0, 0, 0, 40),
            );

            // Progress fill
            let fill_width = rect.width() * self.progress;
            if fill_width > 0.0 {
                let fill_rect = Rect::from_min_size(rect.min, Vec2::new(fill_width, rect.height()));

                // Multi-layer gradient fill using bar_color
                let dark_color = bar_color.linear_multiply(0.4);
                let bright_color = bar_color.linear_multiply(1.2);

                // Base fill
                painter.rect_filled(fill_rect.shrink(1.0), 3.0, dark_color);

                // Middle gradient band
                let mid_rect = Rect::from_min_size(
                    fill_rect.min + Vec2::new(1.0, rect.height() * 0.3),
                    Vec2::new(fill_width - 2.0, rect.height() * 0.4),
                );
                painter.rect_filled(mid_rect, 0.0, bar_color.linear_multiply(0.7));

                // Top highlight (glossy effect)
                let highlight_rect = Rect::from_min_size(
                    fill_rect.min + Vec2::new(1.0, 1.0),
                    Vec2::new(fill_width - 2.0, rect.height() * 0.35),
                );
                let highlight_color = Color32::from_rgba_unmultiplied(
                    bright_color.r(),
                    bright_color.g(),
                    bright_color.b(),
                    80,
                );
                painter.rect_filled(highlight_rect, 2.0, highlight_color);

                // Animated scanline effect
                if self.animated && fill_width > 10.0 {
                    let scan_pos = ((time * 2.0).sin() * 0.5 + 0.5) * fill_width;
                    let scan_width = 20.0;
                    if scan_pos > 0.0 && scan_pos < fill_width {
                        let scan_rect = Rect::from_min_size(
                            fill_rect.min + Vec2::new(scan_pos - scan_width / 2.0, 0.0),
                            Vec2::new(
                                scan_width.min(fill_width - scan_pos + scan_width / 2.0),
                                rect.height(),
                            ),
                        );
                        let scan_color = Color32::from_rgba_unmultiplied(255, 255, 255, 25);
                        painter.rect_filled(scan_rect.shrink(1.0), 2.0, scan_color);
                    }
                }

                // Glow effect on the edge (pulsing)
                let glow_x = rect.min.x + fill_width;
                if fill_width > 2.0 {
                    let pulse = if self.animated {
                        (time * 3.0).sin() * 0.3 + 0.7
                    } else {
                        1.0
                    };

                    for i in 0..5 {
                        let alpha = ((80.0 - i as f32 * 15.0) * pulse) as u8;
                        let glow_color = Color32::from_rgba_unmultiplied(
                            bar_color.r(),
                            bar_color.g(),
                            bar_color.b(),
                            alpha,
                        );
                        painter.vline(
                            glow_x - i as f32,
                            rect.y_range(),
                            Stroke::new(1.0, glow_color),
                        );
                    }

                    // Bright dot at the end
                    let dot_y = rect.center().y;
                    painter.circle_filled(
                        Pos2::new(glow_x - 1.0, dot_y),
                        3.0,
                        Color32::from_rgba_unmultiplied(255, 255, 255, (150.0 * pulse) as u8),
                    );
                }
            }

            // Outer border with subtle glow
            painter.rect_stroke(rect, 4.0, Stroke::new(1.0, CyberColors::BORDER));

            // Label and percentage with trend indicator
            let text_color = CyberColors::TEXT_PRIMARY;

            if let Some(label) = &self.label {
                // Add trend indicator to label if present
                let label_with_trend = if let Some(trend) = self.trend {
                    format!("{} {}", trend, label)
                } else {
                    label.clone()
                };

                // Text shadow
                painter.text(
                    Pos2::new(rect.min.x + 9.0, rect.center().y + 1.0),
                    egui::Align2::LEFT_CENTER,
                    &label_with_trend,
                    egui::FontId::proportional(12.0),
                    Color32::from_black_alpha(180),
                );
                painter.text(
                    Pos2::new(rect.min.x + 8.0, rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    &label_with_trend,
                    egui::FontId::proportional(12.0),
                    text_color,
                );
            }

            if self.show_percentage {
                let percent_text = format!("{:.1}%", self.progress * 100.0);
                // Text shadow
                painter.text(
                    Pos2::new(rect.max.x - 7.0, rect.center().y + 1.0),
                    egui::Align2::RIGHT_CENTER,
                    &percent_text,
                    egui::FontId::proportional(12.0),
                    Color32::from_black_alpha(180),
                );
                painter.text(
                    Pos2::new(rect.max.x - 8.0, rect.center().y),
                    egui::Align2::RIGHT_CENTER,
                    percent_text,
                    egui::FontId::proportional(12.0),
                    text_color,
                );
            }
        }

        // Request repaint for animation
        if self.animated {
            ui.ctx().request_repaint();
        }

        response
    }
}

/// A cyber-styled metric card
pub struct MetricCard<'a> {
    title: &'a str,
    value: String,
    unit: Option<&'a str>,
    color: Color32,
    icon: Option<&'a str>,
}

impl<'a> MetricCard<'a> {
    pub fn new(title: &'a str, value: impl std::fmt::Display) -> Self {
        Self {
            title,
            value: value.to_string(),
            unit: None,
            color: CyberColors::CYAN,
            icon: None,
        }
    }

    pub fn unit(mut self, unit: &'a str) -> Self {
        self.unit = Some(unit);
        self
    }

    pub fn color(mut self, color: Color32) -> Self {
        self.color = color;
        self
    }

    #[allow(dead_code)]
    pub fn icon(mut self, icon: &'a str) -> Self {
        self.icon = Some(icon);
        self
    }
}

impl Widget for MetricCard<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let desired_size = Vec2::new(140.0, 70.0);
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::hover());

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();

            // Card background
            painter.rect_filled(rect, 6.0, CyberColors::SURFACE);

            // Accent border on left
            let accent_rect = Rect::from_min_size(rect.min, Vec2::new(3.0, rect.height()));
            painter.rect_filled(
                accent_rect,
                egui::Rounding {
                    nw: 6.0,
                    sw: 6.0,
                    ne: 0.0,
                    se: 0.0,
                },
                self.color,
            );

            // Title
            painter.text(
                Pos2::new(rect.min.x + 12.0, rect.min.y + 12.0),
                egui::Align2::LEFT_TOP,
                self.title,
                egui::FontId::proportional(11.0),
                CyberColors::TEXT_SECONDARY,
            );

            // Value with unit
            let value_text = if let Some(unit) = self.unit {
                format!("{} {}", self.value, unit)
            } else {
                self.value.clone()
            };

            painter.text(
                Pos2::new(rect.min.x + 12.0, rect.max.y - 12.0),
                egui::Align2::LEFT_BOTTOM,
                value_text,
                egui::FontId::proportional(18.0),
                self.color,
            );
        }

        response
    }
}

/// Sparkline chart for historical data - sexy animated version
pub struct SparklineChart {
    data: Vec<f32>,
    color: Color32,
    height: f32,
    show_grid: bool,
    show_glow: bool,
    show_dots: bool,
    smooth: bool,
    gradient_fill: bool,
}

impl SparklineChart {
    pub fn new(data: Vec<f32>) -> Self {
        Self {
            data,
            color: CyberColors::CYAN,
            height: 60.0,
            show_grid: true,
            show_glow: true,
            show_dots: true,
            smooth: true,
            gradient_fill: true,
        }
    }

    pub fn color(mut self, color: Color32) -> Self {
        self.color = color;
        self
    }

    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    #[allow(dead_code)]
    pub fn show_grid(mut self, show: bool) -> Self {
        self.show_grid = show;
        self
    }

    #[allow(dead_code)]
    pub fn show_glow(mut self, show: bool) -> Self {
        self.show_glow = show;
        self
    }

    #[allow(dead_code)]
    pub fn smooth(mut self, smooth: bool) -> Self {
        self.smooth = smooth;
        self
    }
}

impl Widget for SparklineChart {
    fn ui(self, ui: &mut Ui) -> Response {
        let desired_size = Vec2::new(ui.available_width(), self.height);
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::hover());

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let time = ui.input(|i| i.time) as f32;

            // Background - very muted, nearly invisible
            let muted_bg = Color32::from_rgba_unmultiplied(5, 7, 10, 180);
            painter.rect_filled(rect, 6.0, muted_bg);

            // Very subtle inner shadow for depth
            let inner = rect.shrink(1.0);
            painter.rect_filled(
                Rect::from_min_size(inner.min, Vec2::new(inner.width(), 2.0)),
                0.0,
                Color32::from_rgba_unmultiplied(0, 0, 0, 15),
            );

            // Animated grid lines with pulse - very subtle
            if self.show_grid {
                let grid_pulse = (time * 0.5).sin() * 0.1 + 0.9;
                let grid_color = Color32::from_rgba_unmultiplied(
                    CyberColors::GRID.r(),
                    CyberColors::GRID.g(),
                    CyberColors::GRID.b(),
                    (20.0 * grid_pulse) as u8,
                );

                // Horizontal grid lines
                for i in 1..4 {
                    let y = rect.min.y + rect.height() * (i as f32 / 4.0);
                    painter.hline(rect.x_range(), y, Stroke::new(0.5, grid_color));
                }

                // Vertical grid lines (time markers)
                let num_vlines = 6;
                for i in 1..num_vlines {
                    let x = rect.min.x + rect.width() * (i as f32 / num_vlines as f32);
                    painter.vline(
                        x,
                        rect.y_range(),
                        Stroke::new(0.5, grid_color.linear_multiply(0.4)),
                    );
                }
            }

            // Draw sparkline
            if self.data.len() >= 2 {
                let max_val = self.data.iter().cloned().fold(1.0_f32, f32::max).max(0.01);
                let padding = 4.0;
                let graph_rect = rect.shrink(padding);

                // Calculate points
                let points: Vec<Pos2> = self
                    .data
                    .iter()
                    .enumerate()
                    .map(|(i, &v)| {
                        let x = graph_rect.min.x
                            + (i as f32 / (self.data.len() - 1) as f32) * graph_rect.width();
                        let normalized = (v / max_val).clamp(0.0, 1.0);
                        let y = graph_rect.max.y - normalized * graph_rect.height() * 0.95;
                        Pos2::new(x, y)
                    })
                    .collect();

                // Smooth the curve using Catmull-Rom spline interpolation
                // Higher subdivision = smoother curves (8 for silky smooth)
                let smooth_points = if self.smooth && points.len() >= 4 {
                    catmull_rom_spline(&points, 8)
                } else if self.smooth && points.len() >= 2 {
                    // For fewer points, still apply some smoothing
                    catmull_rom_spline(&points, 4)
                } else {
                    points.clone()
                };

                // Gradient fill under the line (multiple layers for depth)
                if self.gradient_fill {
                    let mut fill_points = smooth_points.clone();
                    fill_points.push(Pos2::new(graph_rect.max.x, graph_rect.max.y));
                    fill_points.push(Pos2::new(graph_rect.min.x, graph_rect.max.y));

                    // Layer 1: Dark base
                    let fill_color_dark = Color32::from_rgba_unmultiplied(
                        self.color.r(),
                        self.color.g(),
                        self.color.b(),
                        15,
                    );
                    painter.add(egui::Shape::convex_polygon(
                        fill_points.clone(),
                        fill_color_dark,
                        Stroke::NONE,
                    ));

                    // Layer 2: Gradient bands (simulated)
                    for layer in 0..3 {
                        let _band_height = graph_rect.height() * (0.3 - layer as f32 * 0.1);
                        let alpha = 20 - layer * 5;
                        let band_points: Vec<Pos2> = smooth_points
                            .iter()
                            .map(|p| {
                                let y_offset =
                                    (graph_rect.max.y - p.y) * (0.3 - layer as f32 * 0.1);
                                Pos2::new(p.x, (p.y + y_offset).min(graph_rect.max.y))
                            })
                            .collect();

                        let mut band_fill = band_points;
                        band_fill.push(Pos2::new(graph_rect.max.x, graph_rect.max.y));
                        band_fill.push(Pos2::new(graph_rect.min.x, graph_rect.max.y));

                        let band_color = Color32::from_rgba_unmultiplied(
                            self.color.r(),
                            self.color.g(),
                            self.color.b(),
                            alpha as u8,
                        );
                        painter.add(egui::Shape::convex_polygon(
                            band_fill,
                            band_color,
                            Stroke::NONE,
                        ));
                    }
                }

                // Glow effect under the line (animated) - using PathShape for smooth AA
                if self.show_glow {
                    let glow_pulse = (time * 2.0).sin() * 0.2 + 0.8;
                    for offset in 1..=4 {
                        let glow_alpha = ((50 - offset * 10) as f32 * glow_pulse) as u8;
                        let glow_color = Color32::from_rgba_unmultiplied(
                            self.color.r(),
                            self.color.g(),
                            self.color.b(),
                            glow_alpha,
                        );

                        // Offset points for glow layer
                        let glow_points: Vec<Pos2> = smooth_points
                            .iter()
                            .map(|p| Pos2::new(p.x, p.y + offset as f32))
                            .collect();

                        // Use PathShape for smooth connected line (no jagged joints)
                        let glow_path = PathShape::line(
                            glow_points,
                            Stroke::new(4.0 - offset as f32 * 0.7, glow_color),
                        );
                        painter.add(glow_path);
                    }
                }

                // Main line with anti-aliased stroke using PathShape (single connected path)
                let line_color = self.color;
                let main_path =
                    PathShape::line(smooth_points.clone(), Stroke::new(2.5, line_color));
                painter.add(main_path);

                // Highlight line (brighter, thinner) - also using PathShape
                let highlight_color = Color32::from_rgba_unmultiplied(
                    255.min(self.color.r() as u16 + 60) as u8,
                    255.min(self.color.g() as u16 + 60) as u8,
                    255.min(self.color.b() as u16 + 60) as u8,
                    180,
                );
                let highlight_points: Vec<Pos2> = smooth_points
                    .iter()
                    .map(|p| Pos2::new(p.x, p.y - 1.0))
                    .collect();
                let highlight_path =
                    PathShape::line(highlight_points, Stroke::new(1.0, highlight_color));
                painter.add(highlight_path);

                // Data point dots (only on original points, not interpolated)
                if self.show_dots && points.len() <= 30 {
                    for (i, point) in points.iter().enumerate() {
                        let is_last = i == points.len() - 1;
                        let dot_size = if is_last { 5.0 } else { 2.5 };

                        // Outer glow for dots
                        if is_last {
                            let pulse = (time * 4.0).sin() * 0.3 + 0.7;
                            for r in (1..=3).rev() {
                                let alpha = ((60 - r * 15) as f32 * pulse) as u8;
                                painter.circle_filled(
                                    *point,
                                    dot_size + r as f32 * 2.0,
                                    Color32::from_rgba_unmultiplied(
                                        self.color.r(),
                                        self.color.g(),
                                        self.color.b(),
                                        alpha,
                                    ),
                                );
                            }
                        }

                        // Inner dot
                        painter.circle_filled(*point, dot_size, self.color);

                        // Bright center
                        if is_last {
                            painter.circle_filled(
                                *point,
                                dot_size * 0.5,
                                Color32::from_rgb(255, 255, 255),
                            );
                        }
                    }
                }

                // Value label on hover or always for latest
                if let Some(&last_val) = self.data.last() {
                    if let Some(&last_point) = points.last() {
                        let label = format!("{:.1}", last_val);
                        let label_pos = Pos2::new(
                            (last_point.x - 25.0).max(rect.min.x + 5.0),
                            (last_point.y - 15.0).max(rect.min.y + 5.0),
                        );

                        // Label background
                        let label_rect = Rect::from_min_size(
                            label_pos - Vec2::new(2.0, 2.0),
                            Vec2::new(35.0, 16.0),
                        );
                        painter.rect_filled(
                            label_rect,
                            3.0,
                            Color32::from_rgba_unmultiplied(0, 0, 0, 180),
                        );

                        painter.text(
                            label_pos + Vec2::new(15.0, 6.0),
                            egui::Align2::CENTER_CENTER,
                            label,
                            egui::FontId::proportional(11.0),
                            self.color,
                        );
                    }
                }
            }

            // Border with subtle glow
            painter.rect_stroke(
                rect,
                6.0,
                Stroke::new(1.0, CyberColors::BORDER.linear_multiply(0.8)),
            );

            // Corner accents
            let corner_size = 8.0;
            let corner_color = self.color.linear_multiply(0.5);
            // Top-left
            painter.line_segment(
                [rect.min, rect.min + Vec2::new(corner_size, 0.0)],
                Stroke::new(2.0, corner_color),
            );
            painter.line_segment(
                [rect.min, rect.min + Vec2::new(0.0, corner_size)],
                Stroke::new(2.0, corner_color),
            );
            // Top-right
            painter.line_segment(
                [
                    Pos2::new(rect.max.x, rect.min.y),
                    Pos2::new(rect.max.x - corner_size, rect.min.y),
                ],
                Stroke::new(2.0, corner_color),
            );
            painter.line_segment(
                [
                    Pos2::new(rect.max.x, rect.min.y),
                    Pos2::new(rect.max.x, rect.min.y + corner_size),
                ],
                Stroke::new(2.0, corner_color),
            );
            // Bottom-left
            painter.line_segment(
                [
                    Pos2::new(rect.min.x, rect.max.y),
                    Pos2::new(rect.min.x + corner_size, rect.max.y),
                ],
                Stroke::new(2.0, corner_color),
            );
            painter.line_segment(
                [
                    Pos2::new(rect.min.x, rect.max.y),
                    Pos2::new(rect.min.x, rect.max.y - corner_size),
                ],
                Stroke::new(2.0, corner_color),
            );
            // Bottom-right
            painter.line_segment(
                [rect.max, rect.max - Vec2::new(corner_size, 0.0)],
                Stroke::new(2.0, corner_color),
            );
            painter.line_segment(
                [rect.max, rect.max - Vec2::new(0.0, corner_size)],
                Stroke::new(2.0, corner_color),
            );
        }

        // Request repaint for animation
        if self.show_glow {
            ui.ctx().request_repaint();
        }

        response
    }
}

/// Catmull-Rom spline interpolation for smooth curves
fn catmull_rom_spline(points: &[Pos2], subdivisions: usize) -> Vec<Pos2> {
    if points.len() < 2 {
        return points.to_vec();
    }

    let mut result = Vec::new();

    for i in 0..points.len() - 1 {
        let p0 = if i == 0 { points[0] } else { points[i - 1] };
        let p1 = points[i];
        let p2 = points[i + 1];
        let p3 = if i + 2 < points.len() {
            points[i + 2]
        } else {
            points[points.len() - 1]
        };

        for j in 0..=subdivisions {
            let t = j as f32 / subdivisions as f32;
            let t2 = t * t;
            let t3 = t2 * t;

            let x = 0.5
                * ((2.0 * p1.x)
                    + (-p0.x + p2.x) * t
                    + (2.0 * p0.x - 5.0 * p1.x + 4.0 * p2.x - p3.x) * t2
                    + (-p0.x + 3.0 * p1.x - 3.0 * p2.x + p3.x) * t3);

            let y = 0.5
                * ((2.0 * p1.y)
                    + (-p0.y + p2.y) * t
                    + (2.0 * p0.y - 5.0 * p1.y + 4.0 * p2.y - p3.y) * t2
                    + (-p0.y + 3.0 * p1.y - 3.0 * p2.y + p3.y) * t3);

            if j == 0 && i > 0 {
                continue; // Skip duplicate points
            }
            result.push(Pos2::new(x, y));
        }
    }

    result
}

/// Section header with cyber styling
pub struct SectionHeader<'a> {
    title: &'a str,
    icon: Option<&'a str>,
}

impl<'a> SectionHeader<'a> {
    pub fn new(title: &'a str) -> Self {
        Self { title, icon: None }
    }

    pub fn icon(mut self, icon: &'a str) -> Self {
        self.icon = Some(icon);
        self
    }
}

impl Widget for SectionHeader<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let desired_size = Vec2::new(ui.available_width(), 28.0);
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::hover());

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();

            // Title with icon
            let title_text = if let Some(icon) = self.icon {
                format!("{} {}", icon, self.title)
            } else {
                self.title.to_string()
            };

            painter.text(
                Pos2::new(rect.min.x, rect.center().y),
                egui::Align2::LEFT_CENTER,
                title_text,
                egui::FontId::proportional(14.0),
                CyberColors::CYAN,
            );

            // Decorative line
            let line_start = rect.min.x
                + painter
                    .layout_no_wrap(
                        self.title.to_string(),
                        egui::FontId::proportional(14.0),
                        CyberColors::CYAN,
                    )
                    .rect
                    .width()
                + 20.0;

            painter.hline(
                line_start..=rect.max.x,
                rect.center().y,
                Stroke::new(1.0, CyberColors::BORDER),
            );
        }

        response
    }
}

/// Glances-style QuickLook summary panel
/// Shows CPU, MEM, SWAP, LOAD in a compact horizontal bar format
pub struct QuickLookPanel {
    cpu_percent: f32,
    mem_percent: f32,
    swap_percent: f32,
    load_1m: f32,
    cpu_trend: Option<&'static str>,
    mem_trend: Option<&'static str>,
}

impl QuickLookPanel {
    pub fn new(cpu: f32, mem: f32, swap: f32, load: f32) -> Self {
        Self {
            cpu_percent: cpu.clamp(0.0, 100.0),
            mem_percent: mem.clamp(0.0, 100.0),
            swap_percent: swap.clamp(0.0, 100.0),
            load_1m: load,
            cpu_trend: None,
            mem_trend: None,
        }
    }

    pub fn with_trends(mut self, cpu_trend: &'static str, mem_trend: &'static str) -> Self {
        self.cpu_trend = Some(cpu_trend);
        self.mem_trend = Some(mem_trend);
        self
    }
}

impl Widget for QuickLookPanel {
    fn ui(self, ui: &mut Ui) -> Response {
        let desired_size = Vec2::new(ui.available_width(), 32.0);
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::hover());

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();

            // Background
            painter.rect_filled(rect, 4.0, CyberColors::SURFACE);

            // Calculate bar widths (4 equal sections)
            let section_width = rect.width() / 4.0 - 8.0;
            let bar_height = 16.0;
            let y_center = rect.center().y;

            // Draw each metric
            let metrics = [
                ("CPU", self.cpu_percent, self.cpu_trend),
                ("MEM", self.mem_percent, self.mem_trend),
                ("SWAP", self.swap_percent, None),
                ("LOAD", (self.load_1m * 10.0).min(100.0), None), // Scale load to 0-100
            ];

            for (i, (label, percent, trend)) in metrics.iter().enumerate() {
                let x_start = rect.min.x + 4.0 + i as f32 * (section_width + 8.0);

                // Label with trend
                let label_text = if let Some(t) = trend {
                    format!("{} {}", t, label)
                } else {
                    label.to_string()
                };

                painter.text(
                    Pos2::new(x_start, y_center - 6.0),
                    egui::Align2::LEFT_CENTER,
                    &label_text,
                    egui::FontId::proportional(10.0),
                    CyberColors::TEXT_SECONDARY,
                );

                // Mini bar
                let bar_rect = Rect::from_min_size(
                    Pos2::new(x_start, y_center + 2.0),
                    Vec2::new(section_width * 0.6, bar_height * 0.5),
                );
                painter.rect_filled(bar_rect, 2.0, CyberColors::BACKGROUND_DARK);

                let fill_width = bar_rect.width() * (percent / 100.0);
                if fill_width > 0.0 {
                    let fill_rect =
                        Rect::from_min_size(bar_rect.min, Vec2::new(fill_width, bar_rect.height()));
                    painter.rect_filled(fill_rect, 2.0, threshold_color(*percent));
                }

                // Percentage text
                let percent_text = if *label == "LOAD" {
                    format!("{:.2}", self.load_1m)
                } else {
                    format!("{:.0}%", percent)
                };

                painter.text(
                    Pos2::new(x_start + section_width * 0.65, y_center + 2.0),
                    egui::Align2::LEFT_CENTER,
                    percent_text,
                    egui::FontId::proportional(11.0),
                    threshold_color(*percent),
                );
            }

            // Border
            painter.rect_stroke(rect, 4.0, Stroke::new(1.0, CyberColors::BORDER));
        }

        response
    }
}

/// Glances-style threshold legend
pub struct ThresholdLegend;

impl Widget for ThresholdLegend {
    fn ui(self, ui: &mut Ui) -> Response {
        let desired_size = Vec2::new(ui.available_width(), 20.0);
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::hover());

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let y = rect.center().y;

            let items = [
                ("OK", CyberColors::THRESHOLD_OK, "0-50%"),
                ("CAREFUL", CyberColors::THRESHOLD_CAREFUL, "50-70%"),
                ("WARNING", CyberColors::THRESHOLD_WARNING, "70-90%"),
                ("CRITICAL", CyberColors::THRESHOLD_CRITICAL, "90%+"),
            ];

            let mut x = rect.min.x;
            for (label, color, range) in items {
                // Color dot
                painter.circle_filled(Pos2::new(x + 6.0, y), 4.0, color);

                // Label
                painter.text(
                    Pos2::new(x + 14.0, y),
                    egui::Align2::LEFT_CENTER,
                    label,
                    egui::FontId::proportional(10.0),
                    color,
                );

                // Range
                let label_width = painter
                    .layout_no_wrap(label.to_string(), egui::FontId::proportional(10.0), color)
                    .rect
                    .width();

                painter.text(
                    Pos2::new(x + 16.0 + label_width, y),
                    egui::Align2::LEFT_CENTER,
                    range,
                    egui::FontId::proportional(9.0),
                    CyberColors::TEXT_MUTED,
                );

                x += 90.0;
            }
        }

        response
    }
}
