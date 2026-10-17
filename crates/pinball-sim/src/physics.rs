//! Ball physics for the simulator: one ball rolling down the sloped playfield,
//! bouncing off walls and posts, and driven by the real coil outputs.
//!
//! The geometry comes from `docs/playfield.svg`, so the map stays the single
//! source of truth:
//!
//! * `data-wall` elements are solid (the outline, rails, posts, the apron)
//! * `data-gate` is a one-way gate that only stops a ball rolling down
//! * a circle or polygon with a switch and a coil is a pop bumper or kicker:
//!   touching it closes the switch, and when the game fires the coil the part
//!   kicks the ball away
//! * a polygon with only a switch is a rubber that closes its switch on contact
//! * a circle or rectangle with only a switch is a rollover the ball runs over
//! * `data-trapdoor` swallows the ball while its flap coil is on
//! * `data-flipper` rises while either of its coils is on
//! * the outhole switch's circle catches a drained ball, and the ball-serve
//!   coil moves it to the shooter lane
//!
//! So nothing is scripted: a ball touching a pop bumper closes its switch, the
//! real rules in `pinball-core` fire the coil, and the coil output kicks the
//! ball. Units are SVG pixels (about a millimetre on the real table) and
//! seconds.

use std::ops::{Add, Mul, Neg, Sub};

use pinball_core::playfield::{coil, sw};
use pinball_core::{Instant, SwitchId, SwitchStates};

use crate::switch_from_index;

/// The ball's radius. A real ball is 27 mm across; the map isn't to scale, and
/// 20 px fits its lanes.
pub const BALL_RADIUS: f32 = 10.0;
/// Pull down the table: g × sin 6.5°, the usual playfield slope, in px/s².
pub const GRAVITY: f32 = 1_100.0;
/// Physics steps per 1 ms machine tick.
const SUBSTEPS: u8 = 4;
/// Fastest the ball may go, so it can't pass through a wall in one step.
const MAX_SPEED: f32 = 3_500.0;
/// Rolling resistance: the fraction of speed lost per second.
const ROLLING_LOSS: f32 = 0.25;

/// How bouncy each kind of surface is (coefficient of restitution).
const WALL_BOUNCE: f32 = 0.45;
const RUBBER_BOUNCE: f32 = 0.7;
const FLIPPER_BOUNCE: f32 = 0.35;
/// Below this speed into a surface the ball is resting on it, not bouncing.
const RESTING_SPEED: f32 = 40.0;
/// Speed along a surface lost in an impact.
const IMPACT_GRIP: f32 = 0.05;
/// How fast a pop bumper and a kicker throw the ball when they fire.
const POP_KICK: f32 = 1_600.0;
const KICKER_KICK: f32 = 1_400.0;
/// A coil only kicks the ball if it touched the part this recently.
const KICK_WINDOW_MS: u32 = 15;
/// Shortest time a touched switch stays closed, so the debouncer sees it.
const MIN_CLOSED_MS: u32 = 10;

/// How far a flipper turns from rest, and how fast (rad/s).
const FLIPPER_STROKE: f32 = 0.9;
const FLIPPER_UP_SPEED: f32 = 22.0;
const FLIPPER_DOWN_SPEED: f32 = 14.0;

/// Plunger: a full pull takes this long, and launches the ball this fast.
const PLUNGER_FULL_MS: u16 = 800;
const PLUNGER_MIN: f32 = 1_350.0;
const PLUNGER_MAX: f32 = 2_400.0;
/// How long a ball takes to roll from a trapdoor to the outhole underneath.
const UNDER_PLAYFIELD_MS: u32 = 800;

const SVG: &str = include_str!("../../../docs/playfield.svg");

/// A 2D point or vector.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec2 {
    /// Across the table, left to right.
    pub x: f32,
    /// Down the table, top to bottom.
    pub y: f32,
}

impl Vec2 {
    /// A vector.
    #[must_use]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y
    }

    fn len(self) -> f32 {
        self.dot(self).sqrt()
    }

    fn rotated(self, angle: f32) -> Self {
        let (s, c) = angle.sin_cos();
        Self::new(self.x * c - self.y * s, self.x * s + self.y * c)
    }
}

