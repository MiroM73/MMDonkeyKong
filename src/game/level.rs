//! The level file format and its parser / validator. Pure: text in, `Level` out.
//!
//! One `key: values` per line, `#` starts a comment, blank lines are ignored. Keys: `name`,
//! `type`, `bonus`, `bonus_drop`, `bonus_every` (seconds), `barrel_every` (seconds), `extra_life`,
//! `start x y`, `ape x y`, `goal x y`, `girder x0 y0 x1 y1`, `ladder x top bottom [broken]`,
//! `hammer x y`, `bonus_hammer x y`, `conveyor girder left|right seconds` (the girder is the
//! 0-based number of its `girder` line; the belt reverses every `seconds`, 0 = never),
//! `lift x top bottom up|down` (a platform that rides between the surface heights `top` and
//! `bottom` and starts again at the other end). Errors carry the line number (`ParseError`).

use std::fmt;

use super::{FIELD_H, FIELD_W, Geometry, Girder, Ladder, STEP_HZ};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LevelKind {
    Barrels,
    Conveyors,
    Lifts,
}

impl LevelKind {
    /// Every level type (the generator tests loop over it).
    pub const ALL: &[LevelKind] = &[LevelKind::Barrels, LevelKind::Conveyors, LevelKind::Lifts];

    /// The name used after `type:` in a level file.
    pub fn as_str(self) -> &'static str {
        match self {
            LevelKind::Barrels => "barrels",
            LevelKind::Conveyors => "conveyors",
            LevelKind::Lifts => "lifts",
        }
    }
}

/// A conveyor belt on the girder with the number `girder`: it carries whoever stands on it in
/// direction `dir` (-1 / 1) and reverses every `every` steps (0 = never).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Belt {
    pub girder: usize,
    pub dir: i8,
    pub every: u32,
}

/// A lift: a platform `LIFT_WIDTH` wide centred on `x` that rides between the surface heights
/// `top` and `bottom` (`top < bottom`), upwards when `dir` is -1, and reappears at the other end.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lift {
    pub x: f32,
    pub top: f32,
    pub bottom: f32,
    pub dir: i8,
}

/// Width of a lift platform.
pub const LIFT_WIDTH: f32 = 16.0;

#[derive(Clone, Debug, PartialEq)]
pub struct Level {
    pub name: String,
    pub kind: LevelKind,
    /// Time bonus at the start, how much it loses per drop and how many steps between drops.
    pub bonus: u32,
    pub bonus_drop: u32,
    pub bonus_every: u32,
    /// Steps between two barrels thrown by the ape.
    pub barrel_every: u32,
    /// Score that gives one extra life.
    pub extra_life: u32,
    /// Where the player starts (middle of the feet).
    pub start: (f32, f32),
    pub ape: (f32, f32),
    pub goal: (f32, f32),
    pub geometry: Geometry,
    pub hammers: Vec<(f32, f32)>,
    /// Extra hammers placed only on the easier difficulties (`bonus_hammer x y`).
    pub bonus_hammers: Vec<(f32, f32)>,
    /// Conveyor belts (the `conveyors` type).
    pub belts: Vec<Belt>,
    /// Lifts (the `lifts` type).
    pub lifts: Vec<Lift>,
}

/// A problem in a level file; `line` is 1-based, 0 means the file as a whole.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line == 0 {
            write!(f, "{}", self.message)
        } else {
            write!(f, "line {}: {}", self.line, self.message)
        }
    }
}

fn err<T>(line: usize, message: impl Into<String>) -> Result<T, ParseError> {
    Err(ParseError {
        line,
        message: message.into(),
    })
}

/// `n` numbers separated by spaces, all finite.
fn numbers(line: usize, key: &str, value: &str, n: usize) -> Result<Vec<f32>, ParseError> {
    let parts: Vec<&str> = value.split_whitespace().collect();
    if parts.len() != n {
        return err(
            line,
            format!("'{key}' needs {n} numbers, found {}", parts.len()),
        );
    }
    parts
        .iter()
        .map(|p| match p.parse::<f32>() {
            Ok(v) if v.is_finite() => Ok(v),
            _ => err(line, format!("'{p}' is not a number")),
        })
        .collect()
}

fn need<T>(v: Option<T>, key: &str) -> Result<T, ParseError> {
    v.map_or_else(|| err(0, format!("missing '{key}'")), Ok)
}

fn one_number(line: usize, key: &str, value: &str) -> Result<f32, ParseError> {
    Ok(numbers(line, key, value, 1)?[0])
}

fn in_field(line: usize, x: f32, y: f32) -> Result<(), ParseError> {
    if (0.0..=FIELD_W as f32).contains(&x) && (0.0..=FIELD_H as f32).contains(&y) {
        Ok(())
    } else {
        err(line, format!("point ({x}, {y}) is outside the playfield"))
    }
}

