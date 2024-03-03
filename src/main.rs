use notan::draw::*;
use notan::math::{vec2, Vec2, Vec3};
use notan::prelude::*;

const INITIAL_ENTITIES: usize = 2540;
const INITIAL_VELOCITY: f32 = 250.0;
const ENTITY_RADIUS: f32 = 4.0;
const GAME_WIDTH: f32 = 1280.0;
const GAME_HEIGHT: f32 = 940.0;
const COLLISION_COLOR_TIME: f32 = 0.1;
const ENTITY_COLOR: Color = Color::SILVER;
const ENTITY_COLLISION_COLOR: Color = Color::ORANGE;

#[derive(Copy, Clone, Debug)]
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
    collision_time: f32,
}

#[derive(AppState)]
struct State {
    entities: Vec<Entity>,
    texture: Texture,
    pause: bool,
}

#[notan_main]
fn main() -> Result<(), String> {
    let win = WindowConfig::default().set_size(GAME_WIDTH as _, GAME_HEIGHT as _);
    // .set_vsync(true);

    notan::init_with(setup)
        .add_config(win)
        .add_config(DrawConfig)
        .update(update)
        .draw(draw)
        .build()
}

fn setup(gfx: &mut Graphics) -> State {
    let entities = init_entities();
    let texture = gfx
        .create_texture()
        .from_image(include_bytes!("../assets/white_circle.png"))
        .build()
        .unwrap();
    State {
        entities,
        pause: false,
        texture,
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

    sys_clean_collisions(&mut state.entities, delta);
    sys_apply_velocity_to_body(&mut state.entities, delta);
    sys_bounce_rect(&mut state.entities);
    let collisions = sys_check_collision(&mut state.entities);
    sys_resolve_collisions(&mut state.entities, collisions);
    sys_body_to_transform(&mut state.entities);

    let fps = app.timer.fps();
    app.window().set_title(&format!("fps:{fps}"));
}

fn draw(gfx: &mut Graphics, state: &mut State) {
    let mut draw = gfx.create_draw();
    draw.clear(Color::BLACK);

    state.entities.iter().for_each(|e| {
        let pos = e.transform.position - e.transform.size * 0.5;
        let color = if e.collision_time > 0.0 {
            interpolate_color(
                ENTITY_COLOR,
                ENTITY_COLLISION_COLOR,
                COLLISION_COLOR_TIME,
                e.collision_time,
            )
        } else {
            ENTITY_COLOR
        };
        draw.image(&state.texture)
            .position(pos.x, pos.y)
            .size(e.transform.size.x, e.transform.size.y)
            .color(color);
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
                    radius: ENTITY_RADIUS,
                },
                transform: Transform {
                    position,
                    size: Vec2::splat(ENTITY_RADIUS * 2.0),
                },
                is_colliding: false,
                collision_time: 0.0,
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

fn interpolate_color(c1: Color, c2: Color, total_time: f32, elapsed: f32) -> Color {
    let c1: Vec3 = c1.rgb().into();
    let c2: Vec3 = c2.rgb().into();
    let delta = c2 - c1;
    let fc = c1 + delta * (elapsed / total_time);
    Color::from_rgb(fc.x, fc.y, fc.z)
}

// systems
fn sys_clean_collisions(entities: &mut [Entity], delta: f32) {
    entities.iter_mut().for_each(|e| {
        e.is_colliding = false;
        if e.collision_time > 0.0 {
            e.collision_time -= delta;
        }
    });
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
        entities[*id1].collision_time = COLLISION_COLOR_TIME;
        entities[*id2].is_colliding = true;
        entities[*id2].collision_time = COLLISION_COLOR_TIME;
    });

    colliding
}

fn sys_resolve_collisions(entities: &mut [Entity], collisions: Vec<Collision>) {
    collisions.into_iter().for_each(|Collision([id1, id2])| {
        let b1 = &entities[id1].body;
        let b2 = &entities[id2].body;

        let sum_radius = b1.radius + b2.radius;
        let pos_delta = b1.position - b2.position;
        let magnitude = pos_delta.length();
        let min_translation_distance = pos_delta * (sum_radius - magnitude) / magnitude;

        let vel_delta = b1.velocity - b2.velocity;
        let normalized_mtd = min_translation_distance.normalize();
        let relative_vel = vel_delta.dot(normalized_mtd);

        if relative_vel > 0.0 {
            return;
        }

        let normalized_rel_vel = normalized_mtd * relative_vel;

        entities[id1].body.velocity -= normalized_rel_vel;
        entities[id1].body.position += min_translation_distance;
        entities[id2].body.velocity += normalized_rel_vel;
        entities[id2].body.position -= min_translation_distance;
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
