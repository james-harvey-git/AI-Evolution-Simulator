use macroquad::prelude::*;

use crate::visual::VisualSettings;

/// Simple bloom post-processing using render targets.
/// Renders the scene to an offscreen target, extracts bright pixels,
/// blurs them at half resolution, and composites additively.
pub struct BloomPipeline {
    scene_target: RenderTarget,
    bright_target: RenderTarget,
    blur_h_target: RenderTarget,
    blur_v_target: RenderTarget,
    bright_material: Material,
    blur_h_material: Material,
    blur_v_material: Material,
    combine_material: Material,
    width: u32,
    height: u32,
}

const BRIGHT_EXTRACT_VERT: &str = r#"#version 100
attribute vec3 position;
attribute vec2 texcoord;
varying lowp vec2 uv;
uniform mat4 Model;
uniform mat4 Projection;
void main() {
    gl_Position = Projection * Model * vec4(position, 1);
    uv = texcoord;
}
"#;

const BRIGHT_EXTRACT_FRAG: &str = r#"#version 100
precision lowp float;
varying lowp vec2 uv;
uniform sampler2D Texture;
uniform float threshold;
void main() {
    vec4 color = texture2D(Texture, uv);
    float brightness = dot(color.rgb, vec3(0.2126, 0.7152, 0.0722));
    if (brightness > threshold) {
        gl_FragColor = vec4(color.rgb * (brightness - threshold), color.a);
    } else {
        gl_FragColor = vec4(0.0);
    }
}
"#;

const BLUR_H_FRAG: &str = r#"#version 100
precision lowp float;
varying lowp vec2 uv;
uniform sampler2D Texture;
uniform vec2 texel_size;
void main() {
    vec4 sum = vec4(0.0);
    sum += texture2D(Texture, uv + vec2(-3.0 * texel_size.x, 0.0)) * 0.06;
    sum += texture2D(Texture, uv + vec2(-2.0 * texel_size.x, 0.0)) * 0.12;
    sum += texture2D(Texture, uv + vec2(-1.0 * texel_size.x, 0.0)) * 0.18;
    sum += texture2D(Texture, uv) * 0.28;
    sum += texture2D(Texture, uv + vec2( 1.0 * texel_size.x, 0.0)) * 0.18;
    sum += texture2D(Texture, uv + vec2( 2.0 * texel_size.x, 0.0)) * 0.12;
    sum += texture2D(Texture, uv + vec2( 3.0 * texel_size.x, 0.0)) * 0.06;
    gl_FragColor = sum;
}
"#;

const BLUR_V_FRAG: &str = r#"#version 100
precision lowp float;
varying lowp vec2 uv;
uniform sampler2D Texture;
uniform vec2 texel_size;
void main() {
    vec4 sum = vec4(0.0);
    sum += texture2D(Texture, uv + vec2(0.0, -3.0 * texel_size.y)) * 0.06;
    sum += texture2D(Texture, uv + vec2(0.0, -2.0 * texel_size.y)) * 0.12;
    sum += texture2D(Texture, uv + vec2(0.0, -1.0 * texel_size.y)) * 0.18;
    sum += texture2D(Texture, uv) * 0.28;
    sum += texture2D(Texture, uv + vec2(0.0,  1.0 * texel_size.y)) * 0.18;
    sum += texture2D(Texture, uv + vec2(0.0,  2.0 * texel_size.y)) * 0.12;
    sum += texture2D(Texture, uv + vec2(0.0,  3.0 * texel_size.y)) * 0.06;
    gl_FragColor = sum;
}
"#;

const COMBINE_FRAG: &str = r#"#version 100
precision lowp float;
varying lowp vec2 uv;
uniform sampler2D Texture;
uniform sampler2D bloom_texture;
uniform float bloom_intensity;
uniform float grade_strength;
uniform float vignette_strength;
uniform vec3 grade_tint;
void main() {
    vec4 scene = texture2D(Texture, uv);
    vec4 bloom = texture2D(bloom_texture, uv);
    vec3 graded = mix(scene.rgb, scene.rgb * grade_tint, grade_strength);
    vec2 p = uv * 2.0 - 1.0;
    float vignette = clamp(1.0 - dot(p, p) * vignette_strength, 0.0, 1.0);
    vec3 composed = (graded + bloom.rgb * bloom_intensity) * vignette;
    gl_FragColor = vec4(composed, scene.a);
}
"#;