fn point(line: usize, key: &str, value: &str) -> Result<(f32, f32), ParseError> {
    let v = numbers(line, key, value, 2)?;
    in_field(line, v[0], v[1])?;
    Ok((v[0], v[1]))
}

fn steps(line: usize, key: &str, value: &str) -> Result<u32, ParseError> {
    let s = one_number(line, key, value)?;
    if s <= 0.0 || s > 600.0 {
        return err(line, format!("'{key}' must be between 0 and 600 seconds"));
    }
    Ok(((s * STEP_HZ as f32).round() as u32).max(1))
}

fn count(line: usize, key: &str, value: &str) -> Result<u32, ParseError> {
    let v = one_number(line, key, value)?;
    if v < 0.0 || v.fract() != 0.0 || v > 1_000_000.0 {
        return err(
            line,
            format!("'{key}' must be a whole number from 0 to 1000000"),
        );
    }
    Ok(v as u32)
}

impl Level {
    /// Parses a level file. Syntax and range problems are reported with their line number;
    /// whether the level can be played is checked by [`Level::validate`].
    pub fn parse(text: &str) -> Result<Level, ParseError> {
        let mut name = None;
        let mut kind = None;
        let mut bonus = None;
        let mut bonus_drop = None;
        let mut bonus_every = None;
        let mut barrel_every = None;
        let mut extra_life = None;
        let mut start = None;
        let mut ape = None;
        let mut goal = None;
        let mut geometry = Geometry::default();
        let mut hammers = Vec::new();
        let mut bonus_hammers = Vec::new();
        let mut belts = Vec::new();
        let mut lifts = Vec::new();

        for (i, raw) in text.lines().enumerate() {
            let line = i + 1;
            let content = raw.split('#').next().unwrap_or("").trim();
            if content.is_empty() {
                continue;
            }
            let Some((key, value)) = content.split_once(':') else {
                return err(line, "expected 'key: value'");
            };
            let (key, value) = (key.trim(), value.trim());
            let dup = |set: bool| {
                if set {
                    err(line, format!("'{key}' is given twice"))
                } else {
                    Ok(())
                }
            };
            match key {
                "name" => {
                    dup(name.is_some())?;
                    if value.is_empty() {
                        return err(line, "'name' is empty");
                    }
                    name = Some(value.to_string());
                }
                "type" => {
                    dup(kind.is_some())?;
                    kind = Some(match LevelKind::ALL.iter().find(|k| k.as_str() == value) {
                        Some(k) => *k,
                        None => return err(line, format!("unknown level type '{value}'")),
                    });
                }
                "bonus" => {
                    dup(bonus.is_some())?;
                    bonus = Some(count(line, key, value)?);
                }
                "bonus_drop" => {
                    dup(bonus_drop.is_some())?;
                    bonus_drop = Some(count(line, key, value)?);
                }
                "bonus_every" => {
                    dup(bonus_every.is_some())?;
                    bonus_every = Some(steps(line, key, value)?);
                }
                "barrel_every" => {
                    dup(barrel_every.is_some())?;
                    barrel_every = Some(steps(line, key, value)?);
                }
                "extra_life" => {
                    dup(extra_life.is_some())?;
                    extra_life = Some(count(line, key, value)?);
                }
                "start" => {
                    dup(start.is_some())?;
                    start = Some(point(line, key, value)?);
                }
                "ape" => {
                    dup(ape.is_some())?;
                    ape = Some(point(line, key, value)?);
                }
                "goal" => {
                    dup(goal.is_some())?;
                    goal = Some(point(line, key, value)?);
                }
                "hammer" => hammers.push(point(line, key, value)?),
                "bonus_hammer" => bonus_hammers.push(point(line, key, value)?),
                "conveyor" => {
                    let parts: Vec<&str> = value.split_whitespace().collect();
                    let [girder, dir, every] = parts[..] else {
                        return err(
                            line,
                            "'conveyor' needs a girder number, left / right and seconds",
                        );
                    };
                    let girder = count(line, key, girder)? as usize;
                    let dir = match dir {
                        "left" => -1,
                        "right" => 1,
                        _ => return err(line, format!("'{dir}' is not left or right")),
                    };
                    let every = if one_number(line, key, every)? == 0.0 {
                        0
                    } else {
                        steps(line, key, every)?
                    };
                    belts.push(Belt { girder, dir, every });
                }
                "lift" => {
                    let (nums, dir) = match (value.strip_suffix("up"), value.strip_suffix("down")) {
                        (Some(rest), _) => (rest, -1),
                        (_, Some(rest)) => (rest, 1),
                        _ => return err(line, "'lift' needs x top bottom up|down"),
                    };
                    let v = numbers(line, key, nums, 3)?;
                    in_field(line, v[0], v[1])?;
                    in_field(line, v[0], v[2])?;
                    if v[1] >= v[2] {
                        return err(line, "a lift needs top < bottom");
                    }
                    lifts.push(Lift {
                        x: v[0],
                        top: v[1],
                        bottom: v[2],
                        dir,
                    });
                }
                "girder" => {
                    let v = numbers(line, key, value, 4)?;
                    in_field(line, v[0], v[1])?;
                    in_field(line, v[2], v[3])?;
                    if v[0] >= v[2] {
                        return err(line, "a girder needs x0 < x1");
                    }
                    geometry.girders.push(Girder::new(v[0], v[1], v[2], v[3]));
                }
                "ladder" => {
                    let (nums, broken) = match value.strip_suffix("broken") {
                        Some(rest) => (rest, true),
                        None => (value, false),
                    };
                    let v = numbers(line, key, nums, 3)?;
                    in_field(line, v[0], v[1])?;
                    in_field(line, v[0], v[2])?;
                    if v[1] >= v[2] {
                        return err(line, "a ladder needs top < bottom");
                    }
                    geometry.ladders.push(Ladder {
                        x: v[0],
                        top: v[1],
                        bottom: v[2],
                        broken,
                    });
                }
                _ => return err(line, format!("unknown key '{key}'")),
            }
        }

        let level = Level {
            name: need(name, "name")?,
            kind: need(kind, "type")?,
            bonus: need(bonus, "bonus")?,
            bonus_drop: bonus_drop.unwrap_or(0),
            bonus_every: bonus_every.unwrap_or(120),
            barrel_every: need(barrel_every, "barrel_every")?,
            extra_life: extra_life.unwrap_or(0),
            start: need(start, "start")?,
            ape: need(ape, "ape")?,
            goal: need(goal, "goal")?,
            geometry,
            hammers,
            bonus_hammers,
            belts,
            lifts,
        };
        if let Some(b) = level
            .belts
            .iter()
            .find(|b| b.girder >= level.geometry.girders.len())
        {
            return err(
                0,
                format!("conveyor: there is no girder number {}", b.girder),
            );
        }
        Ok(level)
    }

