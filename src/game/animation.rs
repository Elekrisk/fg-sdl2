use std::{collections::HashMap, path::Path};

use sdl2::{
    pixels::PixelFormatEnum,
    render::{Texture, TextureCreator},
    video::WindowContext,
};

use crate::fixed_point::Vec2;

pub struct Animation {
    pub texture_atlas: Texture<'static>,
    pub cell_width: usize,
    pub cell_height: usize,
    pub columns: usize,
    pub frame_data: Vec<FrameData>,
    pub hitboxes: HashMap<usize, HitboxInfo>,

    pub startup: usize,
    /// also includes gaps in active frames
    pub active_frames: usize,
    pub recovery: usize,
}

impl Animation {
    pub fn load(
        path: impl AsRef<Path>,
        texture_creator: &'static TextureCreator<WindowContext>,
    ) -> Result<&'static Self, ()> {
        let anim: interface::Animation =
            serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();

        let img_data = image::load_from_memory(&anim.spritesheet)
            .unwrap()
            .into_rgba8();
        let mut texture = texture_creator
            .create_texture(
                PixelFormatEnum::ABGR8888,
                sdl2::render::TextureAccess::Static,
                img_data.width(),
                img_data.height(),
            )
            .unwrap();
        println!("{:?}", texture.blend_mode());
        texture.set_blend_mode(sdl2::render::BlendMode::Blend);
        texture
            .update(None, &img_data, img_data.width() as usize * 4)
            .unwrap();

        let frame_data: Vec<FrameData> = anim
            .info
            .frame_data
            .into_iter()
            .map(|fd| FrameData {
                delay: fd.delay,
                origin: Vec2::new(
                    fd.origin[0].try_into().unwrap(),
                    fd.origin[1].try_into().unwrap(),
                ),
                root_motion: Vec2::new(
                    fd.root_motion[0].try_into().unwrap(),
                    fd.root_motion[1].try_into().unwrap(),
                ),
                hitboxes: fd
                    .hitboxes
                    .into_iter()
                    .filter_map(|(id, hp)| {
                        if hp.enabled {
                            Some((
                                id,
                                HitboxPosition {
                                    id,
                                    pos: Vec2::new(
                                        hp.pos[0].try_into().unwrap(),
                                        (-hp.pos[1]).try_into().unwrap(),
                                    ),
                                    size: Vec2::new(
                                        hp.size[0].try_into().unwrap(),
                                        hp.size[1].try_into().unwrap(),
                                    ),
                                    enabled: hp.enabled,
                                },
                            ))
                        } else {
                            None
                        }
                    })
                    .collect(),
            })
            .collect();

        let hitboxes: HashMap<usize, HitboxInfo> = anim
            .info
            .hitboxes
            .into_iter()
            .map(|(id, hi)| {
                (
                    id,
                    HitboxInfo {
                        id,
                        tag: hi.desc,
                        is_hurtbox: hi.is_hurtbox,
                    },
                )
            })
            .collect();

        let mut state = 0;

        let mut startup = 0;
        let mut active_frames = 0;
        let mut recovery = 0;

        for frame in &frame_data {
            match state {
                0 => for (id, _) in &frame.hitboxes {
                    if !hitboxes[id].is_hurtbox {
                        state = 1;
                        break;
                    }
                },
                1 => {
                    let mut no_hitbox = true;
                    for (id, _) in &frame.hitboxes {
                        if !hitboxes[id].is_hurtbox {
                            no_hitbox = false;
                            break;
                        }
                    }

                    if no_hitbox {
                        state = 2;
                    }
                },
                2 => {
                    for (id, _) in &frame.hitboxes {
                        if !hitboxes[id].is_hurtbox {
                            active_frames += recovery;
                            recovery = 0;
                            state = 1;
                            break;
                        }
                    }
                }
                _ => unreachable!()
            }

            match state {
                0 => startup += frame.delay,
                1 => active_frames += frame.delay,
                2 => recovery += frame.delay,
                _ => unreachable!()
            }
        }

        println!("startup: {startup}");
        println!("active frames: {active_frames}");
        println!("recovery: {recovery}");

        Ok(Box::leak(Box::new(Animation {
            texture_atlas: texture,
            cell_width: anim.info.cell_width,
            cell_height: anim.info.cell_height,
            columns: anim.info.columns,
            frame_data,
            hitboxes,
            startup,
            active_frames,
            recovery,
        })))
    }
}

pub struct FrameData {
    pub delay: usize,
    pub origin: Vec2,
    pub root_motion: Vec2,
    pub hitboxes: HashMap<usize, HitboxPosition>,
}

pub struct HitboxInfo {
    pub id: usize,
    pub tag: String,
    pub is_hurtbox: bool,
}

pub struct HitboxPosition {
    pub id: usize,
    pub pos: Vec2,
    pub size: Vec2,
    pub enabled: bool,
}

mod interface {
    use std::collections::HashMap;

    use serde::{Deserialize, Serialize};

    mod reeee {
        use base64::Engine;
        use serde::{de::Visitor, Deserializer, Serializer};

        pub fn serialize<S: Serializer>(data: &[u8], s: S) -> Result<S::Ok, S::Error> {
            if s.is_human_readable() {
                let str = base64::engine::general_purpose::STANDARD_NO_PAD.encode(data);
                s.serialize_str(&str)
            } else {
                s.serialize_bytes(data)
            }
        }

        pub fn deserialize<'de, D>(d: D) -> Result<Vec<u8>, D::Error>
        where
            D: Deserializer<'de>,
        {
            struct V;
            impl<'de> Visitor<'de> for V {
                type Value = Vec<u8>;

                fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                    formatter.write_str("data")
                }

                fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    base64::engine::general_purpose::STANDARD_NO_PAD
                        .decode(v)
                        .map_err(|_| panic!())
                }

                fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    Ok(v.into())
                }

                fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    Ok(v)
                }
            }

            if d.is_human_readable() {
                d.deserialize_str(V)
            } else {
                d.deserialize_byte_buf(V)
            }
        }
    }

    #[derive(Serialize, Deserialize)]
    pub struct Animation {
        #[serde(with = "reeee")]
        pub spritesheet: Vec<u8>,
        pub info: SpritesheetInfo,
    }

    #[derive(Serialize, Deserialize)]
    pub struct SpritesheetInfo {
        pub cell_width: usize,
        pub cell_height: usize,
        pub columns: usize,
        pub frame_count: usize,
        pub frame_data: Vec<FrameData>,
        pub hitboxes: HashMap<usize, HitboxInfo>,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct FrameData {
        pub delay: usize,
        pub origin: [f32; 2],
        pub root_motion: [f32; 2],
        pub hitboxes: HashMap<usize, HitboxPosition>,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct HitboxPosition {
        pub id: usize,
        pub pos: [f32; 2],
        pub size: [f32; 2],
        pub enabled: bool,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct HitboxInfo {
        pub id: usize,
        pub desc: String,
        pub is_hurtbox: bool,
    }
}