impl Add for Vec2 {
    type Output = Self;
    fn add(self, o: Self) -> Self {
        Self::new(self.x + o.x, self.y + o.y)
    }
}

impl Sub for Vec2 {
    type Output = Self;
    fn sub(self, o: Self) -> Self {
        Self::new(self.x - o.x, self.y - o.y)
    }
}

impl Mul<f32> for Vec2 {
    type Output = Self;
    fn mul(self, k: f32) -> Self {
        Self::new(self.x * k, self.y * k)
    }
}

impl Neg for Vec2 {
    type Output = Self;
    fn neg(self) -> Self {
        Self::new(-self.x, -self.y)
    }
}

/// The point on segment `a`–`b` closest to `p`, and how far along it is (0–1).
fn closest_on_segment(p: Vec2, a: Vec2, b: Vec2) -> (Vec2, f32) {
    let ab = b - a;
    let len2 = ab.dot(ab);
    let t = if len2 == 0.0 {
        0.0
    } else {
        ((p - a).dot(ab) / len2).clamp(0.0, 1.0)
    };
    (a + ab * t, t)
}

// ---------------------------------------------------------------------------
// Reading the SVG

/// One SVG start tag and its attributes.
#[derive(Debug)]
struct Tag {
    name: String,
    attrs: Vec<(String, String)>,
}

impl Tag {
    fn get(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }

    fn num(&self, name: &str) -> f32 {
        self.get(name).and_then(|v| v.parse().ok()).unwrap_or(0.0)
    }
}

/// Every start tag in `svg`, in order. Enough for our own hand-written file:
/// no entities, CDATA or attribute values containing `>`.
fn tags(svg: &str) -> Vec<Tag> {
    let mut out = Vec::new();
    for chunk in svg.split('<').skip(1) {
        let Some(body) = chunk.split('>').next() else {
            continue;
        };
        if body.starts_with(['/', '!', '?']) {
            continue;
        }
        let body = body.trim_end_matches('/');
        let mut rest = body.trim();
        let name_end = rest.find(char::is_whitespace).unwrap_or(rest.len());
        let name = rest[..name_end].to_owned();
        rest = &rest[name_end..];
        let mut attrs = Vec::new();
        while let Some(eq) = rest.find('=') {
            let key = rest[..eq].trim().to_owned();
            let after = rest[eq + 1..].trim_start();
            let Some(quote) = after.chars().next() else {
                break;
            };
            let value_and_rest = &after[1..];
            let close = value_and_rest.find(quote).unwrap_or(value_and_rest.len());
            attrs.push((key, value_and_rest[..close].to_owned()));
            rest = value_and_rest.get(close + 1..).unwrap_or("");
        }
        out.push(Tag { name, attrs });
    }
    out
}

fn numbers(text: &str) -> Vec<f32> {
    text.split(|c: char| c.is_whitespace() || c == ',')
        .filter_map(|n| n.parse().ok())
        .collect()
}

fn points(text: &str) -> Vec<Vec2> {
    numbers(text)
        .chunks_exact(2)
        .map(|p| Vec2::new(p[0], p[1]))
        .collect()
}

/// The outline of an SVG path with absolute `M`, `L`, `Q` and `Z` commands,
/// with each quadratic curve flattened into straight pieces. Returns the
/// points and whether the path is closed.
fn path_points(d: &str) -> (Vec<Vec2>, bool) {
    let mut out = Vec::new();
    let mut closed = false;
    let mut tokens = d.split_whitespace().peekable();
    let take = |tokens: &mut std::iter::Peekable<std::str::SplitWhitespace>| {
        let x: f32 = tokens.next().and_then(|t| t.parse().ok()).unwrap_or(0.0);
        let y: f32 = tokens.next().and_then(|t| t.parse().ok()).unwrap_or(0.0);
        Vec2::new(x, y)
    };
    while let Some(command) = tokens.next() {
        match command {
            "M" | "L" => out.push(take(&mut tokens)),
            "Q" => {
                let control = take(&mut tokens);
                let end = take(&mut tokens);
                let start = out.last().copied().unwrap_or(control);
                for i in 1..=8_u8 {
                    let t = f32::from(i) / 8.0;
                    let u = 1.0 - t;
                    out.push(start * (u * u) + control * (2.0 * u * t) + end * (t * t));
                }
            }
            "Z" => closed = true,
            _ => {}
        }
    }
    (out, closed)
}

