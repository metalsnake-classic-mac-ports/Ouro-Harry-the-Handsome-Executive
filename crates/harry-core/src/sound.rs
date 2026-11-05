//! The sounds a frame asks for, as the frontend plays them: the original
//! plays `snd` resources with `STPlayRegSound(snd, priority, volume)`,
//! `STPlay3DSound(snd, priority, volume, at)` and `STPlay3DSoundPitched`
//! (also a rate, 0x10000 = as recorded); a 3-D sound's volume per ear
//! comes from `CalcStereoVolume`.
//!
//! The original asks its mixer whether a sound plays (`FUN_10056de8`: the
//! voices whose handle or `snd` is the one asked, a handle 0 counting
//! every voice) and decides by it - whether a voice is said, a bump or a
//! machine-gun sound played, even whether a random number is drawn. The
//! frontend reports what still plays before each frame ([`Playing`]); the
//! game adds what it starts and stops within the frame ([`Audio`]).

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
    /// Play `snd` `id` with these volumes (0x100 = full) and rate; the
    /// game's `handle` for it (never 0).
    Play { id: i16, left: u16, right: u16, rate: u32, channel: Channel, handle: u32 },
    /// `FUN_10056c0c(handle)`: stop the sound with this handle; 0 stops
    /// every sound.
    StopHandle(u32),
    /// `FUN_10056f94`: new volumes for the sound with this handle (a
    /// dynamic sound moving).
    Pan { handle: u32, left: u16, right: u16 },
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

/// A sound that plays: its handle, `snd` and channel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Voice {
    pub handle: u32,
    pub id: i16,
    pub channel: Channel,
}

/// The sounds that play.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Playing(pub Vec<Voice>);

impl Playing {
    /// `FUN_10056de8(handle)`: how many play with this handle; 0 counts
    /// every sound.
    pub fn count(&self, handle: u32) -> usize {
        self.0.iter().filter(|v| handle == 0 || v.handle == handle).count()
    }

    /// `FUN_10056de8(snd)`: how many play this `snd`.
    pub fn count_snd(&self, id: i16) -> usize {
        self.0.iter().filter(|v| v.id == id).count()
    }

    /// What an event does to the sounds that play (as the frontend does
    /// it): a sound of a channel other than `Effect` replaces the one
    /// before.
    pub fn apply(&mut self, e: &SoundEvent) {
        match *e {
            SoundEvent::Play { id, channel, handle, .. } => {
                if channel != Channel::Effect {
                    self.0.retain(|v| v.channel != channel);
                }
                self.0.push(Voice { handle, id, channel });
            }
            SoundEvent::HaltVoices => self.0.retain(|v| !matches!(v.channel, Channel::Voice | Channel::Chatter)),
            SoundEvent::StopCharge => self.0.retain(|v| v.channel != Channel::Charge),
            SoundEvent::Stop(id) => self.0.retain(|v| v.id != id),
            SoundEvent::StopHandle(h) => self.0.retain(|v| h != 0 && v.handle != h),
            SoundEvent::Pan { .. } | SoundEvent::FadeMusicOut(_) => {}
        }
    }
}

/// A slot of `STPlayDynamicSound`'s table (`_DAT_10065d34`, 16 of 0x18
/// bytes): a sound whose stereo position moves.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DynamicSound {
    pub used: bool,
    pub handle: u32,
    pub volume: i16,
    /// Where it is (v, h) and how far it moves each pass (v, h).
    pub at: (i16, i16),
    pub speed: (i16, i16),
}

/// The game's side of the sound: what plays, the handles the routines ask
/// about, the dynamic sounds.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Audio {
    pub playing: Playing,
    /// `gVoiceChannel` (the last voice) and `_DAT_10068aa4` (the last
    /// chatter): 0 until the first one - and 0 asks about every sound.
    pub voice: u32,
    pub chatter: u32,
    /// The last handle given (the mixer's counter `_DAT_10066f00`, + 2).
    pub last: u32,
    pub dynamic: [DynamicSound; 16],
    /// Nothing counts as playing, not even what the frame starts (the
    /// whole-frame recorder stubs the mixer so).
    pub silent: bool,
}

impl Audio {
    /// Is a voice or the chatter playing (`FUN_10056de8(gVoiceChannel)`
    /// or `FUN_10056de8(_DAT_10068aa4)`)?
    pub fn voice_playing(&self) -> bool {
        self.playing.count(self.voice) > 0 || self.playing.count(self.chatter) > 0
    }

    /// The next handle.
    pub fn handle(&mut self) -> u32 {
        self.last = self.last.wrapping_add(2);
        if self.last == 0 {
            self.last = 2;
        }
        self.last
    }