    /// Checks that the level can be played (rules in `validate.rs`).
    pub fn validate(&self) -> Result<(), String> {
        super::validate::validate(self)
    }

    /// The level in the file format; `Level::parse(&l.to_text())` gives `l` back.
    #[allow(dead_code)] // the editor (#25) exports with it
    pub fn to_text(&self) -> String {
        let secs = |steps: u32| steps as f32 / STEP_HZ as f32;
        let mut lines = vec![
            format!("name: {}", self.name),
            format!("type: {}", self.kind.as_str()),
            format!("bonus: {}", self.bonus),
            format!("bonus_drop: {}", self.bonus_drop),
            format!("bonus_every: {}", secs(self.bonus_every)),
            format!("barrel_every: {}", secs(self.barrel_every)),
            format!("extra_life: {}", self.extra_life),
            format!("start: {} {}", self.start.0, self.start.1),
            format!("ape: {} {}", self.ape.0, self.ape.1),
            format!("goal: {} {}", self.goal.0, self.goal.1),
        ];
        for g in &self.geometry.girders {
            lines.push(format!("girder: {} {} {} {}", g.x0, g.y0, g.x1, g.y1));
        }
        for l in &self.geometry.ladders {
            let broken = if l.broken { " broken" } else { "" };
            lines.push(format!("ladder: {} {} {}{broken}", l.x, l.top, l.bottom));
        }
        for (key, list) in [
            ("hammer", &self.hammers),
            ("bonus_hammer", &self.bonus_hammers),
        ] {
            for h in list {
                lines.push(format!("{key}: {} {}", h.0, h.1));
            }
        }
        for b in &self.belts {
            let dir = if b.dir < 0 { "left" } else { "right" };
            lines.push(format!("conveyor: {} {dir} {}", b.girder, secs(b.every)));
        }
        for l in &self.lifts {
            let dir = if l.dir < 0 { "up" } else { "down" };
            lines.push(format!("lift: {} {} {} {dir}", l.x, l.top, l.bottom));
        }
        lines.push(String::new());
        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINI: &str = "name: T\ntype: barrels\nbonus: 1000\nbarrel_every: 2\n\
        start: 10 100\nape: 10 100\ngoal: 150 100\ngirder: 0 100 200 100\n";

    #[test]
    fn parses_a_minimal_level() {
        let l = Level::parse(MINI).unwrap();
        assert_eq!(l.name, "T");
        assert_eq!(l.barrel_every, 120);
        assert_eq!(l.bonus_every, 120); // default 2 s
        assert_eq!(l.geometry.girders.len(), 1);
        assert_eq!(l.validate(), Ok(()));
    }

    #[test]
    fn comments_blank_lines_and_broken_ladders() {
        let text = format!("# hi\n\n{MINI}ladder: 50 60 100 broken # trailing\n");
        let l = Level::parse(&text).unwrap();
        assert_eq!(l.geometry.ladders.len(), 1);
        assert!(l.geometry.ladders[0].broken);
    }

    fn error_of(text: &str) -> String {
        Level::parse(text).unwrap_err().to_string()
    }

    #[test]
    fn errors_have_line_numbers() {
        assert_eq!(
            error_of("name: A\nbogus: 1\n"),
            "line 2: unknown key 'bogus'"
        );
        assert_eq!(error_of("name A\n"), "line 1: expected 'key: value'");
        assert_eq!(
            error_of("type: zeppelins\n"),
            "line 1: unknown level type 'zeppelins'"
        );
        assert_eq!(
            error_of("name: A\nname: B\n"),
            "line 2: 'name' is given twice"
        );
        assert_eq!(
            error_of("girder: 10 10 5 10\n"),
            "line 1: a girder needs x0 < x1"
        );
        assert_eq!(
            error_of("girder: 0 10 5\n"),
            "line 1: 'girder' needs 4 numbers, found 3"
        );
        assert_eq!(error_of("start: 1 x\n"), "line 1: 'x' is not a number");
        assert_eq!(
            error_of("start: 1 999\n"),
            "line 1: point (1, 999) is outside the playfield"
        );
        assert_eq!(
            error_of("ladder: 10 50 20\n"),
            "line 1: a ladder needs top < bottom"
        );
        assert_eq!(
            error_of("barrel_every: 0\n"),
            "line 1: 'barrel_every' must be between 0 and 600 seconds"
        );
        assert_eq!(
            error_of("bonus: -5\n"),
            "line 1: 'bonus' must be a whole number from 0 to 1000000"
        );
    }

    #[test]
    fn to_text_round_trips() {
        for text in [
            include_str!("../../levels/01-barrels.txt"),
            include_str!("../../levels/02-conveyors.txt"),
            include_str!("../../levels/03-lifts.txt"),
        ] {
            let l = Level::parse(text).unwrap();
            assert_eq!(Level::parse(&l.to_text()).unwrap(), l);
        }
    }

    #[test]
    fn conveyor_lines_are_parsed_and_checked() {
        let l = Level::parse(&format!(
            "{MINI}conveyor: 0 left 4.5\nconveyor: 0 right 0\n"
        ))
        .unwrap();
        assert_eq!(l.belts[0].dir, -1);
        assert_eq!(l.belts[0].every, 270);
        assert_eq!((l.belts[1].dir, l.belts[1].every), (1, 0));
        assert_eq!(
            error_of(&format!(
                "{MINI}conveyor: 3 left 4
"
            )),
            "conveyor: there is no girder number 3"
        );
        assert_eq!(
            error_of("lift: 50 10 90\n"),
            "line 1: 'lift' needs x top bottom up|down"
        );
        assert_eq!(
            error_of("lift: 50 90 10 up\n"),
            "line 1: a lift needs top < bottom"
        );
        assert_eq!(
            error_of(
                "conveyor: 0 up 4
"
            ),
            "line 1: 'up' is not left or right"
        );
        assert_eq!(
            error_of(
                "conveyor: 0 left
"
            ),
            "line 1: 'conveyor' needs a girder number, left / right and seconds"
        );
    }

    #[test]
    fn missing_keys_are_reported_for_the_whole_file() {
        assert_eq!(error_of("name: A\n"), "missing 'type'");
        assert_eq!(error_of(""), "missing 'name'");
    }

    #[test]
    fn validation_finds_unplayable_levels() {
        let mut l = Level::parse(MINI).unwrap();
        l.start = (10.0, 50.0);
        assert_eq!(l.validate(), Err("the start is not on a girder".into()));

        // Goal on a second girder with no ladder to it, then with a broken ladder, then intact.
        let two = format!("{MINI}girder: 0 60 200 60\nstart: 10 100\n")
            .replace("start: 10 100\nape", "ape")
            .replace("goal: 150 100", "goal: 150 60");
        let l = Level::parse(&two).unwrap();
        assert_eq!(
            l.validate(),
            Err("the goal cannot be reached from the start".into())
        );
        let broken = format!("{two}ladder: 50 60 100 broken\n");
        assert!(Level::parse(&broken).unwrap().validate().is_err());
        let ok = format!("{two}ladder: 50 60 100\n");
        assert_eq!(Level::parse(&ok).unwrap().validate(), Ok(()));
        let floating = format!("{two}ladder: 50 60 80\n");
        assert_eq!(
            Level::parse(&floating).unwrap().validate(),
            Err("the bottom of the ladder at x=50 is not on a girder".into())
        );
    }
}
