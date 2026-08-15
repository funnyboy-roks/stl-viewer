use std::{
    fs,
    path::{Path, PathBuf},
};

use rl::{
    prelude::*,
    rlgl::{self, DrawingMode},
    text::{Font, FontFileType},
};

use crate::stl::{Solid, parse_stl};

mod stl;

fn draw_axis(cam: &mut impl DrawTarget3D, axis: Vector3, color: Color) {
    let thicc = 0.025;
    cam.draw_cylinder(axis * -100., axis * 100., thicc, thicc, 16, color);
}

fn draw_xy_grid(_cam: &mut impl DrawTarget3D, slices: u32, spacing: f32) {
    let half_slices = slices as i32 / 2;

    rlgl::drawing_mode(DrawingMode::Lines, |ctx| {
        for i in -half_slices..=half_slices {
            ctx.color(Vector3::ONE * 0.25);

            ctx.vertex(Vector3::new(
                i as f32 * spacing,
                -half_slices as f32 * spacing,
                0.0,
            ))
            .vertex(Vector3::new(
                i as f32 * spacing,
                half_slices as f32 * spacing,
                0.0,
            ));

            ctx.vertex(Vector3::new(
                -half_slices as f32 * spacing,
                i as f32 * spacing,
                0.0,
            ))
            .vertex(Vector3::new(
                half_slices as f32 * spacing,
                i as f32 * spacing,
                0.0,
            ));
        }
    });
}

const COLOURS: &[(Color, &str)] = &[
    (Color::PURPLE, "Purple"),
    (Color::WHITE, "White"),
    (Color::BLUE, "Blue"),
    (Color::DARKGREEN, "Dark Green"),
    (Color::GREEN, "Green"),
    (Color::LIME, "Lime"),
    (Color::MAROON, "Maroon"),
    (Color::ORANGE, "Orange"),
    (Color::PINK, "Pink"),
    (Color::RED, "Red"),
    (Color::VIOLET, "Violet"),
];

struct LoadedStl {
    solid: Solid,
    avg: Vector3,
    min: Vector3,
    max: Vector3,
}

