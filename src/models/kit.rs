//! A small modelling kit: builds one mesh out of many coloured primitive
//! shapes (boxes, cylinders, spheres, cones, rings, wedges). Colours are
//! stored per vertex, so a whole detailed model is a single mesh and draws
//! cheaply. Used for the guns, hands, ability effects, the mystery box and
//! all the map props.

use bevy::asset::RenderAssetUsages;
use bevy::math::primitives::{Extrusion, Triangle2d};
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology, VertexAttributeValues};

/// Collects shapes and merges them into one vertex-coloured mesh.
#[derive(Default, Clone)]
pub struct Kit {
    mesh: Option<Mesh>,
    /// Smoother curves (more segments), for characters seen up close.
    fine: bool,
}

fn to_u32(mesh: &mut Mesh) {
    if let Some(Indices::U16(v)) = mesh.indices() {
        let v: Vec<u32> = v.iter().map(|i| *i as u32).collect();
        mesh.insert_indices(Indices::U32(v));
    }
}

pub fn lin(c: Color) -> [f32; 4] {
    c.to_linear().to_f32_array()
}

impl Kit {
    pub fn new() -> Self {
        Self::default()
    }

    /// A kit that builds round shapes with many more segments.
    pub fn fine() -> Self {
        Self {
            mesh: None,
            fine: true,
        }
    }

    /// Adds any mesh, placed by `tf`, painted `color`.
    pub fn add(&mut self, mut mesh: Mesh, tf: Transform, color: Color) {
        to_u32(&mut mesh);
        // Keep only what every shape has, so they merge cleanly.
        let keep = [
            Mesh::ATTRIBUTE_POSITION.id,
            Mesh::ATTRIBUTE_NORMAL.id,
            Mesh::ATTRIBUTE_UV_0.id,
        ];
        let ids: Vec<_> = mesh.attributes().map(|(a, _)| a.id).collect();
        for id in ids {
            if !keep.contains(&id) {
                mesh.remove_attribute(id);
            }
        }
        let n = mesh.count_vertices();
        if mesh.attribute(Mesh::ATTRIBUTE_UV_0).is_none() {
            mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0f32, 0.0]; n]);
        }
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![lin(color); n]);
        mesh.transform_by(tf);
        match &mut self.mesh {
            None => self.mesh = Some(mesh),
            Some(m) => {
                let _ = m.merge(&mesh);
            }
        }
    }

    /// Adds everything from another kit, placed by `tf`.
    pub fn append(&mut self, other: Kit, tf: Transform) {
        if let Some(mut mesh) = other.mesh {
            mesh.transform_by(tf);
            match &mut self.mesh {
                None => self.mesh = Some(mesh),
                Some(m) => {
                    let _ = m.merge(&mesh);
                }
            }
        }
    }

    pub fn cuboid(&mut self, center: Vec3, size: Vec3, color: Color) {
        self.add(
            Cuboid::from_size(size).into(),
            Transform::from_translation(center),
            color,
        );
    }

    pub fn cuboid_rot(&mut self, center: Vec3, size: Vec3, rot: Quat, color: Color) {
        self.add(
            Cuboid::from_size(size).into(),
            Transform::from_translation(center).with_rotation(rot),
            color,
        );
    }

    /// A box stretched between two points (beams, struts, rails).
    pub fn beam(&mut self, a: Vec3, b: Vec3, thickness: Vec2, color: Color) {
        let d = b - a;
        let len = d.length();
        if len < 1e-4 {
            return;
        }
        let rot = Quat::from_rotation_arc(Vec3::Y, d / len);
        self.cuboid_rot(
            (a + b) / 2.0,
            Vec3::new(thickness.x, len, thickness.y),
            rot,
            color,
        );
    }

    /// Cylinder standing on Y, rotated by `rot`.
    pub fn cyl(&mut self, center: Vec3, radius: f32, height: f32, rot: Quat, color: Color) {
        let res = if self.fine {
            if radius < 0.02 {
                10
            } else {
                24
            }
        } else if radius < 0.05 {
            8
        } else if radius < 0.6 {
            12
        } else {
            20
        };
        self.add(
            Cylinder::new(radius, height).mesh().resolution(res).build(),
            Transform::from_translation(center).with_rotation(rot),
            color,
        );
    }

    /// Cylinder running along Z (barrels).
    pub fn cyl_z(&mut self, center: Vec3, radius: f32, length: f32, color: Color) {
        self.cyl(
            center,
            radius,
            length,
            Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
            color,
        );
    }

    /// Hollow tube running along Z (sight housings you can see through).
    pub fn tube_z(&mut self, center: Vec3, inner: f32, outer: f32, length: f32, color: Color) {
        let res = if self.fine { 32 } else { 16 };
        self.add(
            Extrusion::new(Annulus::new(inner, outer), length)
                .mesh()
                .resolution(res)
                .build(),
            Transform::from_translation(center),
            color,
        );
    }

    /// Cylinder from `a` to `b`.
    pub fn cyl_between(&mut self, a: Vec3, b: Vec3, radius: f32, color: Color) {
        let d = b - a;
        let len = d.length();
        if len < 1e-4 {
            return;
        }
        self.cyl(
            (a + b) / 2.0,
            radius,
            len,
            Quat::from_rotation_arc(Vec3::Y, d / len),
            color,
        );
    }

    pub fn sphere(&mut self, center: Vec3, radius: f32, color: Color) {
        let detail = if self.fine {
            if radius < 0.02 {
                2
            } else {
                3
            }
        } else if radius < 0.3 {
            1
        } else {
            2
        };
        self.add(
            Sphere::new(radius).mesh().ico(detail).unwrap(),
            Transform::from_translation(center),
            color,
        );
    }

    /// Sphere squashed or stretched by `scale`.
    pub fn blob(&mut self, center: Vec3, scale: Vec3, color: Color) {
        let detail = if self.fine { 4 } else { 2 };
        self.add(
            Sphere::new(1.0).mesh().ico(detail).unwrap(),
            Transform::from_translation(center).with_scale(scale),
            color,
        );
    }

    /// Blob turned by `rot`.
    pub fn blob_rot(&mut self, center: Vec3, scale: Vec3, rot: Quat, color: Color) {
        let detail = if self.fine { 4 } else { 2 };
        self.add(
            Sphere::new(1.0).mesh().ico(detail).unwrap(),
            Transform::from_translation(center)
                .with_rotation(rot)
                .with_scale(scale),
            color,
        );
    }

    pub fn cone(&mut self, center: Vec3, radius: f32, height: f32, rot: Quat, color: Color) {
        self.add(
            Cone::new(radius, height)
                .mesh()
                .resolution(if self.fine { 24 } else { 12 })
                .build(),
            Transform::from_translation(center).with_rotation(rot),
            color,
        );
    }

    pub fn frustum(
        &mut self,
        center: Vec3,
        top: f32,
        bottom: f32,
        height: f32,
        rot: Quat,
        color: Color,
    ) {
        self.add(
            ConicalFrustum {
                radius_top: top,
                radius_bottom: bottom,
                height,
            }
            .mesh()
            .resolution(if self.fine { 28 } else { 14 })
            .build(),
            Transform::from_translation(center).with_rotation(rot),
            color,
        );
    }

    /// Ring lying flat (rotate to stand it up).
    pub fn torus(&mut self, center: Vec3, minor: f32, major: f32, rot: Quat, color: Color) {
        self.add(
            Torus::new(major - minor, major + minor)
                .mesh()
                .minor_resolution(if self.fine { 12 } else { 8 })
                .major_resolution(if self.fine { 32 } else { 18 })
                .build(),
            Transform::from_translation(center).with_rotation(rot),
            color,
        );
    }

    pub fn capsule_between(&mut self, a: Vec3, b: Vec3, radius: f32, color: Color) {
        let d = b - a;
        let len = d.length();
        if len < 1e-4 {
            self.sphere(a, radius, color);
            return;
        }
        self.add(
            Capsule3d::new(radius, len)
                .mesh()
                .longitudes(if self.fine { 24 } else { 10 })
                .latitudes(if self.fine { 12 } else { 6 })
                .build(),
            Transform::from_translation((a + b) / 2.0)
                .with_rotation(Quat::from_rotation_arc(Vec3::Y, d / len)),
            color,
        );
    }

    /// Triangular prism (a roof): `size.x` wide at the base, `size.y` tall,
    /// `size.z` deep. The base sits at `center.y - size.y / 2`.
    pub fn wedge(&mut self, center: Vec3, size: Vec3, rot: Quat, color: Color) {
        let tri = Triangle2d::new(
            Vec2::new(-size.x / 2.0, -size.y / 2.0),
            Vec2::new(size.x / 2.0, -size.y / 2.0),
            Vec2::new(0.0, size.y / 2.0),
        );
        self.add(
            Extrusion::new(tri, size.z).mesh().build(),
            Transform::from_translation(center).with_rotation(rot),
            color,
        );
    }

    pub fn build(self) -> Option<Mesh> {
        self.mesh
    }

    /// Builds, or an empty placeholder mesh if nothing was added.
    pub fn build_or_empty(self) -> Mesh {
        self.mesh.unwrap_or_else(|| {
            let mut m = Mesh::new(
                PrimitiveTopology::TriangleList,
                RenderAssetUsages::default(),
            );
            m.insert_attribute(Mesh::ATTRIBUTE_POSITION, vec![[0.0f32; 3]; 3]);
            m.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0f32, 1.0, 0.0]; 3]);
            m.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0f32; 2]; 3]);
            m.insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![[0.0f32; 4]; 3]);
            m.insert_indices(Indices::U32(vec![0, 1, 2]));
            m
        })
    }
}

