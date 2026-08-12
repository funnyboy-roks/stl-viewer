use rl::{
    embed_font,
    prelude::*,
    rlgl::{self, DrawingMode},
    shader::ShaderLocation,
};

use crate::stl::parse_stl;

mod stl;

struct Light {
    position: Vector3,
    target: Vector3,
    color: Color,

    // Shader locations
    position_loc: ShaderLocation<Vector3>,
    target_loc: ShaderLocation<Vector3>,
    color_loc: ShaderLocation<Vector4>,
}

impl Light {
    fn new(position: Vector3, target: Vector3, color: Color, shader: &Shader) -> Self {
        let pos_loc = shader.get_location("sun.position").unwrap();
        let target_loc = shader.get_location("sun.target").unwrap();
        let color_loc = shader.get_location("sun.color").unwrap();

        Self {
            position,
            target,
            color,
            position_loc: pos_loc,
            target_loc,
            color_loc,
        }
    }

    fn update(&mut self) {
        self.position_loc.set(self.position);
        self.target_loc.set(self.target);
        let [r, g, b, a] = self.color.to_normalized();
        self.color_loc.set(Vector4::new(r, g, b, a));
    }

    fn shader() -> Shader {
        shader! {
            vertex {
                { #version 330 }
                {
                    // Input vertex attributes
                    in vec3 vertexPosition;
                    in vec2 vertexTexCoord;
                    in vec3 vertexNormal;
                    in vec4 vertexColor;

                    // Input uniform values
                    uniform mat4 mvp;
                    uniform mat4 matModel;
                    uniform mat4 matNormal;

                    // Output vertex attributes (to fragment shader)
                    out vec3 fragPosition;
                    out vec2 fragTexCoord;
                    out vec4 fragColor;
                    out vec3 fragNormal;

                    // NOTE: Add your custom variables here

                    void main()
                    {
                        // Send vertex attributes to fragment shader
                        fragPosition = vec3(matModel*vec4(vertexPosition, 1.0));
                        fragTexCoord = vertexTexCoord;
                        fragColor = vertexColor;
                        fragNormal = normalize(vec3(matNormal*vec4(vertexNormal, 1.0)));

                        // Calculate final vertex position
                        gl_Position = mvp*vec4(vertexPosition, 1.0);
                    }
                }
            }
            fragment {
                { #version 330 }
                {
                    // Input vertex attributes (from vertex shader)
                    in vec3 fragPosition;
                    in vec2 fragTexCoord;
                    in vec4 fragColor;
                    in vec3 fragNormal;

                    // Input uniform values
                    uniform sampler2D texture0;
                    uniform vec4 colDiffuse;

                    // Output fragment color
                    out vec4 finalColor;

                    struct Light {
                        vec3 position;
                        vec3 target;
                        vec4 color;
                    };

                    // Input lighting values
                    uniform Light sun;
                    uniform vec4 ambient;
                    uniform vec3 viewPos;

                    void main()
                    {
                        // Texel color fetching from texture sampler
                        vec4 texelColor = texture(texture0, fragTexCoord);
                        vec3 lightDot = vec3(0.0);
                        vec3 normal = normalize(fragNormal);
                        vec3 viewD = normalize(viewPos - fragPosition);
                        vec3 specular = vec3(0.0);

                        vec4 tint = colDiffuse*fragColor;

                        // NOTE: Implement here your fragment shader code

                        vec3 light = -normalize(sun.target - sun.position);

                        float NdotL = max(dot(normal, light), 0.0);
                        lightDot += sun.color.rgb*NdotL;

                        float specCo = 0.0;
                        if (NdotL > 0.0) specCo = pow(max(0.0, dot(viewD, reflect(-(light), normal))), 16.0); // 16 refers to shine
                        specular += specCo;

                        finalColor = (texelColor*((tint + vec4(specular, 1.0))*vec4(lightDot, 1.0)));
                        finalColor += texelColor*(ambient/10.0)*tint;

                        // Gamma correction
                        finalColor = pow(finalColor, vec4(1.0/2.2));
                    }
                }
            }
        }
        .unwrap()
    }
}

fn main() {
    let content = std::fs::read(std::env::args().nth(1).unwrap()).unwrap();

    let mut window = Window::builder()
        .title("stl")
        .size(800, 600)
        .target_fps(60)
        .flags(ConfigFlags::MSAA_4X_HINT)
        .init();

    let solid = parse_stl(&content).unwrap();

    let shader = Light::shader();
    let mut sun = Light::new(
        Vector3::new(0., 30., 0.),
        Vector3::ZERO,
        Color::WHITE,
        &shader,
    );

    let mut view_loc = shader.get_location::<Vector3>("viewPos").unwrap();

    let mut ambient_loc = shader.get_location::<Vector4>("ambient").unwrap();
    ambient_loc.set(Vector4::new(1.0, 1.0, 1.0, 1.));

    let mut avg = Vector3::new(0., 0., 0.);
    let mut min = Vector3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
    let mut max = Vector3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);

    for f in &solid.facets {
        let v1 = Vector3::from(f.v1);
        let v2 = Vector3::from(f.v2);
        let v3 = Vector3::from(f.v3);
        avg += v1 + v2 + v3;
        min = min.min(v1).min(v2).min(v3);
        max = max.max(v1).max(v2).max(v3);
    }

    avg /= (solid.facets.len() * 3) as f32;

    let scale = 10.;

    let mut show_controls = true;
    let mut wireframe = false;
    let mut with_shader = false;
    let mut camera_mode = CameraMode::Orbital;

    let font = embed_font!("../assets/Iosevka.ttf");

    let mut text_max = Vector2::ZERO;

    let init_position = (10., 10., 10.);
    let mut camera = Camera3D::builder()
        .position(init_position)
        // .target(avg / scale)
        .build();
    while let Some(frame) = window.next_frame() {
        if frame.keyboard().is_key_pressed(Key::One) {
            wireframe ^= true;
        }
        if frame.keyboard().is_key_pressed(Key::Two) {
            with_shader ^= true;
        }
        if frame.keyboard().is_key_pressed(Key::Three) {
            frame.mouse().enable_cursor();
            camera_mode = match camera_mode {
                CameraMode::Free => CameraMode::Orbital,
                CameraMode::Orbital => {
                    frame.mouse().disable_cursor();
                    CameraMode::FirstPerson
                }
                CameraMode::FirstPerson => {
                    frame.mouse().disable_cursor();
                    CameraMode::ThirdPerson
                }
                CameraMode::ThirdPerson => {
                    frame.mouse().disable_cursor();
                    CameraMode::Free
                }
                CameraMode::Custom { .. } => unreachable!(),
            };
        }
        if frame.keyboard().is_key_down(Key::Home) {
            frame.mouse().enable_cursor();
            camera = Camera3D::builder().position(init_position).build();
            camera_mode = CameraMode::Orbital;
        }
        if frame.keyboard().is_key_pressed(Key::Slash) {
            show_controls ^= true;
        }

        camera.update(camera_mode);

        view_loc.set(camera.position);
        sun.update();

        let mut canvas = frame.begin_drawing();

        canvas.clear_background(Color::RAYWHITE);
        canvas.with_camera_3d(camera, |cam| {
            cam.draw_cylinder(
                Vector3::new(0., 0., 0.),
                Vector3::new(100., 0., 0.),
                0.025,
                0.025,
                16,
                const { Color::GREEN.brightness(-0.25) },
            );
            cam.draw_cylinder(
                Vector3::new(0., 0., 0.),
                Vector3::new(0., 100., 0.),
                0.025,
                0.025,
                16,
                const { Color::RED.brightness(-0.25) },
            );
            cam.draw_cylinder(
                Vector3::new(0., 0., 0.),
                Vector3::new(0., 0., 100.),
                0.025,
                0.025,
                16,
                const { Color::BLUE.brightness(-0.25) },
            );
            unsafe { rl::sys::rlSetLineWidth(3.0) };
            cam.draw_grid(200, 1.);
            unsafe { rl::sys::rlSetLineWidth(1.0) };
            let body = || {
                rlgl::with_matrix(|_| {
                    rlgl::drawing_mode(
                        if wireframe {
                            DrawingMode::Lines
                        } else {
                            DrawingMode::Triangles
                        },
                        |ctx| {
                            let signum2 = |v: f32| if v == 0. { 0. } else { v.signum() };
                            dbg!(min, max);
                            let normalise = dbg!(
                                dbg!(dbg!(min.apply(signum2) + max.apply(signum2)).apply(signum2))
                                    .mul_components(min.apply(f32::abs))
                            );
                            for f in &solid.facets {
                                let v1 = (Vector3::from(f.v1) - normalise) / scale;
                                let v2 = (Vector3::from(f.v2) - normalise) / scale;
                                let v3 = (Vector3::from(f.v3) - normalise) / scale;

                                let color = Color::BLUE;

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

                                    if with_shader {
                                        ctx.color(color);
                                    } else {
                                        let angle = normal.angle(Vector3::UNIT_Y).to_degrees();
                                        ctx.color(color.brightness(angle / 180. * -0.75));
                                    }

                                    ctx.normal(normal).vertex(v1);
                                    ctx.normal(normal).vertex(v2);
                                    ctx.normal(normal).vertex(v3);
                                }
                            }
                        },
                    );
                })
            };

            if with_shader && !wireframe {
                shader.with(body);
            } else {
                body();
            }
        });

        if show_controls {
            let font_size = 32.;
            let spacing = 0.1;

            fn toggle(b: bool) -> &'static str {
                if b { "on" } else { "off" }
            }

            let lines = [
                "Controls:",
                "Toggle Controls [/]",
                &format!("Wireframe       [1]: {}", toggle(wireframe)),
                &format!("Shader          [2]: {}", toggle(with_shader)),
                &format!("Camera Mode     [3]: {:?}", camera_mode),
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
    }
}