fn edges(points: &[Vec2], closed: bool) -> Vec<(Vec2, Vec2)> {
    let mut out: Vec<_> = points.windows(2).map(|w| (w[0], w[1])).collect();
    if closed && points.len() > 2 {
        out.push((points[points.len() - 1], points[0]));
    }
    out
}

/// The outline of a drawn shape, if it's one the physics understands.
fn shape_of(tag: &Tag) -> Option<Shape> {
    match tag.name.as_str() {
        "circle" => Some(Shape::Circle {
            centre: Vec2::new(tag.num("cx"), tag.num("cy")),
            radius: tag.num("r"),
        }),
        "line" => Some(Shape::Edges(vec![(
            Vec2::new(tag.num("x1"), tag.num("y1")),
            Vec2::new(tag.num("x2"), tag.num("y2")),
        )])),
        "polygon" => Some(Shape::Edges(edges(
            &points(tag.get("points").unwrap_or("")),
            true,
        ))),
        "rect" => {
            let (x, y) = (tag.num("x"), tag.num("y"));
            let (w, h) = (tag.num("width"), tag.num("height"));
            let corners = [
                Vec2::new(x, y),
                Vec2::new(x + w, y),
                Vec2::new(x + w, y + h),
                Vec2::new(x, y + h),
            ];
            Some(Shape::Edges(edges(&corners, true)))
        }
        "path" => tag.get("d").map(|d| {
            let (p, closed) = path_points(d);
            Shape::Edges(edges(&p, closed))
        }),
        _ => None,
    }
}

fn switch_of(tag: &Tag) -> Option<SwitchId> {
    let label = tag.get("data-switch")?;
    let index = if let Some(rest) = label.strip_prefix('C') {
        let (column, row) = rest.split_once('R')?;
        column.parse::<u32>().ok()? * 8 + row.parse::<u32>().ok()?
    } else {
        64 + label.strip_prefix('D')?.parse::<u32>().ok()?
    };
    switch_from_index(index)
}