    /// `STPlayDynamicSound`: the first free slot of the 16 takes the sound
    /// (none free: it is not played - returns false).
    pub fn add_dynamic(&mut self, handle: u32, volume: i16, at: (i16, i16), speed: (i16, i16)) -> bool {
        match self.dynamic.iter_mut().find(|d| !d.used) {
            Some(d) => {
                *d = DynamicSound { used: true, handle, volume, at, speed };
                true
            }
            None => false,
        }
    }

    /// `UpdateDynamicSounds`, slots 0–14 (the 16th is never updated nor
    /// freed): a slot whose sound has ended is freed; the others move by
    /// their speed and get new volumes for the view.
    pub fn update_dynamic(&mut self, view: (i16, i16)) -> Vec<SoundEvent> {
        let mut out = Vec::new();
        for d in self.dynamic.iter_mut().take(15).filter(|d| d.used) {
            if self.playing.count(d.handle) == 0 {
                d.used = false;
                continue;
            }
            d.at = (d.at.0.wrapping_add(d.speed.0), d.at.1.wrapping_add(d.speed.1));
            let (left, right) = stereo(view, d.at, d.volume);
            out.push(SoundEvent::Pan { handle: d.handle, left, right });
        }
        out
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn play(id: i16, channel: Channel, handle: u32) -> SoundEvent {
        SoundEvent::Play { id, left: 0x100, right: 0x100, rate: RATE, channel, handle }
    }

    #[test]
    fn a_handle_of_0_asks_about_every_sound() {
        let mut p = Playing::default();
        assert_eq!(p.count(0), 0);
        p.apply(&play(800, Channel::Effect, 2));
        p.apply(&play(800, Channel::Effect, 4));
        assert_eq!((p.count(0), p.count(2), p.count(6), p.count_snd(800), p.count_snd(801)), (2, 1, 0, 2, 0));
        // no voice yet: "is a voice playing" asks about everything
        let a = Audio { playing: p.clone(), ..Audio::default() };
        assert!(a.voice_playing());
        let a = Audio { playing: p, voice: 8, chatter: 10, ..Audio::default() };
        assert!(!a.voice_playing());
    }

    #[test]
    fn channels_replace_and_stops_remove() {
        let mut p = Playing::default();
        p.apply(&play(210, Channel::Voice, 2));
        p.apply(&play(211, Channel::Voice, 4));
        p.apply(&play(150, Channel::Chatter, 6));
        p.apply(&play(800, Channel::Effect, 8));
        assert_eq!(p.0.iter().map(|v| v.handle).collect::<Vec<_>>(), [4, 6, 8]);
        p.apply(&SoundEvent::HaltVoices);
        assert_eq!(p.0.iter().map(|v| v.handle).collect::<Vec<_>>(), [8]);
        p.apply(&play(801, Channel::Effect, 10));
        p.apply(&SoundEvent::StopHandle(8));
        assert_eq!(p.count(0), 1);
        p.apply(&SoundEvent::StopHandle(0));
        assert_eq!(p.count(0), 0);
    }

    #[test]
    fn dynamic_sounds_move_until_they_end() {
        let mut a = Audio::default();
        let h = a.handle();
        a.playing.apply(&play(400, Channel::Effect, h));
        assert!(a.add_dynamic(h, 0x100, (1000, 1000), (0, 15)));
        let view = (1000 - 224, 1000 - 224);
        let e = a.update_dynamic(view);
        assert_eq!(a.dynamic[0].at, (1000, 1015));
        let (left, right) = stereo(view, (1000, 1015), 0x100);
        assert_eq!(e, [SoundEvent::Pan { handle: h, left, right }]);
        // ended: the slot is freed, nothing more
        a.playing = Playing::default();
        assert!(a.update_dynamic(view).is_empty());
        assert!(!a.dynamic[0].used);
    }

    #[test]
    fn the_16th_dynamic_slot_is_never_updated() {
        let mut a = Audio::default();
        for k in 0..16 {
            let h = a.handle();
            a.playing.apply(&play(400, Channel::Effect, h));
            assert!(a.add_dynamic(h, 0x100, (0, 0), (1, 1)), "slot {k}");
        }
        assert!(!a.add_dynamic(99, 0x100, (0, 0), (1, 1)));
        assert_eq!(a.update_dynamic((0, 0)).len(), 15);
        assert_eq!((a.dynamic[14].at, a.dynamic[15].at), ((1, 1), (0, 0)));
        a.playing = Playing::default();
        a.update_dynamic((0, 0));
        assert!(a.dynamic[..15].iter().all(|d| !d.used) && a.dynamic[15].used);
    }
}