fn load_stl(path: impl AsRef<Path>) -> Result<LoadedStl, Box<dyn std::error::Error>> {
    let content = fs::read(path)?;

    let mut avg = Vector3::new(0., 0., 0.);
    let mut min = Vector3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
    let mut max = Vector3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);

    let solid = parse_stl(&content)?;

    for f in &solid.facets {
        let v1 = Vector3::from(f.v1);
        let v2 = Vector3::from(f.v2);
        let v3 = Vector3::from(f.v3);
        avg += v1 + v2 + v3;
        min = min.min(v1).min(v2).min(v3);
        max = max.max(v1).max(v2).max(v3);
    }

    avg /= (solid.facets.len() * 3) as f32;

    Ok(LoadedStl {
        solid,
        avg,
        min,
        max,
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let paths = std::env::args()
        .skip(1)
        .map(PathBuf::from)
        .collect::<Vec<_>>();

    let mut current_path = 0;
    macro_rules! title {
        () => {
            format!(
                "{} [{}/{}]",
                paths[current_path].file_name().unwrap().display(),
                current_path + 1,
                paths.len()
            )
        };
    }

    let mut window = Window::builder()
        .title(&title!())
        .size(800, 600)
        .target_fps(60)
        .flags(ConfigFlags::MSAA_4X_HINT)
        .init();

    let mut loaded = load_stl(&paths[current_path])?;

    let scale = 10.;

    let mut show_controls = true;
    let mut wireframe = false;
    let mut colour = 0;
    let mut show_grid = true;
    let mut show_axes = true;

    let font_size = 32;
    let font = Font::load_from_memory(
        FontFileType::from_path("../assets/Iosevka.ttf").unwrap(),
        font_size,
        include_bytes!("../assets/Iosevka.ttf"),
    )
    .unwrap();

    let mut text_max = Vector2::ZERO;
    let mut max_debug = 0f32;

    let default_camera = Camera3D::builder()
        .position((10., 10., 10.))
        .up(Vector3::UNIT_Z)
        // .target(avg / scale)
        .build();

    let mut camera = default_camera;
    while let Some(mut frame) = window.next_frame() {
        if frame.keyboard().is_key_pressed(Key::One) {
            wireframe ^= true;
        }
        if frame.keyboard().is_key_pressed(Key::Two) {
            show_grid ^= true;
        }
        if frame.keyboard().is_key_pressed(Key::Three) {
            show_axes ^= true;
        }
        if frame.keyboard().is_key_pressed(Key::Four) {
            colour += 1;
            colour %= COLOURS.len();
        }
        if frame.keyboard().is_key_down(Key::Home) {
            frame.mouse().enable_cursor();
            camera = default_camera;
        }
        if frame.keyboard().is_key_pressed(Key::Slash) {
            show_controls ^= true;
        }
        if frame.keyboard().is_key_pressed(Key::Right) {
            current_path = (current_path + 1) % paths.len();
            loaded = load_stl(&paths[current_path])?;
            frame.window_mut().set_title(title!());
        }
        if frame.keyboard().is_key_pressed(Key::Left) {
            current_path = current_path.checked_sub(1).unwrap_or(paths.len() - 1);
            loaded = load_stl(&paths[current_path])?;
            frame.window_mut().set_title(title!());
        }

        let camera_movement = if frame.mouse().is_button_down(MouseButton::Left) {
            let delta = frame.mouse().delta() * 0.1;
            Vector3::new(delta.y, -delta.x, 0.)
        } else {
            Vector3::ZERO
        };

        if frame.mouse().is_button_down(MouseButton::Right) {
            let t_to_p = camera.target.to(camera.position);
            let delta = frame.mouse().delta() * 0.5;
            camera.position = camera.target
                + t_to_p.rotate_by_axis_angle(Vector3::UNIT_Z, -Angle::degrees(delta.x));
            let t_to_p = camera.target.to(camera.position);
            camera.position = camera.target
                + t_to_p
                    .rotate_by_axis_angle(Vector3::UNIT_Z.cross(t_to_p), -Angle::degrees(delta.y));
        }

        camera.update(CameraMode::Custom {
            movement: camera_movement,
            rotation: Vector3::ZERO,
            zoom: -frame.mouse().wheel_move(),
        });

        let mut canvas = frame.begin_drawing();

        canvas.clear_background(Color::from_int(0x181818ff));
        canvas.with_camera_3d(camera, |cam| {
            if show_axes {
                draw_axis(cam, Vector3::UNIT_X, Color::RED.brightness(-0.25));
                draw_axis(cam, Vector3::UNIT_Y, Color::GREEN.brightness(-0.25));
                draw_axis(cam, Vector3::UNIT_Z, Color::BLUE.brightness(-0.25));
            }
            if show_grid {
                draw_xy_grid(cam, 200, 1.);
            }
            rlgl::with_matrix(|_| {
                let mode = if wireframe {
                    DrawingMode::Lines
                } else {
                    DrawingMode::Triangles
                };
                rlgl::drawing_mode(mode, |ctx| {
                    let signum2 = |v: f32| if v == 0. { 0. } else { v.signum() };
                    let normalise = (loaded.min.apply(signum2) + loaded.max.apply(signum2))
                        .apply(signum2)
                        .mul_components(loaded.min.apply(f32::abs));
                    for f in &loaded.solid.facets {
                        let v1 = (Vector3::from(f.v1) - normalise) / scale;
                        let v2 = (Vector3::from(f.v2) - normalise) / scale;
                        let v3 = (Vector3::from(f.v3) - normalise) / scale;

                        let (color, _) = COLOURS[colour];

                        if wireframe {
                            ctx.color(color);

                            ctx.vertex(v1).vertex(v2);
                            ctx.vertex(v2).vertex(v3);
                            ctx.vertex(v3).vertex(v1);
                        } else {
                            let normal = if f.normal == (0., 0., 0.) {
                                v1.to(v2).cross(v1.to(v3))
                            } else {
                                Vector3::from(f.normal) / scale
                            };

                            let angle = normal
                                .angle(
                                    Vector3::UNIT_Z
                                        + Vector3::UNIT_Y * 0.75
                                        + Vector3::UNIT_X * -0.5,
                                )
                                .to_degrees();
                            ctx.color(color.brightness(angle / 180. * -0.75));

                            ctx.normal(normal).vertex(v1);
                            ctx.normal(normal).vertex(v2);
                            ctx.normal(normal).vertex(v3);
                        }
                    }
                });
            })
        });

        let font_size = font_size as f32;
        let spacing = 0.1;
        if show_controls {
            let toggle = |b: bool| if b { "on" } else { "off" };

            let lines = [
                "Controls:",
                "Toggle Controls [/]",
                &format!("Wireframe       [1]: {}", toggle(wireframe)),
                &format!("Show Grid       [2]: {}", toggle(show_grid)),
                &format!("Show Axes       [3]: {}", toggle(show_axes)),
                &format!("Object Colour   [4]: {}", COLOURS[colour].1),
                "Change Object   [Left/Right]",
                "Reset Camera    [Home]",
            ];

            let mut y = 0.;

            text_max = text_max.max(
                lines
                    .iter()
                    .map(|text| font.measure_text(text, font_size, spacing))
                    .fold(Vector2::ZERO, |acc, i| acc.max(i)),
            );

            for text in lines {
                let size = font.measure_text(text, font_size, spacing);

                canvas.draw_rectangle(
                    Rectangle::new(0., y, text_max.x + 10., size.y + 10.),
                    Color::BLACK.alpha(0.5),
                );

                canvas.draw_text_pro(
                    &font,
                    text,
                    (5., 5. + y),
                    Vector2::ZERO,
                    Angle::ZERO,
                    font_size,
                    spacing,
                    Color::WHITE,
                );

                y += size.y + 10.;
            }
        }

        if true {
            let lines = &[format!("Triangles: {}", loaded.solid.facets.len())];

            let mut y = 0.;
            for l in lines {
                let sz = font.measure_text(l, font_size, spacing);
                max_debug = max_debug.max(sz.x);
                y -= sz.y;
            }

            let line_spacing = 10.;

            canvas.draw_rectangle(
                Rectangle::new(
                    0.,
                    canvas.height() as f32 + y,
                    max_debug + 10.,
                    -y + lines.len() as f32 * line_spacing,
                ),
                Color::BLACK.alpha(0.5),
            );

            for text in lines {
                let size = font.measure_text(text, font_size, spacing);

                canvas.draw_text_pro(
                    &font,
                    text,
                    (5., canvas.height() as f32 + y),
                    Vector2::ZERO,
                    Angle::ZERO,
                    font_size,
                    spacing,
                    Color::WHITE,
                );

                y += size.y + line_spacing;
            }
        }
    }

    Ok(())
}