fn coils_of(tag: &Tag) -> Vec<usize> {
    tag.get("data-coil")
        .map(|c| {
            c.split_whitespace()
                .filter_map(|n| n.parse().ok())
                .collect()
        })
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// The table

/// A solid shape's outline.
#[derive(Clone, Debug)]
enum Shape {
    Circle { centre: Vec2, radius: f32 },
    Edges(Vec<(Vec2, Vec2)>),
}

impl Shape {
    /// The point on the surface closest to `p`, and the surface's thickness
    /// there (a circle's radius; 0 for an edge).
    fn closest(&self, p: Vec2) -> (Vec2, f32) {
        match self {
            Self::Circle { centre, radius } => (*centre, *radius),
            Self::Edges(edges) => {
                let mut best = (edges[0].0, f32::MAX);
                for &(a, b) in edges {
                    let (q, _) = closest_on_segment(p, a, b);
                    let d = (p - q).len();
                    if d < best.1 {
                        best = (q, d);
                    }
                }
                (best.0, 0.0)
            }
        }
    }

    /// How far `p` is from the surface (negative inside a circle).
    fn distance(&self, p: Vec2) -> f32 {
        let (q, thickness) = self.closest(p);
        (p - q).len() - thickness
    }
}

/// Something solid the ball bounces off.
#[derive(Clone, Debug)]
struct Solid {
    shape: Shape,
    bounce: f32,
    /// Closed while the ball touches it.
    switch: Option<SwitchId>,
    /// A coil that kicks the ball away when it fires, and how hard.
    kick: Option<(usize, f32)>,
}

/// Somewhere the ball rolls over, closing a switch.
#[derive(Clone, Debug)]
struct Rollover {
    shape: Shape,
    switch: SwitchId,
}

#[derive(Clone, Debug)]
struct Trapdoor {
    min: Vec2,
    max: Vec2,
    switch: SwitchId,
    flap: usize,
}

/// A flipper: a tapered bar turning about its pivot.
#[derive(Clone, Copy, Debug)]
struct Flipper {
    pivot: Vec2,
    length: f32,
    base_radius: f32,
    tip_radius: f32,
    rest: f32,
    /// Which way it turns to go up: -1 for the left flipper, +1 for the right.
    up: f32,
    coils: u32,
}

impl Flipper {
    fn from_tag(tag: &Tag) -> Option<Self> {
        let (p, _) = path_points(tag.get("d")?);
        if p.len() != 4 {
            return None;
        }
        let base = (p[0] + p[3]) * 0.5;
        let tip = (p[1] + p[2]) * 0.5;
        let along = tip - base;
        Some(Self {
            pivot: base,
            length: along.len(),
            base_radius: (p[0] - p[3]).len() / 2.0,
            tip_radius: (p[1] - p[2]).len() / 2.0,
            rest: along.y.atan2(along.x),
            up: if tag.get("data-flipper") == Some("left") {
                -1.0
            } else {
                1.0
            },
            coils: coils_of(tag).iter().map(|&c| 1 << c).sum(),
        })
    }

    fn tip(&self, turn: f32) -> Vec2 {
        self.pivot + Vec2::new(self.length, 0.0).rotated(self.rest + turn * self.up)
    }
}

/// Everything the ball can meet, read from the playfield map.
#[derive(Clone, Debug)]
pub struct Table {
    solids: Vec<Solid>,
    gates: Vec<(Vec2, Vec2)>,
    rollovers: Vec<Rollover>,
    trapdoors: Vec<Trapdoor>,
    flippers: Vec<Flipper>,
    outhole: Vec2,
    shooter_rest: Vec2,
}

impl Table {
    /// The table drawn in `docs/playfield.svg`.
    ///
    /// # Panics
    ///
    /// If the map has no outhole, shooter-lane switch or flippers.
    #[must_use]
    pub fn from_map() -> Self {
        Self::from_svg(SVG)
    }

    fn from_svg(svg: &str) -> Self {
        let mut table = Self {
            solids: Vec::new(),
            gates: Vec::new(),
            rollovers: Vec::new(),
            trapdoors: Vec::new(),
            flippers: Vec::new(),
            outhole: Vec2::default(),
            shooter_rest: Vec2::default(),
        };
        let mut found_outhole = false;
        let mut found_shooter = false;
        for tag in tags(svg) {
            let Some(shape) = shape_of(&tag) else {
                continue;
            };
            let switch = switch_of(&tag);
            let coils = coils_of(&tag);

            if tag.get("data-flipper").is_some() {
                table.flippers.extend(Flipper::from_tag(&tag));
            } else if tag.get("data-gate").is_some() {
                if let Shape::Edges(e) = shape {
                    table.gates.extend(e);
                }
            } else if tag.get("data-trapdoor").is_some() {
                let (x, y) = (tag.num("x"), tag.num("y"));
                table.trapdoors.push(Trapdoor {
                    min: Vec2::new(x, y),
                    max: Vec2::new(x + tag.num("width"), y + tag.num("height")),
                    switch: switch.unwrap_or(sw::TRAPDOORS[0]),
                    flap: coils.first().copied().unwrap_or(0),
                });
            } else if switch == Some(sw::OUTHOLE) {
                if let Shape::Circle { centre, .. } = shape {
                    table.outhole = centre;
                    found_outhole = true;
                }
            } else if switch == Some(sw::SHOOTER_LANE) {
                if let Shape::Circle { centre, .. } = shape {
                    table.shooter_rest = centre;
                    found_shooter = true;
                }
                table.rollovers.push(Rollover {
                    shape,
                    switch: sw::SHOOTER_LANE,
                });
            } else if let Some(switch) = switch {
                let is_circle = matches!(shape, Shape::Circle { .. });
                let is_polygon = tag.name == "polygon";
                match (coils.first(), is_circle, is_polygon) {
                    // Pop bumpers and kickers.
                    (Some(&c), _, _) => table.solids.push(Solid {
                        shape,
                        bounce: RUBBER_BOUNCE,
                        switch: Some(switch),
                        kick: Some((c, if is_circle { POP_KICK } else { KICKER_KICK })),
                    }),
                    // Rubbers that are switches.
                    (None, _, true) => table.solids.push(Solid {
                        shape,
                        bounce: RUBBER_BOUNCE,
                        switch: Some(switch),
                        kick: None,
                    }),
                    // Rollovers.
                    (None, _, false) => table.rollovers.push(Rollover { shape, switch }),
                }
            } else if tag.get("data-wall").is_some() {
                let bounce = if matches!(shape, Shape::Circle { .. }) {
                    RUBBER_BOUNCE
                } else {
                    WALL_BOUNCE
                };
                table.solids.push(Solid {
                    shape,
                    bounce,
                    switch: None,
                    kick: None,
                });
            }
        }
        assert!(found_outhole, "the map has no outhole");
        assert!(found_shooter, "the map has no shooter-lane switch");
        assert_eq!(table.flippers.len(), 2, "the map should have two flippers");
        table
    }
}

// ---------------------------------------------------------------------------
// The ball

/// Where the ball is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BallState {
    /// On the playfield (including the shooter lane).
    Rolling,
    /// Under the playfield after dropping through a trapdoor, until the given
    /// time.
    Underneath {
        /// When it reaches the outhole.
        until: Instant,
    },
    /// Resting in the outhole, waiting to be served.
    InOuthole,
}

