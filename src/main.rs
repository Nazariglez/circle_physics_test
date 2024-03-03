use notan::draw::*;
use notan::math::{vec2, Vec2};
use notan::prelude::*;

const INITIAL_ENTITIES: usize = 30;
const INITIAL_VELOCITY: f32 = 50.0;
const GAME_WIDTH: f32 = 800.0;
const GAME_HEIGHT: f32 = 600.0;

#[derive(Copy, Clone, Debug, Hash)]
struct Collision([usize; 2]);
impl PartialEq for Collision {
    fn eq(&self, other: &Self) -> bool {
        let contain_id1 = self.0.contains(&other.0[0]);
        let contain_id2 = self.0.contains(&other.0[1]);
        contain_id1 && contain_id2
    }
}

struct Body {
    position: Vec2,
    velocity: Vec2,
    radius: f32,
}

struct Transform {
    position: Vec2,
    size: Vec2,
}

struct Entity {
    body: Body,
    transform: Transform,
    is_colliding: bool,
}

#[derive(AppState)]
struct State {
    entities: Vec<Entity>,
    pause: bool,
}

#[notan_main]
fn main() -> Result<(), String> {
    let win = WindowConfig::default()
        .set_size(GAME_WIDTH as _, GAME_HEIGHT as _)
        .set_vsync(true);

    notan::init_with(setup)
        .add_config(win)
        .add_config(DrawConfig)
        .update(update)
        .draw(draw)
        .build()
}

fn setup() -> State {
    let entities = init_entities();
    State {
        entities,
        pause: false,
    }
}

fn update(app: &mut App, state: &mut State) {
    if app.keyboard.was_pressed(KeyCode::Space) {
        state.pause = !state.pause;
    }

    if state.pause {
        return;
    }

    // -- logic
    let delta = app.timer.delta_f32();

    sys_clean_collisions(&mut state.entities);

    let collisions = sys_check_collision(&mut state.entities);
    sys_resolve_collisions(&mut state.entities, collisions);
    sys_bounce_rect(&mut state.entities);
    sys_apply_velocity_to_body(&mut state.entities, delta);
    sys_body_to_transform(&mut state.entities);
}

fn draw(gfx: &mut Graphics, state: &mut State) {
    let mut draw = gfx.create_draw();
    draw.clear(Color::BLACK);

    state.entities.iter().for_each(|e| {
        let pos = e.transform.position - e.transform.size * 0.5;
        draw.rect(pos.into(), e.transform.size.into())
            .color(Color::WHITE)
            .alpha(0.5)
            .stroke(2.0);

        let color = if e.is_colliding {
            Color::ORANGE
        } else {
            Color::AQUA
        };
        draw.circle(e.body.radius)
            .position(e.transform.position.x, e.transform.position.y)
            .color(color)
            .stroke(1.0);
    });

    gfx.render(&draw);
}

fn init_entities() -> Vec<Entity> {
    let mut rng = Random::default();
    (0..INITIAL_ENTITIES)
        .map(|_| {
            let min_pos = vec2(50.0, 50.0);
            let max_pos = vec2(GAME_WIDTH - min_pos.x * 2.0, GAME_HEIGHT - min_pos.y * 2.0);
            let position = vec2(
                min_pos.x + rng.gen::<f32>() * max_pos.x,
                min_pos.y + rng.gen::<f32>() * max_pos.y,
            );
            let min_vel = INITIAL_VELOCITY * -0.5;
            let max_vel = INITIAL_VELOCITY;
            let velocity = vec2(
                min_vel + rng.gen::<f32>() * max_vel,
                min_vel + rng.gen::<f32>() * max_vel,
            );
            Entity {
                body: Body {
                    position,
                    velocity,
                    radius: 16.0,
                },
                transform: Transform {
                    position,
                    size: vec2(32.0, 32.0),
                },
                is_colliding: false,
            }
        })
        .collect()
}

fn is_colliding(p1: Vec2, r1: f32, p2: Vec2, r2: f32) -> bool {
    let sum_radius = r1 + r2;
    let square_radius = sum_radius * sum_radius;
    let square_distance = p1.distance_squared(p2);
    square_distance <= square_radius
}

// systems
fn sys_clean_collisions(entities: &mut [Entity]) {
    entities.iter_mut().for_each(|e| e.is_colliding = false);
}

fn sys_check_collision(entities: &mut [Entity]) -> Vec<Collision> {
    // TODO do not nest loops, use spatial hashing
    let mut colliding = vec![]; // todo maybe use a hashset?
    entities.iter().enumerate().for_each(|(id1, e1)| {
        entities.iter().enumerate().for_each(|(id2, e2)| {
            if id1 == id2 {
                return;
            }

            if is_colliding(
                e1.body.position,
                e1.body.radius,
                e2.body.position,
                e2.body.radius,
            ) {
                let collision = Collision([id1, id2]);
                if !colliding.contains(&collision) {
                    colliding.push(collision);
                }
            }
        });
    });

    colliding.iter().for_each(|Collision([id1, id2])| {
        entities[*id1].is_colliding = true;
        entities[*id2].is_colliding = true;
    });

    colliding
}

fn sys_resolve_collisions(entities: &mut [Entity], collisions: Vec<Collision>) {
    collisions.into_iter().for_each(|Collision([id1, id2])| {
        let b1 = &entities[id1].body;
        let b2 = &entities[id2].body;
        let diff_pos = b2.position - b1.position;
        let distance = diff_pos.powf(2.0).length();
        let normalized = diff_pos / distance;
        let diff_vel = b1.velocity - b2.velocity;
        let speed = (diff_vel * normalized).length_squared();
        if speed < 0.0 || !speed.is_finite() {
            panic!("Speed: {}", speed);
        }

        entities[id1].body.velocity -= speed * normalized;
        entities[id2].body.velocity += speed * normalized;
    });
}

fn sys_bounce_rect(entities: &mut [Entity]) {
    entities.iter_mut().for_each(|e| {
        let left = e.body.position.x - e.body.radius <= 0.0;
        if left {
            e.body.velocity.x *= -1.0;
            e.body.position.x = e.body.radius;
        }
        let right = e.body.position.x + e.body.radius >= GAME_WIDTH;
        if right {
            e.body.velocity.x *= -1.0;
            e.body.position.x = GAME_WIDTH - e.body.radius;
        }
        let top = e.body.position.y - e.body.radius < 0.0;
        if top {
            e.body.velocity.y *= -1.0;
            e.body.position.y = e.body.radius;
        }
        let bottom = e.body.position.y + e.body.radius >= GAME_HEIGHT;
        if bottom {
            e.body.velocity.y *= -1.0;
            e.body.position.y = GAME_HEIGHT - e.body.radius;
        }
    });
}

fn sys_apply_velocity_to_body(entities: &mut [Entity], delta: f32) {
    entities.iter_mut().for_each(|e| {
        e.body.position += e.body.velocity * delta;
    });
}

fn sys_body_to_transform(entites: &mut [Entity]) {
    entites.iter_mut().for_each(|e| {
        e.transform.position = e.body.position;
    });
}