impl BloomPipeline {
    pub fn new() -> Option<Self> {
        let width = screen_width() as u32;
        let height = screen_height() as u32;
        let half_w = width / 2;
        let half_h = height / 2;

        let scene_target = render_target(width, height);
        scene_target.texture.set_filter(FilterMode::Linear);

        let bright_target = render_target(half_w, half_h);
        bright_target.texture.set_filter(FilterMode::Linear);

        let blur_h_target = render_target(half_w, half_h);
        blur_h_target.texture.set_filter(FilterMode::Linear);

        let blur_v_target = render_target(half_w, half_h);
        blur_v_target.texture.set_filter(FilterMode::Linear);

        let bright_material = load_material(
            ShaderSource::Glsl {
                vertex: BRIGHT_EXTRACT_VERT,
                fragment: BRIGHT_EXTRACT_FRAG,
            },
            MaterialParams {
                uniforms: vec![UniformDesc::new("threshold", UniformType::Float1)],
                ..Default::default()
            },
        )
        .ok()?;

        let blur_h_material = load_material(
            ShaderSource::Glsl {
                vertex: BRIGHT_EXTRACT_VERT,
                fragment: BLUR_H_FRAG,
            },
            MaterialParams {
                uniforms: vec![UniformDesc::new("texel_size", UniformType::Float2)],
                ..Default::default()
            },
        )
        .ok()?;

        let blur_v_material = load_material(
            ShaderSource::Glsl {
                vertex: BRIGHT_EXTRACT_VERT,
                fragment: BLUR_V_FRAG,
            },
            MaterialParams {
                uniforms: vec![UniformDesc::new("texel_size", UniformType::Float2)],
                ..Default::default()
            },
        )
        .ok()?;

        let combine_material = load_material(
            ShaderSource::Glsl {
                vertex: BRIGHT_EXTRACT_VERT,
                fragment: COMBINE_FRAG,
            },
            MaterialParams {
                uniforms: vec![
                    UniformDesc::new("bloom_intensity", UniformType::Float1),
                    UniformDesc::new("grade_strength", UniformType::Float1),
                    UniformDesc::new("vignette_strength", UniformType::Float1),
                    UniformDesc::new("grade_tint", UniformType::Float3),
                ],
                textures: vec!["bloom_texture".to_string()],
                ..Default::default()
            },
        )
        .ok()?;

        Some(Self {
            scene_target,
            bright_target,
            blur_h_target,
            blur_v_target,
            bright_material,
            blur_h_material,
            blur_v_material,
            combine_material,
            width,
            height,
        })
    }

    /// Get the render target for the world camera to render into.
    pub fn scene_render_target(&self) -> RenderTarget {
        self.scene_target.clone()
    }

    /// Process the rendered scene: extract bright, blur, combine.
    pub fn apply(&self, visual_settings: VisualSettings) {
        let half_w = self.width as f32 / 2.0;
        let half_h = self.height as f32 / 2.0;

        // Step 1: Extract bright pixels to half-res target
        set_camera(&Camera2D {
            render_target: Some(self.bright_target.clone()),
            ..Camera2D::from_display_rect(Rect::new(0.0, 0.0, half_w, half_h))
        });
        clear_background(BLACK);
        self.bright_material
            .set_uniform("threshold", visual_settings.bloom_threshold());
        gl_use_material(&self.bright_material);
        draw_texture_ex(
            &self.scene_target.texture,
            0.0,
            0.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(half_w, half_h)),
                ..Default::default()
            },
        );
        gl_use_default_material();

        // Step 2: Horizontal blur
        set_camera(&Camera2D {
            render_target: Some(self.blur_h_target.clone()),
            ..Camera2D::from_display_rect(Rect::new(0.0, 0.0, half_w, half_h))
        });
        clear_background(BLACK);
        self.blur_h_material
            .set_uniform("texel_size", vec2(1.0 / half_w, 1.0 / half_h));
        gl_use_material(&self.blur_h_material);
        draw_texture_ex(
            &self.bright_target.texture,
            0.0,
            0.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(half_w, half_h)),
                ..Default::default()
            },
        );
        gl_use_default_material();

        // Step 3: Vertical blur
        set_camera(&Camera2D {
            render_target: Some(self.blur_v_target.clone()),
            ..Camera2D::from_display_rect(Rect::new(0.0, 0.0, half_w, half_h))
        });
        clear_background(BLACK);
        self.blur_v_material
            .set_uniform("texel_size", vec2(1.0 / half_w, 1.0 / half_h));
        gl_use_material(&self.blur_v_material);
        draw_texture_ex(
            &self.blur_h_target.texture,
            0.0,
            0.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(half_w, half_h)),
                ..Default::default()
            },
        );
        gl_use_default_material();

        // Step 4: Combine scene + bloom
        set_default_camera();
        self.combine_material
            .set_uniform("bloom_intensity", visual_settings.bloom_intensity());
        self.combine_material
            .set_uniform("grade_strength", visual_settings.grade_strength());
        self.combine_material
            .set_uniform("vignette_strength", visual_settings.vignette_strength());
        self.combine_material
            .set_uniform("grade_tint", visual_settings.grade_tint());
        self.combine_material
            .set_texture("bloom_texture", self.blur_v_target.texture.clone());
        gl_use_material(&self.combine_material);
        draw_texture_ex(
            &self.scene_target.texture,
            0.0,
            0.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(screen_width(), screen_height())),
                ..Default::default()
            },
        );
        gl_use_default_material();
    }

    /// Check if window was resized and rebuild targets if needed.
    pub fn check_resize(&mut self) {
        let w = screen_width() as u32;
        let h = screen_height() as u32;
        if w != self.width || h != self.height {
            self.width = w;
            self.height = h;
            let half_w = w / 2;
            let half_h = h / 2;

            self.scene_target = render_target(w, h);
            self.scene_target.texture.set_filter(FilterMode::Linear);

            self.bright_target = render_target(half_w, half_h);
            self.bright_target.texture.set_filter(FilterMode::Linear);

            self.blur_h_target = render_target(half_w, half_h);
            self.blur_h_target.texture.set_filter(FilterMode::Linear);

            self.blur_v_target = render_target(half_w, half_h);
            self.blur_v_target.texture.set_filter(FilterMode::Linear);
        }
    }
}
