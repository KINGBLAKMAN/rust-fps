//! First-person hand and forearm models. Each hand is built in a few poses
//! (gripping a pistol grip, under a handguard, holding a grenade, open palm)
//! and the viewmodel shows whichever pose it needs. Gloves are fingerless, so
//! the fingertips show skin.

use bevy::prelude::*;

use crate::kit::{c, Kit};

pub const SKIN: Color = c(0.86, 0.66, 0.52);
pub const GLOVE: Color = c(0.09, 0.09, 0.1);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HandPose {
    /// Wrapped round a vertical grip; the index finger on the trigger or not.
    Grip { trigger: bool },
    /// Palm up under a handguard that runs along Z.
    Under,
    /// Holding a small object (grenade, magazine, orb) in the palm.
    Hold,
    /// Flat palm pushing forward.
    Open,
}

impl HandPose {
    pub const ALL: [HandPose; 5] = [
        HandPose::Grip { trigger: true },
        HandPose::Grip { trigger: false },
        HandPose::Under,
        HandPose::Hold,
        HandPose::Open,
    ];

    /// Where the wrist is in hand space (the forearm starts here).
    pub fn wrist(self, side: f32) -> Vec3 {
        let p = match self {
            HandPose::Grip { .. } => Vec3::new(0.034, -0.078, 0.042),
            HandPose::Under => Vec3::new(0.035, -0.075, 0.08),
            HandPose::Hold => Vec3::new(0.0, -0.07, 0.05),
            HandPose::Open => Vec3::new(0.0, -0.06, 0.02),
        };
        Vec3::new(p.x * side, p.y, p.z)
    }
}

/// A finger as three segments; the last one is bare skin.
fn finger(k: &mut Kit, pts: &[Vec3], r: f32) {
    for (i, w) in pts.windows(2).enumerate() {
        let col = if i + 2 == pts.len() { SKIN } else { GLOVE };
        k.capsule_between(w[0], w[1], r, col);
    }
}

