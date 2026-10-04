//! Gun mounts: where a character model holds its gun. Set `want` and the
//! gun model appears.

use bevy::prelude::*;

pub struct HumanoidPlugin;

impl Plugin for HumanoidPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, fill_gun_mounts);
    }
}

/// Where a model holds its gun. Set `want` and the gun model appears.
#[derive(Component)]
pub struct GunMount {
    pub want: Option<u8>,
    pub skin: u8,
    pub attach: crate::data::Attach,
    shown: Option<(u8, u8, crate::data::Attach)>,
}

impl GunMount {
    pub fn new(gun: u8, skin: u8) -> Self {
        Self {
            want: Some(gun),
            skin,
            attach: crate::data::Attach::NONE,
            shown: None,
        }
    }
}

fn fill_gun_mounts(
    mut commands: Commands,
    guns: Option<Res<crate::gunmodels::GunAssets>>,
    mut mounts: Query<(Entity, &mut GunMount)>,
) {
    let Some(guns) = guns else { return };
    for (e, mut m) in &mut mounts {
        let want = m.want.map(|g| (g, m.skin, m.attach));
        if m.shown == want {
            continue;
        }
        m.shown = want;
        commands.entity(e).despawn_related::<Children>();
        if let Some((gun, skin, attach)) = want {
            commands.entity(e).with_children(|p| {
                crate::gunmodels::spawn_gun(p, &guns, gun, attach, guns.skin(skin), false)
            });
        }
    }
}
