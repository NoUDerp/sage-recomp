//! Deterministic SAGE simulation core. No Bevy, no rendering, no audio.

pub mod aiw47_1;
pub mod audioevent;
pub mod audiohandle;
pub mod behaviorw47_1;
pub mod class5b1;
pub mod classname;
pub mod contain1;
pub mod crtmath;
pub mod crtbuf;
pub mod drawdata;
pub mod dxbuf;
pub mod dxstate;
pub mod flagname;
pub mod fx;
pub mod fxflags;
pub mod fxstore;
pub mod fxstore2;
pub mod guicontrol;
pub mod ini;
pub mod math;
pub mod math554;
pub mod md5;
pub mod netblob;
pub mod objectw47_1;
pub mod optslot;
pub mod pathfinder1;
pub mod pooledstring;
pub mod recomp_tasks;
pub mod region;
pub mod region2d;
pub mod rng;
pub mod run7f4;
pub mod run9837;
pub mod scriptstore;
pub mod scriptw47_1;
pub mod snapshot;
pub mod stl;
pub mod string;
pub mod systems1;
pub mod vecpair;
pub mod xml;

#[test]
fn workspace_alive() {
    assert_eq!(2 + 2, 4);
}