/// Builds one hand. `side` is 1 for the right hand and -1 for the left.
pub fn hand_kit(side: f32, pose: HandPose, trim: Color) -> Kit {
    let s = side;
    let p = |x: f32, y: f32, z: f32| Vec3::new(x * s, y, z);
    let mut k = Kit::new();
    match pose {
        HandPose::Grip { trigger } => {
            // Back of the hand on the outside of the grip, fingers round the front.
            k.blob(p(0.03, -0.002, 0.006), Vec3::new(0.017, 0.046, 0.04), GLOVE);
            k.cuboid(p(0.045, 0.0, 0.0), Vec3::new(0.004, 0.055, 0.038), trim);
            let ys = [0.03, 0.01, -0.01, -0.029];
            for (i, y) in ys.iter().enumerate() {
                let r = if i == 3 { 0.0075 } else { 0.0088 };
                if i == 0 && trigger {
                    finger(
                        &mut k,
                        &[
                            p(0.028, 0.03, -0.025),
                            p(0.014, 0.032, -0.046),
                            p(0.003, 0.03, -0.062),
                        ],
                        r,
                    );
                    continue;
                }
                finger(
                    &mut k,
                    &[
                        p(0.03, *y, -0.024),
                        p(0.012, *y, -0.042),
                        p(-0.014, *y, -0.036),
                        p(-0.026, *y - 0.003, -0.014),
                    ],
                    r,
                );
            }
            // Thumb along the far side.
            finger(
                &mut k,
                &[
                    p(0.022, 0.018, 0.03),
                    p(-0.012, 0.03, 0.016),
                    p(-0.024, 0.036, -0.012),
                ],
                0.0095,
            );
            // Cuff.
            k.cyl_between(p(0.03, -0.045, 0.02), p(0.036, -0.09, 0.05), 0.027, GLOVE);
            k.cyl_between(p(0.032, -0.062, 0.03), p(0.034, -0.072, 0.036), 0.029, trim);
        }
        HandPose::Under => {
            // Palm under the handguard, fingers round the far side, thumb near.
            k.blob(
                p(0.006, -0.042, 0.004),
                Vec3::new(0.036, 0.015, 0.046),
                GLOVE,
            );
            for (i, z) in [-0.033f32, -0.011, 0.011, 0.031].iter().enumerate() {
                let r = if i == 3 { 0.0075 } else { 0.0088 };
                finger(
                    &mut k,
                    &[
                        p(-0.012, -0.042, *z),
                        p(-0.032, -0.026, *z),
                        p(-0.037, -0.002, *z),
                        p(-0.03, 0.017, *z),
                    ],
                    r,
                );
            }
            finger(
                &mut k,
                &[
                    p(0.03, -0.034, 0.03),
                    p(0.035, -0.008, 0.0),
                    p(0.03, 0.01, -0.03),
                ],
                0.0095,
            );
            k.cyl_between(p(0.022, -0.05, 0.04), p(0.04, -0.085, 0.09), 0.027, GLOVE);
            k.cyl_between(p(0.03, -0.065, 0.062), p(0.033, -0.071, 0.072), 0.029, trim);
        }
        HandPose::Hold => {
            k.blob(p(0.0, -0.046, 0.012), Vec3::new(0.04, 0.015, 0.042), GLOVE);
            for (i, x) in [-0.03f32, -0.01, 0.01, 0.03].iter().enumerate() {
                let r = if i == 0 { 0.0075 } else { 0.0088 };
                finger(
                    &mut k,
                    &[
                        p(*x, -0.046, -0.022),
                        p(*x, -0.03, -0.044),
                        p(*x, -0.004, -0.047),
                    ],
                    r,
                );
            }
            finger(
                &mut k,
                &[
                    p(0.042, -0.036, 0.016),
                    p(0.047, -0.012, -0.004),
                    p(0.036, 0.008, -0.026),
                ],
                0.0095,
            );
            k.cyl_between(p(0.0, -0.05, 0.035), p(0.0, -0.08, 0.07), 0.027, GLOVE);
            k.cyl_between(p(0.0, -0.06, 0.046), p(0.0, -0.068, 0.056), 0.029, trim);
        }
        HandPose::Open => {
            k.blob(p(0.0, 0.0, 0.0), Vec3::new(0.04, 0.046, 0.015), GLOVE);
            k.cuboid(p(0.0, 0.0, 0.014), Vec3::new(0.05, 0.05, 0.004), trim);
            for (i, x) in [-0.03f32, -0.01, 0.01, 0.03].iter().enumerate() {
                let r = if i == 0 { 0.0075 } else { 0.0088 };
                finger(
                    &mut k,
                    &[
                        p(*x, 0.04, 0.0),
                        p(*x * 1.1, 0.07, -0.006),
                        p(*x * 1.15, 0.095, -0.012),
                    ],
                    r,
                );
            }
            finger(
                &mut k,
                &[
                    p(0.04, -0.01, 0.0),
                    p(0.065, 0.015, -0.008),
                    p(0.078, 0.035, -0.014),
                ],
                0.0095,
            );
            k.cyl_between(p(0.0, -0.035, 0.0), p(0.0, -0.08, 0.015), 0.027, GLOVE);
        }
    }
    k
}

/// A forearm one metre long running up +Y from the wrist, scaled to length
/// in the viewmodel. Sleeve in the suit colour with a trim band.
pub fn forearm_kit(suit: Color, trim: Color) -> Kit {
    let mut k = Kit::new();
    k.frustum(
        Vec3::new(0.0, 0.5, 0.0),
        0.042,
        0.03,
        1.0,
        Quat::IDENTITY,
        suit,
    );
    k.frustum(
        Vec3::new(0.0, 0.04, 0.0),
        0.032,
        0.031,
        0.08,
        Quat::IDENTITY,
        GLOVE,
    );
    k.frustum(
        Vec3::new(0.0, 0.11, 0.0),
        0.034,
        0.033,
        0.035,
        Quat::IDENTITY,
        trim,
    );
    // The forearm muscle swells below the elbow, then slims to the wrist.
    k.blob(Vec3::new(0.0, 0.66, 0.004), Vec3::new(0.044, 0.22, 0.043), suit);
    // Elbow pad seam.
    k.frustum(
        Vec3::new(0.0, 0.75, 0.0),
        0.043,
        0.039,
        0.05,
        Quat::IDENTITY,
        suit.darker(0.12),
    );
    k
}