/// The ball, the flippers and the plunger.
#[derive(Clone, Debug)]
pub struct Physics {
    table: Table,
    pos: Vec2,
    vel: Vec2,
    state: BallState,
    /// How far each flipper has turned from rest (0 to the full stroke).
    flipper_turn: Vec<f32>,
    flipper_speed: Vec<f32>,
    /// When the plunger was pulled back, if it's held.
    plunger_from: Option<Instant>,
    /// The physics clock, which wraps like the machine's.
    now: Instant,
    prev_coils: u32,
    /// When the ball last touched each solid.
    touched: Vec<Option<Instant>>,
    /// Each switch the ball is holding closed, until when.
    closed_until: Vec<Option<Instant>>,
}

impl Default for Physics {
    fn default() -> Self {
        Self::new(Table::from_map())
    }
}

impl Physics {
    /// A ball resting in the outhole, as at power-on.
    #[must_use]
    pub fn new(table: Table) -> Self {
        let flippers = table.flippers.len();
        let solids = table.solids.len();
        Self {
            pos: table.outhole,
            vel: Vec2::default(),
            state: BallState::InOuthole,
            flipper_turn: vec![0.0; flippers],
            flipper_speed: vec![0.0; flippers],
            plunger_from: None,
            now: Instant::from_millis(0),
            prev_coils: 0,
            touched: vec![None; solids],
            closed_until: vec![None; pinball_core::switches::SWITCH_COUNT],
            table,
        }
    }

    /// The ball's position.
    #[must_use]
    pub const fn ball(&self) -> Vec2 {
        self.pos
    }

    /// The ball's velocity, in px/s.
    #[must_use]
    pub const fn velocity(&self) -> Vec2 {
        self.vel
    }

    /// Where the ball is.
    #[must_use]
    pub const fn state(&self) -> BallState {
        self.state
    }

    /// How far flipper `n` (0 = left, 1 = right) has turned up from rest, in
    /// radians, signed as the page should rotate it.
    #[must_use]
    pub fn flipper_turn(&self, n: usize) -> f32 {
        self.table
            .flippers
            .get(n)
            .map_or(0.0, |f| self.flipper_turn[n] * f.up)
    }

