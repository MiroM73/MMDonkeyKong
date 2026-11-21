//! The pixel-art sprites: small const arrays of text rows (one character = one pixel, see
//! `palette`), decoded once into `NEAREST` textures. No image files; all designs are original.
//! The playfield colours are fixed - the light / dark UI theme does not touch them.

use crate::app::*;

/// One sprite: the rows of the picture; with `mirror` the rows hold only the left half and the
/// right half is its mirror image (symmetric figures are drawn half as many characters).
pub(in crate::app) struct SpriteDef {
    pub rows: &'static [&'static str],
    pub mirror: bool,
}

/// The colour of a sprite character, `None` for an unknown one. `.` is transparent.
pub(in crate::app) fn palette(c: char) -> Option<egui::Color32> {
    let rgb = |r, g, b| Some(egui::Color32::from_rgb(r, g, b));
    match c {
        '.' => Some(egui::Color32::TRANSPARENT),
        'K' => rgb(24, 18, 36),
        'R' => rgb(220, 50, 50),
        'T' => rgb(40, 190, 170),
        'B' => rgb(60, 100, 230),
        'S' => rgb(250, 196, 150),
        'N' => rgb(150, 90, 45),
        'O' => rgb(230, 140, 50),
        'Y' => rgb(250, 220, 70),
        'W' => rgb(250, 250, 250),
        'G' => rgb(160, 165, 180),
        'P' => rgb(130, 70, 180),
        'L' => rgb(190, 140, 230),
        'M' => rgb(240, 110, 170),
        'F' => rgb(255, 80, 30),
        _ => None,
    }
}

/// A decoded sprite: size and pixels (row by row).
pub(in crate::app) struct Decoded {
    pub size: [usize; 2],
    pub pixels: Vec<egui::Color32>,
}

/// Turns the text rows into pixels; an error says what is wrong with the sprite.
pub(in crate::app) fn decode(def: &SpriteDef) -> Result<Decoded, String> {
    let half = def.rows.first().ok_or("no rows")?.chars().count();
    let width = if def.mirror { half * 2 } else { half };
    let mut pixels = Vec::with_capacity(width * def.rows.len());
    for (y, row) in def.rows.iter().enumerate() {
        let chars: Vec<char> = row.chars().collect();
        if chars.len() != half {
            return Err(format!(
                "row {y} has {} characters, not {half}",
                chars.len()
            ));
        }
        let all = chars.iter().copied();
        let full: Vec<char> = if def.mirror {
            all.clone().chain(all.rev()).collect()
        } else {
            all.collect()
        };
        for c in full {
            pixels.push(palette(c).ok_or(format!("row {y}: unknown character {c:?}"))?);
        }
    }
    Ok(Decoded {
        size: [width, def.rows.len()],
        pixels,
    })
}

macro_rules! sprites {
    ($( $id:ident, $mirror:expr, [ $($row:expr),+ $(,)? ]; )+) => {
        /// Every sprite of the game.
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub(in crate::app) enum Id { $($id),+ }

        impl Id {
            pub(in crate::app) const ALL: &'static [Id] = &[$(Id::$id),+];

            pub(in crate::app) fn def(self) -> SpriteDef {
                match self {
                    $(Id::$id => SpriteDef { rows: &[$($row),+], mirror: $mirror }),+
                }
            }
        }
    };
}