/// Scales a mesh's UVs (so textures repeat across big surfaces).
pub fn scale_uvs(mesh: &mut Mesh, scale: Vec2) {
    if let Some(VertexAttributeValues::Float32x2(uvs)) = mesh.attribute_mut(Mesh::ATTRIBUTE_UV_0) {
        for uv in uvs.iter_mut() {
            uv[0] *= scale.x;
            uv[1] *= scale.y;
        }
    }
}

/// Material that shows vertex colours as they are.
pub fn vertex_material(roughness: f32, metallic: f32) -> StandardMaterial {
    StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: roughness,
        metallic,
        ..default()
    }
}

/// Material for glowing vertex-coloured parts (lights, screens, signs).
/// Unlit, so each part shows its own vertex colour at full brightness;
/// `strength` brightens it further.
pub fn glow_material(strength: f32) -> StandardMaterial {
    StandardMaterial {
        base_color: Color::linear_rgb(strength, strength, strength),
        unlit: true,
        ..default()
    }
}

/// Clear glass for sight lenses: mostly see-through with a faint tint
/// (from the vertex colour) and a glossy sheen.
pub fn glass_material() -> StandardMaterial {
    StandardMaterial {
        base_color: Color::srgba(1.0, 1.0, 1.0, 0.16),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.04,
        reflectance: 0.9,
        ..default()
    }
}

/// Shorthand for an sRGB colour.
pub const fn c(r: f32, g: f32, b: f32) -> Color {
    Color::srgb(r, g, b)
}