    /// How far the plunger is pulled back, 0 to 1.
    #[must_use]
    pub fn plunger_pull(&self) -> f32 {
        self.plunger_from.map_or(0.0, |from| {
            let held = self.now.since(from).min(u32::from(PLUNGER_FULL_MS));
            let held = u16::try_from(held).unwrap_or(PLUNGER_FULL_MS);
            f32::from(held) / f32::from(PLUNGER_FULL_MS)
        })
    }

    /// Whether the ball is sitting in the shooter lane, ready to plunge.
    #[must_use]
    pub fn ready_to_plunge(&self) -> bool {
        self.state == BallState::Rolling
            && (self.pos - self.table.shooter_rest).len() < BALL_RADIUS * 2.0
            && self.vel.len() < 60.0
    }

    /// Pulls the plunger back, or lets it go. Letting go launches the ball if
    /// it's in the shooter lane, harder the longer it was pulled.
    pub fn plunger(&mut self, pulled: bool) {
        if pulled {
            self.plunger_from.get_or_insert(self.now);
        } else if self.plunger_from.is_some() {
            let pull = self.plunger_pull();
            self.plunger_from = None;
            if self.ready_to_plunge() {
                self.vel = Vec2::new(0.0, -(PLUNGER_MIN + (PLUNGER_MAX - PLUNGER_MIN) * pull));
            }
        }
    }

    /// Bumps the table: a small shove up and to one side.
    pub fn nudge(&mut self) {
        if self.state == BallState::Rolling {
            let side = if self.now.as_millis() % 2 == 0 {
                1.0
            } else {
                -1.0
            };
            self.vel = self.vel + Vec2::new(120.0 * side, -180.0);
        }
    }

    /// Puts the ball at `pos` with velocity `vel`, rolling. For tests.
    pub fn place(&mut self, pos: Vec2, vel: Vec2) {
        self.pos = pos;
        self.vel = vel;
        self.state = BallState::Rolling;
    }

    /// Runs one 1 ms machine tick with the coil outputs from the last tick,
    /// and returns the switches the ball is closing.
    pub fn tick(&mut self, coils: u32) -> SwitchStates {
        self.now = self.now.plus(1);
        let fired = coils & !self.prev_coils;
        self.prev_coils = coils;

        // The ball-serve coil puts the ball in the shooter lane. Normally it's
        // waiting in the outhole; if the player ended a ball by hand while it
        // was still rolling, it comes back to the shooter lane all the same.
        if fired & (1 << coil::BALL_SERVE.index()) != 0 {
            self.place(self.table.shooter_rest, Vec2::default());
        }

        // Pop bumpers and kickers fire at a ball that just touched them.
        if self.state == BallState::Rolling {
            for (i, solid) in self.table.solids.iter().enumerate() {
                let Some((c, speed)) = solid.kick else {
                    continue;
                };
                let recent = self.touched[i].is_some_and(|t| self.now.since(t) <= KICK_WINDOW_MS);
                if fired & (1 << c) != 0
                    && recent
                    && solid.shape.distance(self.pos) <= BALL_RADIUS + 8.0
                {
                    let (q, _) = solid.shape.closest(self.pos);
                    let away = self.pos - q;
                    let n = away * (1.0 / away.len().max(0.001));
                    let along = self.vel.dot(n);
                    self.vel = self.vel + n * (speed - along.min(speed));
                }
            }
        }

        let dt = 0.001 / f32::from(SUBSTEPS);
        for _ in 0..SUBSTEPS {
            self.move_flippers(coils, dt);
            if self.state == BallState::Rolling {
                self.step(dt);
            }
        }

        match self.state {
            BallState::Underneath { until } if self.now.reached(until) => {
                self.state = BallState::InOuthole;
                self.pos = self.table.outhole;
            }
            BallState::Rolling => self.check_holes(coils),
            _ => {}
        }
        self.switches()
    }

    fn move_flippers(&mut self, coils: u32, dt: f32) {
        for (i, flipper) in self.table.flippers.iter().enumerate() {
            let up = coils & flipper.coils != 0;
            let turn = self.flipper_turn[i];
            let speed = if up {
                FLIPPER_UP_SPEED
            } else {
                -FLIPPER_DOWN_SPEED
            };
            let next = (turn + speed * dt).clamp(0.0, FLIPPER_STROKE);
            self.flipper_speed[i] = (next - turn) / dt;
            self.flipper_turn[i] = next;
        }
    }