// The hero is 10 x 12 (feet in the bottom row, the hit box is the 6 x 12 middle), looks to the
// right; the front end flips it. The ape is 24 x 24, the friend 12 x 16, barrels 8 x 8.
sprites! {
    HeroStand, false, [
        "...TTTT...",
        "..TTTTTT..",
        "..SSSSKS..",
        "..SSSSSS..",
        ".TTTTTTTT.",
        "SSTBBBBTSS",
        ".STBBBBTS.",
        "..BBBBBB..",
        "..BB..BB..",
        "..BB..BB..",
        ".NNN..NNN.",
        ".NNN..NNN.",
    ];
    HeroWalk1, false, [
        "...TTTT...",
        "..TTTTTT..",
        "..SSSSKS..",
        "..SSSSSS..",
        ".TTTTTTTT.",
        "SSTBBBBTSS",
        ".STBBBBTS.",
        "..BBBBBB..",
        ".BB....BB.",
        "BB......BB",
        "NN......NN",
        "NN......NN",
    ];
    HeroWalk2, false, [
        "...TTTT...",
        "..TTTTTT..",
        "..SSSSKS..",
        "..SSSSSS..",
        ".TTTTTTTT.",
        "SSTBBBBTSS",
        ".STBBBBTS.",
        "..BBBBBB..",
        "...BBBB...",
        "...BBBB...",
        "...NNNN...",
        "...NNNN...",
    ];
    HeroJump, false, [
        "S..TTTT..S",
        "S.TTTTTT.S",
        "STSSSSKSTS",
        ".TSSSSSST.",
        "..TTTTTT..",
        "..TBBBBT..",
        "..BBBBBB..",
        "..BBBBBB..",
        ".BBB..BBB.",
        "BB......BB",
        "NN......NN",
        "N........N",
    ];
    HeroClimb, false, [
        "...TTTT...",
        "..TTTTTT..",
        "..NNNNNN..",
        "S.NNNNNN..",
        "SSTTTTTT..",
        "..TTTTTTSS",
        "..BBBBBB.S",
        "..BBBBBB..",
        "..BB..BB..",
        "..BB..BB..",
        "..NN..NN..",
        "..NN......",
    ];
    HeroHammer, false, [
        "...TTTT...",
        "..TTTTTT..",
        "..SSSSKS..",
        "..SSSSSS..",
        ".TTTTTTTSS",
        ".TTBBBBTSS",
        "..TBBBBT..",
        "..BBBBBB..",
        "..BB..BB..",
        "..BB..BB..",
        ".NNN..NNN.",
        ".NNN..NNN.",
    ];
    HeroDead, false, [
        "S........S",
        "SS.TTTT.SS",
        ".STTTTTTS.",
        "..SKSSKS..",
        "..SSSSSS..",
        "..TTTTTT..",
        "..TBBBBT..",
        "..BBBBBB..",
        "..BB..BB..",
        "..BB..BB..",
        ".NNN..NNN.",
        ".NNN..NNN.",
    ];
    // The hammer in the hand: raised (8 x 10) and swung forward (10 x 6).
    HammerUp, false, [
        "WGGGGGGK",
        "GGGGGGGK",
        "WGGGGGGK",
        "...NN...",
        "...NN...",
        "...NN...",
        "...NN...",
        "...NN...",
        "...NN...",
        "...NN...",
    ];
    HammerSide, false, [
        "......WGGK",
        "......GGGK",
        "NNNNNNGGGK",
        "NNNNNNGGGK",
        "......GGGK",
        "......WGGK",
    ];
    // The ape: the rows hold the left half (12 x 24 -> 24 x 24).
    ApeIdle, true, [
        "....PPPPPPPP",
        "...PPPPPPPPP",
        "...PPLLLLLLL",
        "...PLLLKLLLL",
        "...PLLLLLLLL",
        "...PLLLLKKKK",
        "....PLLLLLLL",
        ".....PPPPPPP",
        "..PPPPPPPPPP",
        ".PPPPPPPPPPP",
        "PPPPPPLLLLLL",
        "PPPPPLLLLLLL",
        "PPPPPLLLLLLL",
        "PPPPPPLLLLLL",
        "PPPPPPPPPPPP",
        "PPPPPPPPPPPP",
        "LLPPPPPPPPPP",
        "LLPPPPPPPPPP",
        ".PPPPPPPPPPP",
        "..PPPPPPPPPP",
        "..PPPPPPPPPP",
        "..PPPPPPPPPP",
        ".PPPPPPPP...",
        "LLLLLLLLL...",
    ];
    ApeBeat, true, [
        "....PPPPPPPP",
        "...PPPPPPPPP",
        "...PPLLLLLLL",
        "...PLLLKLLLL",
        "LL.PLLLLLLLL",
        "LL.PLLLLKKKK",
        "LLPPLLLLLLLL",
        "PP...PPPPPPP",
        "..PPPPPPPPPP",
        ".PPPPPPPPPPP",
        "PPPPPPLLLLLL",
        "PPPPPLLLLLLL",
        "PPPPPLLLLLLL",
        "PPPPPPLLLLLL",
        "PPPPPPPPPPPP",
        "PPPPPPPPPPPP",
        "PPPPPPPPPPPP",
        "PPPPPPPPPPPP",
        ".PPPPPPPPPPP",
        "..PPPPPPPPPP",
        "..PPPPPPPPPP",
        "..PPPPPPPPPP",
        ".PPPPPPPP...",
        "LLLLLLLLL...",
    ];
    ApeThrow, true, [
        "....PPPPPPPP",
        "...PPPPPPPPP",
        "...PPLLLLLLL",
        "...PLLLKLLLL",
        "...PLLLLLLLL",
        "...PLLLLKKKK",
        "....PLLLLLLL",
        ".....PPPPPPP",
        "..PPPPPPPPPP",
        ".PPPPPPPPPPP",
        "PPPPPPLLLLLL",
        "PPPPPLLLLLLL",
        "PPPPPLLLLLLL",
        "PPPPPPLLLLLL",
        "PPPPPPPPLLLL",
        "PPPPPPPPLLLL",
        "PPPPPPPPLLLL",
        "PPPPPPPPLLLL",
        ".PPPPPPPPPPP",
        "..PPPPPPPPPP",
        "..PPPPPPPPPP",
        "..PPPPPPPPPP",
        ".PPPPPPPP...",
        "LLLLLLLLL...",
    ];
    // The friend at the goal (6 x 16 half -> 12 x 16).
    Friend, true, [
        "..YYYY",
        ".YYYYY",
        ".YYSSS",
        ".YSSKS",
        ".YYSSS",
        "..YSSS",
        "..MMMM",
        ".SMMMM",
        ".SMMMM",
        "..MMMM",
        ".MMMMM",
        "MMMMMM",
        "MMMMMM",
        "MMMMMM",
        "..SS..",
        "..NN..",
    ];
    // Rolling barrel: the marks turn a quarter per frame.
    Barrel0, false, [
        "..KKKK..",
        ".KOOOOK.",
        "KOYOOOOK",
        "KOOOOOOK",
        "KOOOOOOK",
        "KOOOONOK",
        ".KOOOOK.",
        "..KKKK..",
    ];
    Barrel1, false, [
        "..KKKK..",
        ".KOOOOK.",
        "KOOOOYOK",
        "KOOOOOOK",
        "KOOOOOOK",
        "KONOOOOK",
        ".KOOOOK.",
        "..KKKK..",
    ];
    Barrel2, false, [
        "..KKKK..",
        ".KOOOOK.",
        "KONOOOOK",
        "KOOOOOOK",
        "KOOOOOOK",
        "KOOOOYOK",
        ".KOOOOK.",
        "..KKKK..",
    ];
    Barrel3, false, [
        "..KKKK..",
        ".KOOOOK.",
        "KOOOONOK",
        "KOOOOOOK",
        "KOOOOOOK",
        "KOYOOOOK",
        ".KOOOOK.",
        "..KKKK..",
    ];
    // A barrel climbing down a ladder, seen from the front.
    BarrelFront, false, [
        "..KKKK..",
        ".KOYYOK.",
        "KOOYYOOK",
        "KNNNNNNK",
        "KNNNNNNK",
        "KOOYYOOK",
        ".KOYYOK.",
        "..KKKK..",
    ];
    Explosion0, false, [
        "..........",
        "..........",
        "....YY....",
        "...YOOY...",
        "..YOWWOY..",
        "..YOWWOY..",
        "...YOOY...",
        "....YY....",
        "..........",
        "..........",
    ];
    Explosion1, false, [
        "....YY....",
        ".Y..YY..Y.",
        "..YOOOOY..",
        "..OOWWOO..",
        "YYOWWWWOYY",
        "YYOWWWWOYY",
        "..OOWWOO..",
        "..YOOOOY..",
        ".Y..YY..Y.",
        "....YY....",
    ];
    Explosion2, false, [
        "..Y.YY.Y..",
        "...O..O...",
        ".Y......Y.",
        "..........",
        "Y...OO...Y",
        "Y...OO...Y",
        "..........",
        ".Y......Y.",
        "...O..O...",
        "..Y.YY.Y..",
    ];
    // Fireballs of the later levels (#14).
    Fire0, false, [
        "...F....",
        "...FF...",
        "..FFF.F.",
        ".FFOFFF.",
        ".FOOOFF.",
        "FFOYOOFF",
        "FOYYYOOF",
        "FOYYYYOF",
        ".FOYYOF.",
        "..FFFF..",
    ];
    Fire1, false, [
        "....F...",
        "...FF...",
        ".F.FFF..",
        ".FFFOFF.",
        ".FFOOOF.",
        "FFOOYOFF",
        "FOOYYYOF",
        "FOYYYYOF",
        ".FOYYOF.",
        "..FFFF..",
    ];
}

