//! The sounds a frame asks for, as the frontend plays them: the original
//! plays `snd` resources with `STPlayRegSound(snd, priority, volume)`,
//! `STPlay3DSound(snd, priority, volume, at)` and `STPlay3DSoundPitched`
//! (also a rate, 0x10000 = as recorded); a 3-D sound's volume per ear
//! comes from `CalcStereoVolume`.

use crate::effect::pe_distance;

/// What the original keeps a sound's channel for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Channel {
    /// Any free channel.
    Effect,
    /// A voice (`_DAT_10068aa0`): `HaltTalkingSounds` stops it.
    Voice,
    /// Harry's idle chatter (`_DAT_10068aa4`), also a voice.
    Chatter,
    /// A remark (`uRam10068a98`): a new one stops the last.
    Remark,
    /// The charging weapon (`_DAT_10068a9c`).
    Charge,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoundEvent {
    /// Play `snd` `id` with these volumes (0x100 = full) and rate.
    Play { id: i16, left: u16, right: u16, rate: u32, channel: Channel },
    /// `HaltTalkingSounds`: stop the voices.
    HaltVoices,
    /// Stop the charge sound.
    StopCharge,
    /// Stop every playing instance of a sound (`FUN_10056c0c`).
    Stop(i16),
    /// `FadeMusic(0, ticks)`: the music fades out in 24 steps of `ticks`
    /// (Harry's and the boss's death).
    FadeMusicOut(u16),
}

/// The normal rate.
pub const RATE: u32 = 0x10000;

/// `CalcStereoVolume`: the volume for each ear - the left at the view's
/// left edge, the right at its right edge, both at the middle's height:
/// (1024 − `PEDistance`) / 8 of 128 for sounds within 1024 px.
pub fn stereo(view: (i16, i16), at: (i16, i16), volume: i16) -> (u16, u16) {
    let ear = |h: i16| {
        let dh = (i32::from(h) - i32::from(at.1)).abs() as i16;
        let dv = (i32::from(view.0.wrapping_add(224)) - i32::from(at.0)).abs() as i16;
        let d = pe_distance(dh, dv);
        if d < 0x401 { (0x400 - d) >> 3 } else { 0 }
    };
    let l = (i32::from(volume) * ear(view.1)) >> 7;
    let r = (i32::from(volume) * ear(view.1.wrapping_add(448))) >> 7;
    (l.clamp(0, 0xffff) as u16, r.clamp(0, 0xffff) as u16)
}

/// `snd` ids of sounds the routines name by what happens.
pub mod snd {
    /// Harry's idle chatter (three samples).
    pub const CHATTER: [i16; 3] = [150, 151, 152];
    /// `Ouch` (`SATRand(100)`: below 43, below 86, else).
    pub const OUCH: [i16; 3] = [220, 225, 226];
    pub const SCOOT_WET: i16 = 215;
    /// Scooting on floor kind 1, 2, 3.
    pub const SCOOT: [i16; 3] = [200, 202, 201];
    pub const KICK_HIT: i16 = 203;
    pub const HALF_PIPE_OUT: i16 = 209;
    pub const LAND: i16 = 207;
    pub const BOUNCE: i16 = 208;
    pub const FALL: i16 = 4901;
    pub const RETURN: i16 = 224;
    /// The level's end (`UpLevel`, `_DAT_10065b54`).
    pub const LEVEL_DONE: i16 = 4800;
    /// The plant's halfpipe, landing and bouncing sound.
    pub const PLANT: i16 = 1101;
    pub const WATER: i16 = 215;
    pub const BALLOON: i16 = 217;
    pub const PHONE_RING: i16 = 301;
}