    fn step(&mut self, dt: f32) {
        self.vel = self.vel + Vec2::new(0.0, GRAVITY * dt);
        self.vel = self.vel * (1.0 - ROLLING_LOSS * dt);
        let speed = self.vel.len();
        if speed > MAX_SPEED {
            self.vel = self.vel * (MAX_SPEED / speed);
        }
        self.pos = self.pos + self.vel * dt;

        for i in 0..self.table.solids.len() {
            let (q, thickness) = self.table.solids[i].shape.closest(self.pos);
            let bounce = self.table.solids[i].bounce;
            if self.collide(q, thickness, bounce, Vec2::default()) {
                self.touched[i] = Some(self.now);
                if let Some(s) = self.table.solids[i].switch {
                    self.close(s);
                }
            }
        }
        for g in 0..self.table.gates.len() {
            let (a, b) = self.table.gates[g];
            // The side of the gate facing up the table.
            let along = b - a;
            let mut up = Vec2::new(along.y, -along.x);
            if up.y > 0.0 {
                up = -up;
            }
            let up = up * (1.0 / up.len().max(0.001));
            // Only a ball above the gate and moving down onto it meets it.
            if (self.pos - a).dot(up) > 0.0 && self.vel.dot(up) < 0.0 {
                let (q, _) = closest_on_segment(self.pos, a, b);
                self.collide(q, 0.0, WALL_BOUNCE, Vec2::default());
            }
        }
        for i in 0..self.table.flippers.len() {
            let f = self.table.flippers[i];
            let turn = self.flipper_turn[i];
            let tip = f.tip(turn);
            let (q, t) = closest_on_segment(self.pos, f.pivot, tip);
            let radius = f.base_radius + (f.tip_radius - f.base_radius) * t;
            // The surface moves with the flipper: ω × r.
            let omega = self.flipper_speed[i] * f.up;
            let r = q - f.pivot;
            let surface = Vec2::new(-r.y, r.x) * omega;
            self.collide(q, radius, FLIPPER_BOUNCE, surface);
        }
    }

    /// Pushes the ball out of a surface it overlaps and bounces it. Returns
    /// whether they touched.
    fn collide(&mut self, closest: Vec2, thickness: f32, bounce: f32, surface: Vec2) -> bool {
        let gap = self.pos - closest;
        let dist = gap.len();
        let reach = BALL_RADIUS + thickness;
        if dist >= reach || dist == 0.0 {
            return false;
        }
        let n = gap * (1.0 / dist);
        self.pos = closest + n * reach;
        let relative = self.vel - surface;
        let into = relative.dot(n);
        if into < 0.0 {
            // A slow approach is resting contact: just stop going into the
            // surface, so the ball rolls along it freely.
            let restitution = if into < -RESTING_SPEED { bounce } else { 0.0 };
            let mut bounced = relative - n * (into * (1.0 + restitution));
            if into < -RESTING_SPEED {
                // A real impact loses a little speed along the surface too.
                let along = bounced - n * bounced.dot(n);
                bounced = bounced - along * IMPACT_GRIP;
            }
            self.vel = bounced + surface;
        }
        true
    }

    fn check_holes(&mut self, coils: u32) {
        let p = self.pos;
        for t in &self.table.trapdoors {
            let inside = p.x > t.min.x && p.x < t.max.x && p.y > t.min.y && p.y < t.max.y;
            if inside && coils & (1 << t.flap) != 0 {
                self.state = BallState::Underneath {
                    until: self.now.plus(UNDER_PLAYFIELD_MS),
                };
                let s = t.switch;
                self.close(s);
                return;
            }
        }
        let outhole = self.table.outhole;
        let lost = !(0.0..=600.0).contains(&p.x) || !(0.0..=1_100.0).contains(&p.y);
        if (p - outhole).len() < BALL_RADIUS + 6.0 || lost {
            self.state = BallState::InOuthole;
            self.pos = outhole;
            self.vel = Vec2::default();
        }
    }