/// The textures of all sprites.
pub(in crate::app) struct Sprites {
    textures: Vec<egui::TextureHandle>,
}

impl Sprites {
    pub(in crate::app) fn load(ctx: &egui::Context) -> Self {
        let textures = Id::ALL
            .iter()
            .map(|&id| {
                let d =
                    decode(&id.def()).expect("a built-in sprite decodes (checked by the tests)");
                let image = egui::ColorImage::new(d.size, d.pixels);
                ctx.load_texture(
                    format!("sprite-{id:?}"),
                    image,
                    egui::TextureOptions::NEAREST,
                )
            })
            .collect();
        Self { textures }
    }

    pub(in crate::app) fn get(&self, id: Id) -> &egui::TextureHandle {
        &self.textures[id as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn size(id: Id) -> [usize; 2] {
        decode(&id.def()).expect("decodes").size
    }

    #[test]
    fn every_sprite_decodes() {
        for &id in Id::ALL {
            let d = decode(&id.def()).unwrap_or_else(|e| panic!("{id:?}: {e}"));
            assert_eq!(d.pixels.len(), d.size[0] * d.size[1], "{id:?}");
            assert!(
                d.pixels.iter().any(|p| p.a() > 0),
                "{id:?} is fully transparent"
            );
        }
    }

    #[test]
    fn sprites_have_the_sizes_the_renderer_assumes() {
        use Id::*;
        for id in [
            HeroStand, HeroWalk1, HeroWalk2, HeroJump, HeroClimb, HeroHammer, HeroDead,
        ] {
            assert_eq!(size(id), [10, 12], "{id:?}");
        }
        for id in [ApeIdle, ApeBeat, ApeThrow] {
            assert_eq!(size(id), [24, 24], "{id:?}");
        }
        for id in [Barrel0, Barrel1, Barrel2, Barrel3, BarrelFront] {
            assert_eq!(size(id), [8, 8], "{id:?}");
        }
        assert_eq!(size(Friend), [12, 16]);
        assert_eq!(size(HammerUp), [8, 10]);
        assert_eq!(size(HammerSide), [10, 6]);
        for id in [Explosion0, Explosion1, Explosion2] {
            assert_eq!(size(id), [10, 10], "{id:?}");
        }
        for id in [Fire0, Fire1] {
            assert_eq!(size(id), [8, 10], "{id:?}");
        }
    }

    #[test]
    fn a_bad_sprite_is_reported() {
        let ragged = SpriteDef {
            rows: &["RR", "R"],
            mirror: false,
        };
        assert!(decode(&ragged).is_err());
        let unknown = SpriteDef {
            rows: &["R?"],
            mirror: false,
        };
        assert!(decode(&unknown).is_err());
    }

    #[test]
    fn mirrored_sprites_are_symmetric() {
        let d = decode(&Id::Friend.def()).expect("decodes");
        let [w, h] = d.size;
        for y in 0..h {
            for x in 0..w / 2 {
                assert_eq!(d.pixels[y * w + x], d.pixels[y * w + (w - 1 - x)]);
            }
        }
    }
}