    /// Holds `switch` closed for at least `ms` from now.
    fn close_for(&mut self, switch: SwitchId, ms: u32) {
        let until = self.now.plus(ms);
        let slot = &mut self.closed_until[switch.index()];
        if !slot.is_some_and(|held| held.reached(until)) {
            *slot = Some(until);
        }
    }

    fn close(&mut self, switch: SwitchId) {
        self.close_for(switch, MIN_CLOSED_MS);
    }

    fn switches(&mut self) -> SwitchStates {
        let mut states = SwitchStates::new();
        if self.state == BallState::Rolling {
            for i in 0..self.table.rollovers.len() {
                let r = &self.table.rollovers[i];
                if r.shape.distance(self.pos) < BALL_RADIUS {
                    let s = r.switch;
                    self.close_for(s, 1);
                }
            }
        }
        let now = self.now;
        for (i, slot) in self.closed_until.iter_mut().enumerate() {
            match *slot {
                Some(until) if !now.reached(until) => {
                    if let Some(id) = switch_from_index(i as u32) {
                        states.set(id, true);
                    }
                }
                _ => *slot = None,
            }
        }
        if self.state == BallState::InOuthole {
            states.set(sw::OUTHOLE, true);
        }
        states
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_map_has_every_kind_of_part() {
        let t = Table::from_map();
        let kicks = |circle: bool| {
            t.solids
                .iter()
                .filter(|s| s.kick.is_some() && matches!(s.shape, Shape::Circle { .. }) == circle)
                .count()
        };
        assert_eq!(kicks(true), 6, "pop bumpers");
        assert_eq!(kicks(false), 6, "kickers");
        let rubbers = t
            .solids
            .iter()
            .filter(|s| s.switch.is_some() && s.kick.is_none())
            .count();
        assert_eq!(rubbers, 2, "top rubbers");
        assert_eq!(t.trapdoors.len(), 3);
        assert_eq!(t.flippers.len(), 2);
        assert_eq!(t.gates.len(), 1);
        // Top lanes, side lanes and the shooter lane.
        assert_eq!(t.rollovers.len(), 7);
        // The outline, the shooter lane wall, the roller, four lane rails, the
        // centre post and six apron walls.
        let walls = t.solids.iter().filter(|s| s.switch.is_none()).count();
        assert_eq!(walls, 14);
    }

    #[test]
    fn switches_still_open_when_the_clock_wraps_round() {
        let mut physics = Physics {
            now: Instant::from_millis(u32::MAX - 3),
            ..Physics::default()
        };
        physics.close(sw::POP_BUMPERS[0]);
        assert!(physics.tick(0).is_closed(sw::POP_BUMPERS[0]));
        for _ in 0..MIN_CLOSED_MS {
            physics.tick(0);
        }
        assert!(!physics.tick(0).is_closed(sw::POP_BUMPERS[0]));
    }

    #[test]
    fn paths_with_curves_are_flattened() {
        let (p, closed) = path_points("M 0 0 Q 10 0 10 10 L 0 10 Z");
        assert!(closed);
        assert_eq!(p.len(), 10);
        assert_eq!(p[8], Vec2::new(10.0, 10.0));
    }

    #[test]
    fn flippers_point_inwards_and_turn_up() {
        let t = Table::from_map();
        let [left, right] = [t.flippers[0], t.flippers[1]];
        assert!(left.pivot.x < right.pivot.x);
        // At rest both tips are below their pivots, pointing to the centre.
        assert!(left.tip(0.0).y > left.pivot.y && left.tip(0.0).x > left.pivot.x);
        assert!(right.tip(0.0).y > right.pivot.y && right.tip(0.0).x < right.pivot.x);
        // Up, both tips are above their pivots.
        assert!(left.tip(FLIPPER_STROKE).y < left.pivot.y);
        assert!(right.tip(FLIPPER_STROKE).y < right.pivot.y);
    }
}
